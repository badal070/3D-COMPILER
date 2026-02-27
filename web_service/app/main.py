"""edu3d modeling service.

Provides:
- .dsl model save/load/list APIs
- deterministic description -> DSL translation
- LLM-assisted model description generation/refinement
- Rust DSL compile endpoint with event-driven WebSocket IR broadcasts
"""

from __future__ import annotations

import asyncio
import json
import os
import re
import shutil
import subprocess
import tempfile
from dataclasses import dataclass
from pathlib import Path
from typing import Any

from fastapi import FastAPI, HTTPException, WebSocket, WebSocketDisconnect
from fastapi.responses import FileResponse, JSONResponse, PlainTextResponse, Response
from fastapi.staticfiles import StaticFiles
from pydantic import BaseModel, Field

try:
    import aiofiles
except Exception:  # pragma: no cover - fallback for minimal environments
    aiofiles = None

from .description_to_dsl import (
    ValidationIssue,
    description_to_dsl,
    map_compiler_errors_to_description,
    validate_description,
)
from .llm_bridge import explain_description, generate_description, refine_description
from .precision import compute_measurements

APP_DIR = Path(__file__).resolve().parent
PROJECT_ROOT = APP_DIR.parent.parent
STATIC_DIR = APP_DIR / "static"
MODELS_DIR = PROJECT_ROOT / "models"

MODELS_DIR.mkdir(parents=True, exist_ok=True)
(MODELS_DIR / "examples").mkdir(parents=True, exist_ok=True)

app = FastAPI(title="edu3d Modeling Service", version="2.0.0")
app.mount("/static", StaticFiles(directory=str(STATIC_DIR)), name="static")

_current_ir: dict[str, Any] = {}
_ws_clients: set[WebSocket] = set()
_ws_lock = asyncio.Lock()


class ModelSaveRequest(BaseModel):
    name: str = Field(min_length=1)
    dsl_source: str


class CompileRequest(BaseModel):
    dsl_source: str


class DescribeRequest(BaseModel):
    prompt: str = Field(min_length=1)
    current_description: dict[str, Any] | None = None
    unit_system: str = "SI"
    precision: float = 0.01


class RefineRequest(BaseModel):
    prompt: str = Field(min_length=1)
    current_description: dict[str, Any]
    unit_system: str = "SI"
    precision: float = 0.01


class DescriptionRequest(BaseModel):
    description: dict[str, Any]


@dataclass
class CompileResult:
    ok: bool
    ir: dict[str, Any] | None
    errors: list[dict[str, Any]]


@app.get("/")
async def index() -> FileResponse:
    return FileResponse(STATIC_DIR / "index.html")


@app.get("/favicon.ico")
async def favicon() -> Response:
    return Response(status_code=204)


@app.post("/api/model/save")
async def save_model(body: ModelSaveRequest) -> JSONResponse:
    filename = sanitize_model_filename(body.name)
    path = MODELS_DIR / filename

    await write_text(path, body.dsl_source)
    return JSONResponse(
        {
            "ok": True,
            "name": filename,
            "path": str(path.relative_to(PROJECT_ROOT)),
        }
    )


@app.get("/api/model/load/{name}")
async def load_model(name: str) -> PlainTextResponse:
    filename = sanitize_model_filename(name)
    path = MODELS_DIR / filename
    if not path.exists():
        raise HTTPException(status_code=404, detail=f"Model '{filename}' not found")

    source = await read_text(path)
    return PlainTextResponse(source)


@app.get("/api/model/list")
async def list_models() -> JSONResponse:
    models = sorted(path.name for path in MODELS_DIR.glob("*.dsl"))
    return JSONResponse({"models": models})


@app.post("/api/model/compile")
async def compile_model(body: CompileRequest) -> JSONResponse:
    result = await run_dsl_compiler(body.dsl_source)
    if result.ok and result.ir is not None:
        global _current_ir
        _current_ir = result.ir
        measurements = compute_measurements(result.ir)
        await broadcast_ir(_current_ir)
        return JSONResponse(
            {
                "ok": True,
                "ir": result.ir,
                "errors": [],
                "measurements": measurements,
            }
        )

    return JSONResponse({"ok": False, "ir": None, "errors": result.errors, "measurements": None})


@app.post("/api/llm/describe")
async def llm_describe(body: DescribeRequest) -> JSONResponse:
    description = await generate_description(
        user_prompt=body.prompt,
        current_description=body.current_description,
        unit_system=body.unit_system,
        precision=body.precision,
    )
    return JSONResponse({"description": description})


@app.post("/api/llm/refine")
async def llm_refine(body: RefineRequest) -> JSONResponse:
    description, changed_fields = await refine_description(
        user_prompt=body.prompt,
        current_description=body.current_description,
        unit_system=body.unit_system,
        precision=body.precision,
    )
    return JSONResponse({"description": description, "changed_fields": changed_fields})


@app.post("/api/description/to_dsl")
async def description_to_dsl_endpoint(body: DescriptionRequest) -> JSONResponse:
    issues = validate_description(body.description)
    if issues:
        return JSONResponse(
            {
                "dsl": "",
                "errors": [serialize_issue(issue) for issue in issues],
            }
        )

    translation = description_to_dsl(body.description)
    compile_result = await run_dsl_compiler(translation.dsl)

    mapped_errors = map_compiler_errors_to_description(
        compile_result.errors,
        translation.source_map,
    )

    response: dict[str, Any] = {
        "dsl": translation.dsl,
        "errors": mapped_errors,
    }

    if compile_result.ok and compile_result.ir is not None:
        global _current_ir
        _current_ir = compile_result.ir
        response["ir"] = compile_result.ir
        response["measurements"] = compute_measurements(compile_result.ir)
        await broadcast_ir(_current_ir)

    return JSONResponse(response)


@app.post("/api/llm/explain")
async def llm_explain(body: DescriptionRequest) -> JSONResponse:
    explanation = await explain_description(body.description)
    return JSONResponse({"explanation": explanation})


@app.websocket("/ws")
async def ws_ir_stream(websocket: WebSocket) -> None:
    await websocket.accept()

    async with _ws_lock:
        _ws_clients.add(websocket)

    if _current_ir:
        try:
            await websocket.send_text(json.dumps(_current_ir))
        except Exception:
            pass

    try:
        while True:
            # Keep socket alive; client may send optional pings/messages.
            await websocket.receive_text()
    except WebSocketDisconnect:
        pass
    finally:
        async with _ws_lock:
            _ws_clients.discard(websocket)


async def broadcast_ir(ir_payload: dict[str, Any]) -> None:
    payload = json.dumps(ir_payload)
    dead_clients: list[WebSocket] = []

    async with _ws_lock:
        for client in _ws_clients:
            try:
                await client.send_text(payload)
            except Exception:
                dead_clients.append(client)

        for client in dead_clients:
            _ws_clients.discard(client)


async def run_dsl_compiler(dsl_source: str) -> CompileResult:
    if not dsl_source.strip():
        return CompileResult(
            ok=False,
            ir=None,
            errors=[{"code": "E000", "message": "DSL source is empty", "line": None, "column": None}],
        )

    compiler_cmd = resolve_compiler_command()

    with tempfile.NamedTemporaryFile(mode="w", suffix=".dsl", delete=False) as handle:
        handle.write(dsl_source)
        temp_path = Path(handle.name)

    try:
        cmd = [*compiler_cmd, str(temp_path), "--json"]
        process = await asyncio.to_thread(
            subprocess.run,
            cmd,
            capture_output=True,
            text=True,
            cwd=str(PROJECT_ROOT),
            timeout=120,
        )
    except FileNotFoundError:
        temp_path.unlink(missing_ok=True)
        return CompileResult(
            ok=False,
            ir=None,
            errors=[
                {
                    "code": "E000",
                    "message": "dsl-compiler is not available. Build the Rust workspace first.",
                    "line": None,
                    "column": None,
                }
            ],
        )
    except subprocess.TimeoutExpired:
        temp_path.unlink(missing_ok=True)
        return CompileResult(
            ok=False,
            ir=None,
            errors=[
                {
                    "code": "E000",
                    "message": "dsl-compiler timed out",
                    "line": None,
                    "column": None,
                }
            ],
        )
    finally:
        temp_path.unlink(missing_ok=True)

    if process.returncode == 0:
        try:
            ir = json.loads(process.stdout.strip())
        except json.JSONDecodeError as exc:
            return CompileResult(
                ok=False,
                ir=None,
                errors=[
                    {
                        "code": "E000",
                        "message": f"Failed to parse compiler JSON output: {exc}",
                        "line": None,
                        "column": None,
                    }
                ],
            )

        return CompileResult(ok=True, ir=ir, errors=[])

    errors = parse_compiler_errors(process.stderr or process.stdout)
    if not errors:
        errors = [
            {
                "code": "E000",
                "message": (process.stderr or process.stdout or "Compilation failed").strip(),
                "line": None,
                "column": None,
            }
        ]

    return CompileResult(ok=False, ir=None, errors=errors)


def resolve_compiler_command() -> list[str]:
    candidates = [
        PROJECT_ROOT / "target" / "release" / "dsl-compiler",
        PROJECT_ROOT / "target" / "debug" / "dsl-compiler",
        PROJECT_ROOT / "dsl" / "target" / "release" / "dsl-compiler",
        PROJECT_ROOT / "dsl" / "target" / "debug" / "dsl-compiler",
    ]

    for candidate in candidates:
        if candidate.exists() and os.access(candidate, os.X_OK):
            return [str(candidate)]

    cargo = shutil.which("cargo")
    if cargo:
        return [
            cargo,
            "run",
            "--manifest-path",
            str(PROJECT_ROOT / "dsl" / "Cargo.toml"),
            "--bin",
            "dsl-compiler",
            "--",
        ]

    return ["dsl-compiler"]


def parse_compiler_errors(stderr: str) -> list[dict[str, Any]]:
    errors: list[dict[str, Any]] = []
    pattern = re.compile(
        r"E(?P<code>\d{3,4}):\s*(?P<message>.*?)(?:\((?P<file>.*?):(?P<line>\d+):(?P<column>\d+)\))?$"
    )

    for line in stderr.splitlines():
        line = line.strip()
        if not line:
            continue
        if line.startswith("-"):
            line = line[1:].strip()
        if line.startswith("✗"):
            continue

        match = pattern.search(line)
        if match:
            errors.append(
                {
                    "code": f"E{match.group('code')}",
                    "message": match.group("message").strip(),
                    "line": int(match.group("line")) if match.group("line") else None,
                    "column": int(match.group("column")) if match.group("column") else None,
                }
            )

    return errors


def sanitize_model_filename(name: str) -> str:
    clean = re.sub(r"[^A-Za-z0-9_.-]+", "_", name.strip())
    clean = clean.strip("._") or "model"
    if not clean.endswith(".dsl"):
        clean = f"{clean}.dsl"
    return clean


async def write_text(path: Path, text: str) -> None:
    if aiofiles is not None:
        async with aiofiles.open(path, "w", encoding="utf-8") as handle:
            await handle.write(text)
    else:
        await asyncio.to_thread(path.write_text, text, "utf-8")


async def read_text(path: Path) -> str:
    if aiofiles is not None:
        async with aiofiles.open(path, "r", encoding="utf-8") as handle:
            return await handle.read()
    return await asyncio.to_thread(path.read_text, "utf-8")


def serialize_issue(issue: ValidationIssue) -> dict[str, str]:
    return {"code": "DESC_VALIDATION", "path": issue.path, "message": issue.message}
