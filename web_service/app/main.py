"""FastAPI adapter for edu3d runtime — serves a simple Three.js page and
provides a mock snapshot API and WebSocket updates.

Endpoints:
- GET / -> serves static index.html
- GET /api/snapshot -> returns current snapshot JSON (mock or later integrate with runtime)
- WebSocket /ws -> pushes periodic snapshot updates to connected clients

This file avoids importing heavy dependencies at top-level so `py_compile` can
run without installing the packages. The app imports at runtime.
"""

import asyncio
import json
import time
import tempfile
import subprocess
import os
from pathlib import Path
import re
import shutil

from fastapi import FastAPI, WebSocket
from fastapi.staticfiles import StaticFiles
from fastapi.responses import FileResponse, JSONResponse

APP_DIR = Path(__file__).resolve().parent
STATIC_DIR = APP_DIR / "static"

app = FastAPI()

# Mount static files
app.mount("/static", StaticFiles(directory=str(STATIC_DIR)), name="static")

# Simple in-memory snapshot store (mock). Replace integration point later.
_snapshot = {
    "tick": 0,
    "timestamp": time.time(),
    "objects": [
        {"id": 1, "position": [0.0, 0.0, 0.0], "highlighted": False}
    ],
}

# Server-side movement targets and configuration
_snapshot_targets = {}  # id(str) -> [x,y,z]
_movement_config = {"speed": 0.1, "quantize": True}
_entity_configs = {}  # id -> {speed: number, quantize: bool}
# Active compound motions scheduled from IR: list of dicts with
# {motion_id, entity_id, start_time, end_time, start_pos, end_pos}
_active_motions = []
_last_compile_error = None


def _parse_simple_dsl(dsl_text: str):
    """Very small DSL parser for demo purposes.

    Recognizes patterns like:
      - "position x y z"
      - "at x,y,z"
      - numeric triple anywhere in the text

    Returns a dict with `position` key (list of 3 floats).
    This is deliberately small and intended as a placeholder for
    a richer parser or RPC to the Rust DSL toolchain.
    """
    if not dsl_text:
        return {}

    # Try to find "position" followed by numbers
    m = re.search(r"position\s*([-+]?[0-9]*\.?[0-9]+)\s*,?\s*([-+]?[0-9]*\.?[0-9]+)\s*,?\s*([-+]?[0-9]*\.?[0-9]+)", dsl_text, re.I)
    if not m:
        # try "at x y z" or any three numbers
        m = re.search(r"at\s+([-+]?[0-9]*\.?[0-9]+)\s*,?\s*([-+]?[0-9]*\.?[0-9]+)\s*,?\s*([-+]?[0-9]*\.?[0-9]+)", dsl_text, re.I)

    if not m:
        # fallback: first triple of numbers found anywhere
        m = re.search(r"([-+]?[0-9]*\.?[0-9]+)\s*,?\s*([-+]?[0-9]*\.?[0-9]+)\s*,?\s*([-+]?[0-9]*\.?[0-9]+)", dsl_text)

    if m:
        try:
            x = float(m.group(1))
            y = float(m.group(2))
            z = float(m.group(3))
            return {"position": [x, y, z]}
        except Exception:
            return {}

    return {}


def _compile_with_rust(dsl_text: str):
    """Write DSL to a temp file and invoke the Rust `dsl-compiler --json`.

    Returns parsed JSON (dict) on success, or None on failure.
    """
    global _last_compile_error
    _last_compile_error = None
    if not dsl_text:
        return None

    # Determine repo root and DSL crate path
    repo_root = APP_DIR.parent.parent
    dsl_dir = repo_root / "dsl"

    # Create a temp file for the DSL source
    tf = None
    try:
        # Strip common comment styles that the Rust compiler rejects (lines starting with # or //)
        cleaned_lines = []
        for ln in (dsl_text or "").splitlines():
            if re.match(r"^\s*#", ln):
                continue
            if re.match(r"^\s*//", ln):
                continue
            cleaned_lines.append(ln)
        cleaned_text = "\n".join(cleaned_lines)

        tf = tempfile.NamedTemporaryFile(mode="w", suffix=".dsl", delete=False)
        tf.write(cleaned_text)
        tf.flush()
        tf.close()

        # Prefer a prebuilt binary in target/release or target/debug to avoid rebuilding
        release_bin = dsl_dir / "target" / "release" / "dsl-compiler"
        debug_bin = dsl_dir / "target" / "debug" / "dsl-compiler"
        # Also check workspace-level target (monorepo layouts) if not present under crate
        repo_release = APP_DIR.parent.parent / "target" / "release" / "dsl-compiler"
        repo_debug = APP_DIR.parent.parent / "target" / "debug" / "dsl-compiler"
        if not release_bin.exists() and repo_release.exists():
            release_bin = repo_release
        if not debug_bin.exists() and repo_debug.exists():
            debug_bin = repo_debug
        if release_bin.exists() or debug_bin.exists():
            bin_path = str(release_bin if release_bin.exists() else debug_bin)
            cmd = [bin_path, tf.name, "--json"]
            proc = subprocess.run(cmd, cwd=str(dsl_dir), capture_output=True, text=True, timeout=30)
        else:
            # If cargo is available, try to build the release binary first to create a stable executable
            cargo_path = shutil.which('cargo')
            if cargo_path:
                try:
                    build_cmd = [cargo_path, 'build', '--release', '--manifest-path', str(dsl_dir / 'Cargo.toml')]
                    build_proc = subprocess.run(build_cmd, cwd=str(dsl_dir), capture_output=True, text=True, timeout=300)
                    if build_proc.returncode == 0 and release_bin.exists():
                        bin_path = str(release_bin)
                        cmd = [bin_path, tf.name, "--json"]
                        proc = subprocess.run(cmd, cwd=str(dsl_dir), capture_output=True, text=True, timeout=30)
                    else:
                        # Fallback to cargo run (may build in debug)
                        cmd = [
                            cargo_path,
                            "run",
                            "--manifest-path",
                            str(dsl_dir / "Cargo.toml"),
                            "--bin",
                            "dsl-compiler",
                            "--",
                            tf.name,
                            "--json",
                        ]
                        proc = subprocess.run(cmd, cwd=str(dsl_dir), capture_output=True, text=True, timeout=120)
                except subprocess.TimeoutExpired as e:
                    _last_compile_error = f"cargo build timed out: {e}"
                    print(_last_compile_error)
                    return None
            else:
                _last_compile_error = "cargo not found in PATH; install Rust toolchain to use real compiler"
                print(_last_compile_error)
                return None
        if proc.returncode != 0:
            # Compilation/execution failed — capture stderr and return None
            _last_compile_error = proc.stderr or proc.stdout or 'unknown compiler error'
            print("dsl-compiler error:", _last_compile_error)
            return None

        stdout = proc.stdout.strip()
        if not stdout:
            return None

        # The compiler prints the JSON to stdout when --json provided
        try:
            parsed = json.loads(stdout)
        except Exception as e:
            _last_compile_error = f"failed to parse compiler JSON output: {e}\nstdout:\n{stdout}\nstderr:\n{proc.stderr}"
            print(_last_compile_error)
            return None
        return parsed
    except Exception as e:
        _last_compile_error = str(e)
        print("Error running rust compiler:", e)
        return None
    finally:
        try:
            if tf:
                os.unlink(tf.name)
        except Exception:
            pass


def _schedule_motions_from_ir(ir: dict):
    """Parse simple motion definitions from IR and schedule them into
    `_active_motions`. Supports `motions` and optional `timelines`/`compound_motions`.

    Expected IR shapes (flexible):
      ir['motions'] = [ { 'id': 'm1', 'entity': 'e1', 'params': { 'to':[x,y,z] }, 'duration': 2.0 }, ... ]
      ir['timelines'] = [ { 'events': [ { 'motion': 'm1', 'start':0.0, 'duration':2.0 }, ... ] }, ... ]
    """
    global _active_motions, _snapshot
    now = time.time()
    motions = {}
    for m in ir.get('motions', []):
        mid = m.get('id') or m.get('name')
        if not mid:
            continue
        motions[mid] = m
    # include trajectories as motions with path parameters
    for t in ir.get('trajectories', []):
        tid = t.get('id') or t.get('name')
        if not tid:
            continue
        motions[tid] = {
            'id': tid,
            'entity': t.get('target'),
            'path_type': t.get('path_type'),
            'params': t
        }

    def schedule_motion_entry(m, start_offset=0.0, override_duration=None, easing=None):
        entity_id = m.get('entity') or m.get('target') or m.get('entity_id')
        if entity_id is None:
            return
        params = m.get('params', m.get('parameters', {})) or {}
        # capture easing and normal/axis if present
        easing_value = easing or params.get('easing') or m.get('easing') or None
        normal_vec = params.get('normal') or params.get('axis') or None
        duration = float(override_duration if override_duration is not None else m.get('duration') or params.get('duration') or 2.0)
        # determine start position from current snapshot
        start_pos = None
        for obj in _snapshot.get('objects', []):
            if str(obj.get('id')) == str(entity_id):
                start_pos = list(obj.get('position', [0.0, 0.0, 0.0]))
                break
        if start_pos is None:
            start_pos = [0.0, 0.0, 0.0]

        # if this is a path-based motion, store path params instead of computing end_pos
        if m.get('path_type') or params.get('path_type'):
            # keep trajectory/path parameters on the scheduled entry
            entry = {
                'motion_id': m.get('id'),
                'entity_id': str(entity_id),
                'start_time': now + float(start_offset),
                'end_time': now + float(start_offset) + float(duration),
                'start_pos': start_pos,
                'path_type': m.get('path_type') or params.get('path_type'),
                'path_params': params if params else m.get('params', {}),
                'easing': easing_value,
                'normal': normal_vec,
            }
            _active_motions.append(entry)
            return

        if 'to' in params:
            end_pos = list(params['to'])
        elif 'offset' in params:
            off = params['offset']
            end_pos = [start_pos[i] + float(off[i]) for i in range(3)]
        else:
            axis = params.get('axis')
            dist = params.get('distance') or params.get('dist') or 0
            try:
                dist = float(dist)
            except Exception:
                dist = 0
            if axis and dist:
                if axis == 'x':
                    end_pos = [start_pos[0] + dist, start_pos[1], start_pos[2]]
                elif axis == 'y':
                    end_pos = [start_pos[0], start_pos[1] + dist, start_pos[2]]
                else:
                    end_pos = [start_pos[0], start_pos[1], start_pos[2] + dist]
            else:
                return

        entry = {
            'motion_id': m.get('id'),
            'entity_id': str(entity_id),
            'start_time': now + float(start_offset),
            'end_time': now + float(start_offset) + float(duration),
            'start_pos': start_pos,
            'end_pos': end_pos,
        }
        _active_motions.append(entry)

    # Build quick lookup for compound motions
    compound_map = {cm.get('id'): cm for cm in ir.get('compound_motions', [])}

    def expand_compound(name, seen=None):
        # returns list of motion dicts (from motions) for a compound_motion name
        if seen is None:
            seen = set()
        if name in seen:
            return []
        seen.add(name)
        cm = compound_map.get(name)
        if not cm:
            return []
        out = []
        for ref in cm.get('motions', []):
            ref = ref.strip()
            # if references another compound_motion
            if ref in compound_map:
                out.extend(expand_compound(ref, seen))
            else:
                md = motions.get(ref)
                if md:
                    out.append(md)
        return out

    timelines = ir.get('timelines') or []
    if timelines:
        for tl in timelines:
            events = tl.get('events', [])
            for ev in events:
                ref = ev.get('ref')
                start_offset = ev.get('start', 0.0)
                duration = ev.get('duration') or ev.get('length') or None
                if not ref:
                    continue
                rtype, rid = ref if isinstance(ref, tuple) else (None, None)
                if rtype == 'motion' or rtype == 'trajectory':
                    mid = rid
                    if mid and mid in motions:
                        m = motions[mid]
                        schedule_motion_entry(m, start_offset=start_offset, override_duration=duration)
                elif rtype == 'compound_motion':
                    # expand and schedule all inner motions in compound (parallel semantics)
                    inner = expand_compound(rid)
                    for m in inner:
                        schedule_motion_entry(m, start_offset=start_offset, override_duration=duration)
                else:
                    # if the event uses direct motion id string
                    mid = ev.get('motion') or ev.get('id')
                    if mid and mid in motions:
                        m = motions[mid]
                        schedule_motion_entry(m, start_offset=start_offset, override_duration=duration)
    else:
        for mid, m in motions.items():
            schedule_motion_entry(m, start_offset=0.0)


def _nl_to_dsl(nl_text: str):
    """Simple NL -> DSL translator using heuristic rules.

    Converts prompts like "move cube to 1 0 2" or
    "place cube at 1,0,2" into a minimal DSL scene string.
    This is intentionally lightweight; for more advanced
    translation an LLM or rule engine can be integrated.
    """
    if not nl_text:
        return ""

    # Find a triple of numbers in the NL text
    m = re.search(r"([-+]?[0-9]*\.?[0-9]+)\s*,?\s*([-+]?[0-9]*\.?[0-9]+)\s*,?\s*([-+]?[0-9]*\.?[0-9]+)", nl_text)
    if m:
        x, y, z = m.group(1), m.group(2), m.group(3)
        dsl = (
            'scene { name: "NL" version:1 ir_version:"0.1.0" unit_system:"SI" }\n'
            f'entity cube1 {{ kind: solid components {{ transform {{ position: [{x}, {y}, {z}] }} }} }}'
        )
        return dsl

    # Try to parse phrases like 'move to X Y Z' with spaces
    m2 = re.search(r"(move|place|put).*?(?:to|at)\s+([-+]?[0-9]*\.?[0-9]+)\s+([-+]?[0-9]*\.?[0-9]+)\s+([-+]?[0-9]*\.?[0-9]+)", nl_text, re.I)
    if m2:
        x, y, z = m2.group(2), m2.group(3), m2.group(4)
        dsl = (
            'scene { name: "NL" version:1 ir_version:"0.1.0" unit_system:"SI" }\n'
            f'entity cube1 {{ kind: solid components {{ transform {{ position: [{x}, {y}, {z}] }} }} }}'
        )
        return dsl

    # Fallback: return an empty string so caller can handle
    return ""


def _extract_block(text: str, start_idx: int) -> (str, int):
    """Given text and index of an opening '{', return the block contents and index after closing '}'."""
    depth = 0
    i = start_idx
    n = len(text)
    buf = []
    while i < n:
        ch = text[i]
        if ch == '{':
            depth += 1
        elif ch == '}':
            depth -= 1
            if depth == 0:
                return (''.join(buf), i + 1)
        buf.append(ch)
        i += 1
    return ('', start_idx)


def _parse_dsl_to_ir(dsl_text: str) -> dict:
    """Lightweight DSL -> IR converter for subset used in samples (entities, motions, trajectories, timelines, compound_motions).

    This parser is tolerant and uses simple brace matching and regex to extract ids and numeric vectors.
    """
    if not dsl_text:
        return {}
    text = dsl_text
    ir = {'entities': [], 'motions': [], 'trajectories': [], 'compound_motions': [], 'timelines': []}

    # find entity/ motion/trajectory/compound_motion/ timeline occurrences
    for kind in ['entity', 'motion', 'trajectory', 'compound_motion', 'timeline']:
        idx = 0
        while True:
            m = re.search(rf"\b{kind}\s+([A-Za-z0-9_\-]+)\s*\{{", text[idx:])
            if not m:
                break
            name = m.group(1)
            start = idx + m.end() - 1
            body, endpos = _extract_block(text, start)
            idx = idx + m.end() + (endpos - start)

            # parse body for key fields using regex
            def extract_vec(body, key):
                mm = re.search(rf"{key}\s*:\s*\[([^\]]+)\]", body)
                if mm:
                    parts = [p.strip() for p in mm.group(1).split(',')]
                    try:
                        return [float(parts[0]), float(parts[1]), float(parts[2])]
                    except Exception:
                        return None
                return None

            def extract_num(body, key):
                mm = re.search(rf"{key}\s*:\s*([-+]?[0-9]*\.?[0-9]+)", body)
                if mm:
                    try:
                        return float(mm.group(1))
                    except Exception:
                        return None
                return None

            if kind == 'entity':
                pos = extract_vec(body, 'position')
                prim_m = re.search(r"primitive\s*:\s*([A-Za-z0-9_\-]+)", body)
                prim = prim_m.group(1) if prim_m else None
                ent = {'id': name, 'components': {}}
                if pos is not None:
                    ent['components']['transform'] = {'properties': {'position': pos}}
                if prim:
                    ent['components']['geometry'] = {'properties': {'primitive': prim}}
                ir['entities'].append(ent)

            elif kind == 'motion':
                target_m = re.search(r"target\s*:\s*([A-Za-z0-9_\-]+)", body)
                type_m = re.search(r"type\s*:\s*([A-Za-z0-9_\-]+)", body)
                params = {}
                # common params
                params['center'] = extract_vec(body, 'center') or extract_vec(body, 'centre')
                params['axis'] = extract_vec(body, 'axis')
                params['normal'] = extract_vec(body, 'normal')
                params['to'] = None
                to_m = re.search(r"to\s*:\s*\[([^\]]+)\]", body)
                if to_m:
                    parts = [p.strip() for p in to_m.group(1).split(',')]
                    try:
                        params['to'] = [float(parts[0]), float(parts[1]), float(parts[2])]
                    except Exception:
                        params['to'] = None
                params['radius'] = extract_num(body, 'radius')
                params['pitch'] = extract_num(body, 'pitch')
                params['turns'] = extract_num(body, 'turns')
                params['start_angle'] = extract_num(body, 'start_angle') or 0.0
                # optional easing
                easing_m = re.search(r"easing\s*:\s*([A-Za-z0-9_\-]+)", body)
                if easing_m:
                    params['easing'] = easing_m.group(1)
                params['speed'] = extract_num(body, 'speed')
                tgt = target_m.group(1) if target_m else None
                typ = type_m.group(1) if type_m else None
                ir['motions'].append({'id': name, 'entity': tgt, 'type': typ, 'params': params})

            elif kind == 'trajectory':
                target_m = re.search(r"target\s*:\s*([A-Za-z0-9_\-]+)", body)
                path_m = re.search(r"path_type\s*:\s*([A-Za-z0-9_\-]+)", body)
                params = {}
                params['center'] = extract_vec(body, 'center')
                params['axis'] = extract_vec(body, 'axis')
                params['normal'] = extract_vec(body, 'normal')
                params['radius'] = extract_num(body, 'radius')
                params['pitch'] = extract_num(body, 'pitch')
                params['turns'] = extract_num(body, 'turns')
                params['start_angle'] = extract_num(body, 'start_angle') or 0.0
                easing_m = re.search(r"easing\s*:\s*([A-Za-z0-9_\-]+)", body)
                if easing_m:
                    params['easing'] = easing_m.group(1)
                params['speed'] = extract_num(body, 'speed')
                tgt = target_m.group(1) if target_m else None
                ptype = path_m.group(1) if path_m else None
                ir['trajectories'].append({'id': name, 'target': tgt, 'path_type': ptype, **params})

            elif kind == 'compound_motion':
                type_m = re.search(r"type\s*:\s*([A-Za-z0-9_\-]+)", body)
                motions_m = re.search(r"motions\s*:\s*\[([^\]]+)\]", body)
                motions_list = []
                if motions_m:
                    motions_list = [p.strip() for p in motions_m.group(1).split(',')]
                ir['compound_motions'].append({'id': name, 'type': type_m.group(1) if type_m else None, 'motions': motions_list})

            elif kind == 'timeline':
                # find events inside the timeline body
                events = []
                ev_idx = 0
                while True:
                    ev_m = re.search(r"event\s*\{", body[ev_idx:])
                    if not ev_m:
                        break
                    ev_start = ev_idx + ev_m.end() - 1
                    ev_body, ev_endpos = _extract_block(body, ev_start)
                    ev_idx = ev_idx + ev_m.end() + (ev_endpos - ev_start)
                    # extract fields
                    ref = None
                    for key in ['motion', 'compound_motion', 'trajectory']:
                        mm = re.search(rf"{key}\s*:\s*([A-Za-z0-9_\-]+)", ev_body)
                        if mm:
                            ref = (key, mm.group(1))
                            break
                    start = extract_num(ev_body, 'start') or 0.0
                    duration = extract_num(ev_body, 'duration') or extract_num(ev_body, 'length')
                    events.append({'ref': ref, 'start': start, 'duration': duration})
                ir['timelines'].append({'id': name, 'events': events})

    return ir


@app.post("/api/translate")
async def translate_nl(payload: dict):
    """Translate a natural language prompt to DSL.

    Expected JSON: { "nl": "<prompt>" }
    Returns: { "dsl": "<dsl_text>" }
    """
    nl = payload.get("nl") if isinstance(payload, dict) else None
    if not nl:
        return JSONResponse(content={"dsl": ""})

    dsl = _nl_to_dsl(nl)
    return JSONResponse(content={"dsl": dsl})


@app.post("/api/config")
async def set_config(payload: dict):
    """Set server-side movement configuration.

    Expected JSON: { "speed": number, "quantize": bool }
    """
    global _movement_config
    if not isinstance(payload, dict):
        return JSONResponse(content={"ok": False, "error": "invalid payload"}, status_code=400)
    s = payload.get("speed")
    q = payload.get("quantize")
    try:
        if s is not None:
            s = float(s)
            if s < 0:
                raise ValueError()
            _movement_config["speed"] = s
        if q is not None:
            _movement_config["quantize"] = bool(q)
    except Exception:
        return JSONResponse(content={"ok": False, "error": "invalid values"}, status_code=400)
    return JSONResponse(content={"ok": True, "config": _movement_config})


@app.get("/api/config")
async def get_config():
    """Return global movement config and per-entity configs."""
    return JSONResponse(content={"global": _movement_config, "entities": _entity_configs})


@app.post("/api/config/entity")
async def set_entity_config(payload: dict):
    """Set per-entity movement config.

    Expected JSON: { "id": "entity_id", "speed": number, "quantize": bool }
    """
    if not isinstance(payload, dict):
        return JSONResponse(content={"ok": False, "error": "invalid payload"}, status_code=400)
    eid = payload.get("id")
    if not eid:
        return JSONResponse(content={"ok": False, "error": "missing id"}, status_code=400)
    try:
        entry = _entity_configs.get(str(eid), {}).copy()
        if "speed" in payload:
            entry["speed"] = float(payload["speed"])
        if "quantize" in payload:
            entry["quantize"] = bool(payload["quantize"])
        _entity_configs[str(eid)] = entry
        return JSONResponse(content={"ok": True, "entity": {str(eid): entry}})
    except Exception as e:
        return JSONResponse(content={"ok": False, "error": "invalid values"}, status_code=400)


@app.get("/")
async def index():
    return FileResponse(STATIC_DIR / "index.html")


@app.get("/api/snapshot")
async def get_snapshot():
    return JSONResponse(content=_snapshot)


@app.post("/api/render")
async def render_dsl(payload: dict):
    """Accept a small DSL string and update the in-memory snapshot.

    Expected JSON: { "dsl": "..." }

    This endpoint returns the updated snapshot so the frontend can
    immediately update without waiting for the WebSocket tick.
    """
    dsl_text = payload.get("dsl") if isinstance(payload, dict) else None

    # First, try to use the Rust DSL pipeline (preferred)
    ir_json = None
    ir_source = None
    try:
        ir_json = _compile_with_rust(dsl_text or "")
        if ir_json is not None:
            ir_source = 'rust'
    except Exception:
        ir_json = None

    # If rust pipeline not available, try lightweight DSL parser to produce IR-like dict
    if not ir_json:
        try:
            parsed_ir = _parse_dsl_to_ir(dsl_text or "")
            # accept parsed IR if it contains useful information
            if parsed_ir and (parsed_ir.get('entities') or parsed_ir.get('motions') or parsed_ir.get('trajectories') or parsed_ir.get('timelines') or parsed_ir.get('compound_motions')):
                ir_json = parsed_ir
                ir_source = 'sample'
        except Exception:
            ir_json = None


    # If we got an IR, construct a multi-entity snapshot from entities and set server-side targets
    if ir_json and isinstance(ir_json, dict):
        entities = ir_json.get("entities") or []
        new_objs = []
        for idx, ent in enumerate(entities):
            ent_id = ent.get("id") or f"entity_{idx}"
            ent_id = str(ent_id)
            kind = ent.get("kind") or ent.get("type") or "entity"
            comps = ent.get("components") or {}
            pos = None
            # search for a transform component or any property named 'position'
            for comp_name, comp_val in comps.items():
                if isinstance(comp_val, dict):
                    props = comp_val.get("properties")
                    if isinstance(props, dict):
                        p = props.get("position")
                        if isinstance(p, list) and len(p) == 3:
                            pos = [float(p[0]), float(p[1]), float(p[2])]
                            break
            # detect geometry primitive if present
            primitive = None
            for comp_name, comp_val in comps.items():
                if isinstance(comp_val, dict):
                    props = comp_val.get("properties")
                    if isinstance(props, dict):
                        prim = props.get("primitive")
                        if prim:
                            primitive = prim
                            break
            if not pos:
                pos = [0.0, 0.0, 0.0]
            # If there is no existing current object, set initial current pos to target
            existing = next((o for o in _snapshot.get("objects", []) if str(o.get("id")) == ent_id), None)
            if not existing:
                entry = {"id": ent_id, "position": pos, "kind": kind, "label": ent_id}
                if primitive:
                    entry["primitive"] = primitive
                new_objs.append(entry)
            else:
                # Preserve current position but ensure id/labels
                entry = {"id": ent_id, "position": existing.get("position", pos), "kind": kind, "label": ent_id}
                if primitive:
                    entry["primitive"] = primitive
                new_objs.append(entry)
            # Update server-side target
            _snapshot_targets[ent_id] = pos
        _snapshot["objects"] = new_objs
        # Clear previously scheduled motions and schedule new ones from IR
        try:
            _active_motions.clear()
            _schedule_motions_from_ir(ir_json)
        except Exception:
            pass
    else:
        # Fallback to simple parser if Rust pipeline didn't produce entities: treat as single-target update
        parsed = _parse_simple_dsl(dsl_text or "")
        pos = parsed.get("position")
        if pos:
            if len(_snapshot.get("objects", [])) > 0:
                # set target for first object
                first_id = str(_snapshot["objects"][0].get("id", "obj_0"))
                _snapshot_targets[first_id] = pos
            else:
                # create object and target
                oid = "obj_0"
                _snapshot["objects"] = [{"id": oid, "position": pos, "kind": "entity", "label": oid}]
                _snapshot_targets[oid] = pos

    # Update a timestamp and tick for client visibility
    _snapshot["timestamp"] = time.time()
    _snapshot["tick"] = _snapshot.get("tick", 0) + 1

    # Return IR and snapshot and indicate source (rust or sample parser)
    if ir_json:
        return JSONResponse(content={"ir": ir_json, "snapshot": _snapshot, "ir_source": ir_source or "unknown", "ir_error": _last_compile_error})
    return JSONResponse(content={"snapshot": _snapshot, "ir_source": ir_source, "ir_error": _last_compile_error})


@app.websocket("/ws")
async def websocket_endpoint(websocket: WebSocket):
    await websocket.accept()
    try:
        while True:
            # update mock snapshot tick and advance each object's position toward its target
            _snapshot["tick"] += 1
            _snapshot["timestamp"] = time.time()
            # Apply active compound motions: compute current targets for any scheduled motions
            try:
                now = time.time()
                remove_idxs = []
                for i, m in enumerate(list(_active_motions)):
                    st = float(m.get('start_time', now))
                    et = float(m.get('end_time', now))
                    if now < st:
                        # not started yet
                        continue
                    duration = max(1e-6, et - st)
                    t = min(1.0, (now - st) / duration)
                    sp = m.get('start_pos', [0.0, 0.0, 0.0])
                    # Path-based motions: helix/circular/orbital with arbitrary normal + easing
                    if m.get('path_type'):
                        ptype = str(m.get('path_type')).lower()
                        params = m.get('path_params') or {}
                        # choose simple implementations for common path types
                        try:
                            center = params.get('center') or params.get('centre') or [0.0, 0.0, 0.0]
                            center = [float(center[0]), float(center[1]), float(center[2])]
                        except Exception:
                            center = [0.0, 0.0, 0.0]
                        radius = float(params.get('radius') or 1.0)
                        pitch = float(params.get('pitch') or 0.0)
                        turns = float(params.get('turns') or 1.0)
                        start_angle = float(params.get('start_angle') or 0.0)
                        easing_name = (m.get('easing') or (params.get('easing') if isinstance(params, dict) else None) or 'linear')
                        # angle along path
                        import math
                        raw_angle = start_angle + 2.0 * math.pi * turns * t
                        # apply easing to t if requested
                        def apply_ease(tt, mode):
                            if not mode:
                                return tt
                            mode = str(mode).lower()
                            if mode == 'linear':
                                return tt
                            if mode == 'ease_in':
                                return tt * tt
                            if mode == 'ease_out':
                                return 1 - (1 - tt) * (1 - tt)
                            if mode == 'ease_in_out':
                                if tt < 0.5:
                                    return 2 * tt * tt
                                else:
                                    return -1 + (4 - 2 * tt) * tt
                            return tt

                        et = apply_ease(t, easing_name)
                        angle = start_angle + 2.0 * math.pi * turns * et
                        # compute plane basis from normal/axis
                        normal = None
                        try:
                            normal = m.get('normal') or (params.get('normal') if isinstance(params, dict) else None) or params.get('axis') if isinstance(params, dict) else None
                        except Exception:
                            normal = None
                        def normalize(v):
                            import math
                            l = math.sqrt(v[0]*v[0]+v[1]*v[1]+v[2]*v[2])
                            if l <= 1e-9:
                                return [0.0,1.0,0.0]
                            return [v[0]/l, v[1]/l, v[2]/l]

                        if normal:
                            try:
                                n = normalize([float(normal[0]), float(normal[1]), float(normal[2])])
                            except Exception:
                                n = [0.0, 1.0, 0.0]
                        else:
                            n = [0.0, 1.0, 0.0]

                        # pick arbitrary vector not parallel to n
                        import math
                        if abs(n[1]) < 0.9:
                            arbitrary = [0.0, 1.0, 0.0]
                        else:
                            arbitrary = [1.0, 0.0, 0.0]

                        # cross product u = normalize(cross(arbitrary, n))
                        ux = arbitrary[1]*n[2] - arbitrary[2]*n[1]
                        uy = arbitrary[2]*n[0] - arbitrary[0]*n[2]
                        uz = arbitrary[0]*n[1] - arbitrary[1]*n[0]
                        u = normalize([ux, uy, uz])
                        # v = cross(n, u)
                        vx = n[1]*u[2] - n[2]*u[1]
                        vy = n[2]*u[0] - n[0]*u[2]
                        vz = n[0]*u[1] - n[1]*u[0]
                        v = normalize([vx, vy, vz])

                        if ptype in ('helix', 'helical'):
                            # helix: rotate in u/v plane, move along n by pitch*turns*et
                            cx = center[0] + radius * (u[0]*math.cos(angle) + v[0]*math.sin(angle))
                            cy = center[1] + radius * (u[1]*math.cos(angle) + v[1]*math.sin(angle))
                            cz = center[2] + radius * (u[2]*math.cos(angle) + v[2]*math.sin(angle))
                            # move along normal
                            cx += n[0] * (pitch * turns * et)
                            cy += n[1] * (pitch * turns * et)
                            cz += n[2] * (pitch * turns * et)
                            tgt = [cx, cy, cz]
                        else:
                            # circular/orbital in plane perpendicular to n
                            cx = center[0] + radius * (u[0]*math.cos(angle) + v[0]*math.sin(angle))
                            cy = center[1] + radius * (u[1]*math.cos(angle) + v[1]*math.sin(angle))
                            cz = center[2] + radius * (u[2]*math.cos(angle) + v[2]*math.sin(angle))
                            tgt = [cx, cy, cz]
                    else:
                        ep = m.get('end_pos', sp)
                        # linear interpolation
                        tgt = [sp[j] + (ep[j] - sp[j]) * t for j in range(3)]
                    _snapshot_targets[str(m.get('entity_id'))] = tgt
                    if t >= 1.0:
                        remove_idxs.append(i)
                # remove completed motions
                if remove_idxs:
                    # remove by identity to be safe
                    remaining = []
                    for m in _active_motions:
                        et = float(m.get('end_time', now))
                        if now < et:
                            remaining.append(m)
                    _active_motions[:] = remaining
            except Exception:
                pass
            cfg = _movement_config
            speed = float(cfg.get("speed", 0.1))
            quantize = bool(cfg.get("quantize", True))

            for i, obj in enumerate(_snapshot.get("objects", [])):
                try:
                    oid = str(obj.get("id", i))
                    cur = obj.get("position") or [0.0, 0.0, 0.0]
                    tgt = _snapshot_targets.get(oid, cur)
                    # ensure float lists
                    cur = [float(cur[0]), float(cur[1]), float(cur[2])]
                    tgt = [float(tgt[0]), float(tgt[1]), float(tgt[2])]
                    if quantize:
                        # step per-axis by speed
                        for ax in range(3):
                            delta = tgt[ax] - cur[ax]
                            if abs(delta) <= speed:
                                cur[ax] = tgt[ax]
                            else:
                                cur[ax] += (speed if delta > 0 else -speed)
                    else:
                        # smooth interpolation using speed as factor (clamped)
                        factor = max(0.001, min(0.9, speed))
                        for ax in range(3):
                            cur[ax] += (tgt[ax] - cur[ax]) * factor
                    obj["position"] = cur
                except Exception:
                    pass
            await websocket.send_text(json.dumps(_snapshot))
            await asyncio.sleep(0.1)
    except Exception:
        await websocket.close()
