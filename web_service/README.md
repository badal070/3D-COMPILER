FastAPI visualizer for edu3d

Quick start (create virtualenv then install):

python -m venv .venv
source .venv/bin/activate
pip install -r requirements.txt

Run:

uvicorn app.main:app --reload --host 127.0.0.1 --port 8000

Open http://127.0.0.1:8000/ to view the Three.js visualizer. The app currently
serves a mock snapshot stream over WebSocket at `/ws` — replace integration
with the runtime by POSTing orchestrator responses or adding IPC to read
runtime snapshots.
