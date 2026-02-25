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
from pathlib import Path

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


@app.get("/")
async def index():
    return FileResponse(STATIC_DIR / "index.html")


@app.get("/api/snapshot")
async def get_snapshot():
    return JSONResponse(content=_snapshot)


@app.websocket("/ws")
async def websocket_endpoint(websocket: WebSocket):
    await websocket.accept()
    try:
        while True:
            # update mock snapshot
            _snapshot["tick"] += 1
            _snapshot["timestamp"] = time.time()
            _snapshot["objects"][0]["position"][0] += 0.05
            await websocket.send_text(json.dumps(_snapshot))
            await asyncio.sleep(0.1)
    except Exception:
        await websocket.close()
