"""LLM bridge for generating and refining structured model descriptions."""

from __future__ import annotations

import asyncio
import json
import os
import re
from pathlib import Path
from typing import Any

try:
    from anthropic import Anthropic
except Exception:  # pragma: no cover - dependency optional in CI
    Anthropic = None

PROMPT_PATH = Path(__file__).resolve().parent / "prompts" / "description_system.txt"
DEFAULT_MODEL = os.getenv("ANTHROPIC_MODEL", "claude-sonnet-4-6")


def _load_system_prompt(unit_system: str, precision: float) -> str:
    base = PROMPT_PATH.read_text(encoding="utf-8") if PROMPT_PATH.exists() else (
        "You are a precision 3D modeling assistant. Output only valid JSON."
    )
    return base.replace("{unit_system}", unit_system).replace("{precision}", str(precision))


async def generate_description(
    user_prompt: str,
    current_description: dict[str, Any] | None,
    unit_system: str,
    precision: float,
) -> dict[str, Any]:
    if Anthropic is None or not os.getenv("ANTHROPIC_API_KEY"):
        return _fallback_description(user_prompt, current_description, unit_system, precision)

    system_prompt = _load_system_prompt(unit_system, precision)
    user_parts = [f"User request:\n{user_prompt.strip()}"]
    if current_description:
        user_parts.append(
            "Current description JSON:\n" + json.dumps(current_description, indent=2, sort_keys=True)
        )

    response_text = await _request_anthropic_json(system_prompt, "\n\n".join(user_parts))
    parsed = _extract_json(response_text)
    if parsed is None:
        correction_prompt = (
            "Your previous response was not valid JSON. Return only valid JSON matching the schema."
        )
        response_text = await _request_anthropic_json(system_prompt, correction_prompt)
        parsed = _extract_json(response_text)

    if parsed is None:
        return _fallback_description(user_prompt, current_description, unit_system, precision)

    return parsed


async def refine_description(
    user_prompt: str,
    current_description: dict[str, Any],
    unit_system: str,
    precision: float,
) -> tuple[dict[str, Any], list[str]]:
    updated = await generate_description(
        user_prompt=user_prompt,
        current_description=current_description,
        unit_system=unit_system,
        precision=precision,
    )
    changed = diff_field_paths(current_description, updated)
    return updated, changed


async def explain_description(description: dict[str, Any]) -> str:
    if Anthropic is None or not os.getenv("ANTHROPIC_API_KEY"):
        return _fallback_explanation(description)

    client = Anthropic(api_key=os.getenv("ANTHROPIC_API_KEY"))

    def _call() -> str:
        response = client.messages.create(
            model=DEFAULT_MODEL,
            temperature=0.2,
            max_tokens=512,
            system="Explain the given model description in plain English for a CAD user.",
            messages=[
                {
                    "role": "user",
                    "content": json.dumps(description, indent=2, sort_keys=True),
                }
            ],
        )
        return _extract_text(response)

    try:
        return await asyncio.to_thread(_call)
    except Exception:
        return _fallback_explanation(description)


async def _request_anthropic_json(system_prompt: str, user_content: str) -> str:
    client = Anthropic(api_key=os.getenv("ANTHROPIC_API_KEY"))

    def _call() -> str:
        response = client.messages.create(
            model=DEFAULT_MODEL,
            temperature=0.3,
            max_tokens=2500,
            system=system_prompt,
            messages=[{"role": "user", "content": user_content}],
        )
        return _extract_text(response)

    return await asyncio.to_thread(_call)


def _extract_text(response: Any) -> str:
    parts: list[str] = []
    for block in getattr(response, "content", []):
        text = getattr(block, "text", None)
        if text:
            parts.append(text)
    return "\n".join(parts).strip()


def _extract_json(text: str) -> dict[str, Any] | None:
    if not text:
        return None

    text = text.strip()
    if text.startswith("```"):
        text = re.sub(r"^```(?:json)?", "", text).strip()
        text = re.sub(r"```$", "", text).strip()

    try:
        parsed = json.loads(text)
        if isinstance(parsed, dict):
            return parsed
    except Exception:
        pass

    start = text.find("{")
    end = text.rfind("}")
    if start >= 0 and end > start:
        try:
            parsed = json.loads(text[start : end + 1])
            if isinstance(parsed, dict):
                return parsed
        except Exception:
            return None

    return None


def diff_field_paths(before: Any, after: Any, path: str = "") -> list[str]:
    changes: list[str] = []
    if type(before) is not type(after):
        return [path or "/"]

    if isinstance(before, dict):
        keys = sorted(set(before.keys()) | set(after.keys()))
        for key in keys:
            child_path = f"{path}/{key}" if path else f"/{key}"
            if key not in before or key not in after:
                changes.append(child_path)
                continue
            changes.extend(diff_field_paths(before[key], after[key], child_path))
        return changes

    if isinstance(before, list):
        max_len = max(len(before), len(after))
        for index in range(max_len):
            child_path = f"{path}/{index}" if path else f"/{index}"
            if index >= len(before) or index >= len(after):
                changes.append(child_path)
                continue
            changes.extend(diff_field_paths(before[index], after[index], child_path))
        return changes

    if before != after:
        return [path or "/"]

    return changes


def _fallback_description(
    user_prompt: str,
    current_description: dict[str, Any] | None,
    unit_system: str,
    precision: float,
) -> dict[str, Any]:
    if current_description:
        # Best-effort refinement for common dimension update commands.
        refined = json.loads(json.dumps(current_description))
        dims = re.findall(r"(width|height|depth|radius)\s*(?:to|=)?\s*([0-9]+(?:\.[0-9]+)?)", user_prompt, re.I)
        if refined.get("shapes") and dims:
            for key, value in dims:
                refined["shapes"][0].setdefault("dimensions", {})[key.lower()] = float(value)
        return refined

    match = re.search(
        r"([0-9]+(?:\.[0-9]+)?)\s*(?:mm|cm|m)?\s*[x×]\s*([0-9]+(?:\.[0-9]+)?)\s*(?:mm|cm|m)?\s*[x×]\s*([0-9]+(?:\.[0-9]+)?)",
        user_prompt,
        re.I,
    )
    width, height, depth = (50.0, 30.0, 10.0)
    if match:
        width, height, depth = (float(match.group(1)), float(match.group(2)), float(match.group(3)))

    description = {
        "name": normalize_name_from_prompt(user_prompt),
        "unit": "mm" if unit_system == "SI" else "in",
        "precision": precision,
        "summary": user_prompt.strip(),
        "shapes": [
            {
                "id": "body",
                "type": "box",
                "label": "Main body",
                "dimensions": {"width": width, "height": height, "depth": depth},
                "position": [0, 0, 0],
                "material": "steel",
            }
        ],
        "features": [],
        "constraints": [],
        "notes": "Generated via local fallback heuristic.",
    }

    if re.search(r"m6|bolt hole", user_prompt, re.I):
        description["shapes"].append(
            {
                "id": "hole_1",
                "type": "cylinder",
                "label": "M6 clearance hole",
                "dimensions": {"radius": 3.1, "depth": depth},
                "position": [0, 0, 0],
                "operation": "subtract",
                "target": "body",
            }
        )

    fillet_match = re.search(r"([0-9]+(?:\.[0-9]+)?)\s*(?:mm)?\s*fillet", user_prompt, re.I)
    if fillet_match:
        description["features"].append(
            {
                "type": "fillet",
                "label": "Auto fillet",
                "target": "body",
                "edges": ["top_front", "top_back"],
                "radius": float(fillet_match.group(1)),
            }
        )

    return description


def _fallback_explanation(description: dict[str, Any]) -> str:
    name = description.get("name", "model")
    shapes = description.get("shapes", [])
    features = description.get("features", [])
    constraints = description.get("constraints", [])
    return (
        f"{name} contains {len(shapes)} shape(s), {len(features)} feature(s), "
        f"and {len(constraints)} constraint(s)."
    )


def normalize_name_from_prompt(prompt: str) -> str:
    cleaned = re.sub(r"[^A-Za-z0-9]+", "_", prompt.strip().lower()).strip("_")
    if not cleaned:
        return "model_description"
    return cleaned[:40]
