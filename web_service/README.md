# edu3d Modeling Service

## Setup

```bash
python -m venv .venv
source .venv/bin/activate
pip install -r requirements.txt
```

Set `ANTHROPIC_API_KEY` to enable live LLM description generation.

## Run

```bash
uvicorn app.main:app --reload --host 127.0.0.1 --port 8000
```

Open `http://127.0.0.1:8000/`.

## APIs

- `POST /api/model/save` -> save `.dsl` model file
- `GET /api/model/load/{name}` -> load model source
- `GET /api/model/list` -> list saved models
- `POST /api/model/compile` -> compile DSL to IR + measurements
- `POST /api/llm/describe` -> prompt -> model description JSON
- `POST /api/llm/refine` -> refine existing description JSON
- `POST /api/description/to_dsl` -> deterministic description -> DSL translation
- `POST /api/llm/explain` -> plain-language explanation of a description
- `GET /ws` -> event-driven IR WebSocket stream (broadcasts on successful compile)
