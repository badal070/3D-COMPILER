"""Deterministic Model Description -> DSL translation."""

from __future__ import annotations

import json
import re
from dataclasses import dataclass
from typing import Any

try:
    from jsonschema import Draft202012Validator
except Exception:  # pragma: no cover - optional dependency fallback
    Draft202012Validator = None

_IDENTIFIER_RE = re.compile(r"^[A-Za-z_][A-Za-z0-9_]*$")

MODEL_DESCRIPTION_SCHEMA: dict[str, Any] = {
    "$schema": "https://json-schema.org/draft/2020-12/schema",
    "type": "object",
    "required": ["name", "unit", "precision", "shapes"],
    "properties": {
        "name": {"type": "string", "minLength": 1},
        "unit": {"type": "string", "minLength": 1},
        "precision": {"type": "number", "exclusiveMinimum": 0},
        "summary": {"type": "string"},
        "shapes": {
            "type": "array",
            "items": {
                "type": "object",
                "required": ["id", "type", "label", "dimensions", "position"],
                "properties": {
                    "id": {"type": "string", "minLength": 1},
                    "type": {"type": "string", "minLength": 1},
                    "label": {"type": "string", "minLength": 1},
                    "dimensions": {"type": "object", "minProperties": 1},
                    "position": {
                        "type": "array",
                        "items": {"type": "number"},
                        "minItems": 3,
                        "maxItems": 3,
                    },
                    "rotation": {
                        "type": "array",
                        "items": {"type": "number"},
                        "minItems": 3,
                        "maxItems": 3,
                    },
                    "scale": {
                        "type": "array",
                        "items": {"type": "number"},
                        "minItems": 3,
                        "maxItems": 3,
                    },
                    "material": {"type": "string"},
                    "operation": {
                        "type": "string",
                        "enum": ["subtract", "union", "intersect"],
                    },
                    "target": {"type": "string"},
                },
            },
        },
        "features": {"type": "array", "items": {"type": "object"}},
        "constraints": {"type": "array", "items": {"type": "object"}},
        "notes": {"type": "string"},
    },
}


@dataclass
class TranslationResult:
    dsl: str
    source_map: dict[str, str]


@dataclass
class ValidationIssue:
    path: str
    message: str


def validate_description(description: dict[str, Any]) -> list[ValidationIssue]:
    issues: list[ValidationIssue] = []
    if Draft202012Validator is None:
        # Fallback structural checks when jsonschema is unavailable.
        for key in ("name", "unit", "precision", "shapes"):
            if key not in description:
                issues.append(ValidationIssue(path=f"/{key}", message="is required"))
        if "shapes" in description and not isinstance(description["shapes"], list):
            issues.append(ValidationIssue(path="/shapes", message="must be an array"))
        return issues

    validator = Draft202012Validator(MODEL_DESCRIPTION_SCHEMA)
    for error in sorted(validator.iter_errors(description), key=lambda entry: list(entry.path)):
        path = "/" + "/".join(str(part) for part in error.path)
        issues.append(ValidationIssue(path=path or "/", message=error.message))
    return issues


def description_to_dsl(description: dict[str, Any]) -> TranslationResult:
    id_map: dict[str, str] = {}
    used_ids: set[str] = set()

    def canonical_id(raw: str) -> str:
        candidate = normalize_identifier(raw)
        if candidate not in used_ids:
            used_ids.add(candidate)
            return candidate
        index = 2
        while f"{candidate}_{index}" in used_ids:
            index += 1
        final = f"{candidate}_{index}"
        used_ids.add(final)
        return final

    for shape in description.get("shapes", []):
        original = str(shape.get("id", "shape"))
        id_map[original] = canonical_id(original)

    model_name = str(description.get("name", "model"))
    unit = str(description.get("unit", "mm"))
    unit_system = "SI" if unit.lower() in {"mm", "cm", "m"} else "Imperial"
    precision = float(description.get("precision", 0.01))

    lines: list[str] = []
    lines.extend(
        [
            "scene {",
            f'  name: "{escape_string(model_name)}"',
            "  version: 1",
            '  ir_version: "2.0.0"',
            f"  unit_system: {unit_system}",
            "  domain: modeling",
            f"  precision: {format_number(precision)}",
            "}",
            "",
            "library_imports {",
            '  modeling: "modeling_core"',
            "}",
            "",
        ]
    )

    material_names = sorted(
        {
            normalize_identifier(str(shape.get("material")))
            for shape in description.get("shapes", [])
            if shape.get("material")
        }
    )
    if material_names:
        lines.append("materials {")
        for material in material_names:
            lines.extend(
                [
                    f"  material {material} {{",
                    "    density: 1.0",
                    "    elasticity: 0.5",
                    "    friction: 0.3",
                    "  }",
                ]
            )
        lines.extend(["}", ""])

    for shape in description.get("shapes", []):
        original_id = str(shape.get("id", "shape"))
        shape_id = id_map[original_id]
        shape_type = str(shape.get("type", "box")).lower()
        primitive, dimensions = normalize_geometry(shape_type, shape.get("dimensions", {}))
        position = normalize_vector3(shape.get("position", [0, 0, 0]))
        rotation = normalize_vector3(shape.get("rotation", [0, 0, 0]))
        scale = normalize_scale3(shape.get("scale", [1, 1, 1]))

        lines.append(f"entity {shape_id} {{")
        lines.append("  kind: solid")
        lines.append("  components {")
        lines.append("    transform {")
        lines.append(f"      position: {format_vector(position)}")
        lines.append(f"      rotation: {format_vector(rotation)}")
        lines.append(f"      scale: {format_vector(scale)}")
        lines.append("    }")
        lines.append("    geometry {")
        lines.append(f"      primitive: {format_identifier(primitive)}")
        lines.append(f"      dimensions: {format_vector(dimensions)}")
        lines.append("    }")
        lines.append("    solid {")
        lines.append(f"      primitive: {format_identifier(primitive)}")
        lines.append(f"      dimensions: {format_vector(dimensions)}")
        lines.append("    }")

        material = shape.get("material")
        if material:
            lines.append("    material_ref {")
            lines.append(f"      name: {format_identifier(normalize_identifier(str(material)))}")
            lines.append("    }")

        lines.append("  }")
        lines.append("}")
        lines.append("")

    for shape in description.get("shapes", []):
        operation = str(shape.get("operation", "")).lower().strip()
        if operation not in {"subtract", "union", "intersect"}:
            continue

        tool_original = str(shape.get("id", "shape"))
        target_original = str(shape.get("target", "")).strip()
        if not target_original or target_original not in id_map:
            continue

        constraint_type = {
            "subtract": "boolean_subtract",
            "union": "boolean_union",
            "intersect": "boolean_intersect",
        }[operation]
        constraint_id = normalize_identifier(f"{tool_original}_{operation}")
        lines.append(f"constraint {constraint_id} {{")
        lines.append(f"  type: {constraint_type}")
        lines.append(f"  target: {id_map[target_original]}")
        lines.append(f"  tool: {id_map[tool_original]}")
        lines.append("}")
        lines.append("")

    for index, feature in enumerate(description.get("features", []), start=1):
        feature_type = str(feature.get("type", "")).lower().strip()
        feature_id = normalize_identifier(str(feature.get("id") or f"feature_{index}"))
        if feature_type not in {"fillet", "chamfer", "thread", "shell", "annotation"}:
            continue

        normalized_feature = dict(feature)
        if "target_edges" in normalized_feature and "edges" not in normalized_feature:
            normalized_feature["edges"] = normalized_feature.pop("target_edges")
        if "target_faces" in normalized_feature and "open_faces" not in normalized_feature:
            normalized_feature["open_faces"] = normalized_feature.pop("target_faces")

        lines.append(f"entity {feature_id} {{")
        lines.append("  kind: feature")
        lines.append("  components {")
        lines.append(f"    {feature_type} {{")

        for key, value in sorted(normalized_feature.items()):
            if key in {"id", "type", "label"}:
                continue
            lines.append(f"      {normalize_identifier(key)}: {format_value(value, id_map)}")

        lines.append("    }")
        lines.append("  }")
        lines.append("}")
        lines.append("")

    for index, constraint in enumerate(description.get("constraints", []), start=1):
        constraint_type = str(constraint.get("type", "")).lower().strip()
        if not constraint_type:
            continue

        constraint_id = normalize_identifier(str(constraint.get("id") or f"constraint_{index}"))
        lines.append(f"constraint {constraint_id} {{")
        lines.append(f"  type: {format_identifier(constraint_type)}")

        if "entities" in constraint and isinstance(constraint["entities"], list):
            entities = [id_map.get(str(item), normalize_identifier(str(item))) for item in constraint["entities"]]
            if len(entities) >= 1:
                lines.append(f"  entity_a: {entities[0]}")
            if len(entities) >= 2:
                lines.append(f"  entity_b: {entities[1]}")

        for key, value in sorted(constraint.items()):
            if key in {"id", "type", "label", "entities"}:
                continue
            lines.append(f"  {normalize_identifier(key)}: {format_value(value, id_map)}")

        lines.append("}")
        lines.append("")

    lines.extend(["timeline modeling_main {", "}", ""])
    dsl_source = "\n".join(lines)
    return TranslationResult(dsl=dsl_source, source_map=id_map)


def map_compiler_errors_to_description(
    compiler_errors: list[dict[str, Any]],
    source_map: dict[str, str],
) -> list[dict[str, Any]]:
    inverse_map = {dsl_id: source_id for source_id, dsl_id in source_map.items()}
    mapped: list[dict[str, Any]] = []

    for error in compiler_errors:
        message = str(error.get("message", ""))
        field_path = None
        for dsl_id, source_id in inverse_map.items():
            if dsl_id in message:
                field_path = f"/shapes[id={source_id}]"
                break

        mapped.append(
            {
                "code": error.get("code"),
                "message": message,
                "line": error.get("line"),
                "column": error.get("column"),
                "field_path": field_path,
            }
        )

    return mapped


def normalize_identifier(raw: str) -> str:
    text = raw.strip().lower()
    text = re.sub(r"[^a-zA-Z0-9_]+", "_", text)
    text = re.sub(r"_+", "_", text).strip("_")
    if not text:
        text = "item"
    if text[0].isdigit():
        text = f"_{text}"
    return text


def normalize_geometry(shape_type: str, dimensions: dict[str, Any]) -> tuple[str, list[float]]:
    dim = {str(key).lower(): float(value) for key, value in (dimensions or {}).items() if is_number(value)}

    if shape_type in {"box", "cube", "rect_prism", "rectangular_prism"}:
        return "box", [dim.get("width", 1.0), dim.get("height", 1.0), dim.get("depth", 1.0)]
    if shape_type == "sphere":
        radius = dim.get("radius", 1.0)
        diameter = radius * 2.0
        return "sphere", [diameter, diameter, diameter]
    if shape_type == "cylinder":
        radius = dim.get("radius", 0.5)
        depth = dim.get("depth", dim.get("height", 1.0))
        diameter = radius * 2.0
        return "cylinder", [diameter, depth, diameter]
    if shape_type == "cone":
        radius = dim.get("radius", 0.5)
        depth = dim.get("depth", dim.get("height", 1.0))
        diameter = radius * 2.0
        return "cone", [diameter, depth, diameter]
    if shape_type == "torus":
        major = dim.get("major_radius", dim.get("radius", 2.0))
        minor = dim.get("minor_radius", dim.get("tube", 0.3))
        return "torus", [major * 2.0, minor * 2.0, minor * 2.0]
    if shape_type == "plane":
        return "plane", [dim.get("width", 1.0), dim.get("height", 1.0), 0.0]

    return "box", [1.0, 1.0, 1.0]


def normalize_vector3(value: Any) -> list[float]:
    if isinstance(value, (list, tuple)) and len(value) >= 3:
        return [float(value[0]), float(value[1]), float(value[2])]
    return [0.0, 0.0, 0.0]


def normalize_scale3(value: Any) -> list[float]:
    scale = normalize_vector3(value)
    return [max(float(item), 0.001) for item in scale]


def format_value(value: Any, id_map: dict[str, str]) -> str:
    if isinstance(value, bool):
        return "true" if value else "false"
    if is_number(value):
        return format_number(float(value))
    if isinstance(value, str):
        mapped = id_map.get(value)
        if mapped:
            return mapped
        if _IDENTIFIER_RE.match(value):
            return value
        return f'"{escape_string(value)}"'
    if isinstance(value, (list, tuple)):
        return "[" + ", ".join(format_value(item, id_map) for item in value) + "]"
    if isinstance(value, dict):
        return f'"{escape_string(json.dumps(value, sort_keys=True))}"'
    return f'"{escape_string(json.dumps(value))}"'


def format_identifier(value: str) -> str:
    if _IDENTIFIER_RE.match(value):
        return value
    return normalize_identifier(value)


def format_vector(values: list[float]) -> str:
    return "[" + ", ".join(format_number(float(value)) for value in values[:3]) + "]"


def format_number(value: float) -> str:
    if value.is_integer():
        return str(int(value))
    return f"{value:.6f}".rstrip("0").rstrip(".")


def escape_string(value: str) -> str:
    return value.replace("\\", "\\\\").replace('"', '\\"')


def is_number(value: Any) -> bool:
    return isinstance(value, (int, float)) and not isinstance(value, bool)
