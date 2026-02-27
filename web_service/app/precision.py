"""Precision and measurement helpers for IR scenes."""

from __future__ import annotations

from dataclasses import dataclass
from itertools import combinations
from math import acos, pi, sqrt
from typing import Any


@dataclass
class BoundingBox:
    min_point: list[float]
    max_point: list[float]


def measure_distance(entity_a_ir: dict[str, Any], entity_b_ir: dict[str, Any]) -> float:
    pa = _entity_position(entity_a_ir)
    pb = _entity_position(entity_b_ir)
    return sqrt((pa[0] - pb[0]) ** 2 + (pa[1] - pb[1]) ** 2 + (pa[2] - pb[2]) ** 2)


def measure_angle(entity_a_ir: dict[str, Any], entity_b_ir: dict[str, Any]) -> float:
    va = _entity_axis(entity_a_ir)
    vb = _entity_axis(entity_b_ir)
    magnitude = _norm(va) * _norm(vb)
    if magnitude <= 1e-9:
        return 0.0
    cosine = max(-1.0, min(1.0, _dot(va, vb) / magnitude))
    return acos(cosine) * 180.0 / pi


def check_coincident(entity_a_ir: dict[str, Any], entity_b_ir: dict[str, Any], tolerance: float) -> bool:
    return measure_distance(entity_a_ir, entity_b_ir) <= tolerance


def compute_bounding_box(ir_scene: dict[str, Any]) -> BoundingBox:
    entities = ir_scene.get("entities", [])
    if not entities:
        return BoundingBox(min_point=[0.0, 0.0, 0.0], max_point=[0.0, 0.0, 0.0])

    mins = [float("inf"), float("inf"), float("inf")]
    maxs = [float("-inf"), float("-inf"), float("-inf")]

    for entity in entities:
        position = _entity_position(entity)
        half_extents = _entity_half_extents(entity)
        for axis in range(3):
            mins[axis] = min(mins[axis], position[axis] - half_extents[axis])
            maxs[axis] = max(maxs[axis], position[axis] + half_extents[axis])

    return BoundingBox(min_point=mins, max_point=maxs)


def compute_volume(entity_ir: dict[str, Any]) -> float:
    primitive = _entity_primitive(entity_ir)
    dims = _entity_dimensions(entity_ir)
    if primitive == "box":
        return abs(dims[0] * dims[1] * dims[2])
    if primitive == "sphere":
        radius = max(dims) / 2.0
        return 4.0 / 3.0 * pi * radius**3
    if primitive == "cylinder":
        radius = max(dims[0], dims[2]) / 2.0
        height = abs(dims[1])
        return pi * radius**2 * height
    if primitive == "cone":
        radius = max(dims[0], dims[2]) / 2.0
        height = abs(dims[1])
        return (pi * radius**2 * height) / 3.0
    if primitive == "torus":
        major = abs(dims[0]) / 2.0
        minor = abs(dims[1]) / 2.0
        return 2.0 * pi**2 * major * minor**2
    return 0.0


def compute_measurements(ir_scene: dict[str, Any]) -> dict[str, Any]:
    entities = ir_scene.get("entities", [])
    measurements: dict[str, Any] = {
        "entity_volumes": {},
        "entity_distances": [],
        "bounding_box": None,
    }

    for entity in entities:
        entity_id = str(entity.get("id", ""))
        measurements["entity_volumes"][entity_id] = compute_volume(entity)

    for left, right in combinations(entities, 2):
        measurements["entity_distances"].append(
            {
                "entity_a": left.get("id"),
                "entity_b": right.get("id"),
                "distance": measure_distance(left, right),
                "angle": measure_angle(left, right),
            }
        )

    bounds = compute_bounding_box(ir_scene)
    measurements["bounding_box"] = {
        "min": bounds.min_point,
        "max": bounds.max_point,
    }

    return measurements


def _entity_position(entity_ir: dict[str, Any]) -> list[float]:
    transform = _component(entity_ir, "transform")
    value = _property(transform, "position", [0.0, 0.0, 0.0])
    vector = _to_vector3(value)
    return vector


def _entity_axis(entity_ir: dict[str, Any]) -> list[float]:
    transform = _component(entity_ir, "transform")
    rotation = _property(transform, "rotation", [0.0, 0.0, 0.0])
    rot = _to_vector3(rotation)
    axis = [rot[0] + 1.0, rot[1] + 1.0, rot[2] + 1.0]
    if _norm(axis) <= 1e-9:
        return [0.0, 1.0, 0.0]
    return axis


def _entity_dimensions(entity_ir: dict[str, Any]) -> list[float]:
    solid = _component(entity_ir, "solid")
    geometry = _component(entity_ir, "geometry")
    value = _property(solid, "dimensions")
    if value is None:
        value = _property(geometry, "dimensions", [1.0, 1.0, 1.0])
    return _to_vector3(value)


def _entity_half_extents(entity_ir: dict[str, Any]) -> list[float]:
    dims = _entity_dimensions(entity_ir)
    return [abs(dims[0]) / 2.0, abs(dims[1]) / 2.0, abs(dims[2]) / 2.0]


def _entity_primitive(entity_ir: dict[str, Any]) -> str:
    solid = _component(entity_ir, "solid")
    geometry = _component(entity_ir, "geometry")
    primitive = _property(solid, "primitive")
    if primitive is None:
        primitive = _property(geometry, "primitive", "box")
    if isinstance(primitive, str):
        return primitive.lower()
    return str(primitive).lower()


def _component(entity_ir: dict[str, Any], name: str) -> dict[str, Any]:
    components = entity_ir.get("components", {})
    component = components.get(name) if isinstance(components, dict) else None
    if isinstance(component, dict):
        return component
    return {}


def _property(component: dict[str, Any], key: str, default: Any = None) -> Any:
    props = component.get("properties", {}) if isinstance(component, dict) else {}
    if not isinstance(props, dict):
        return default
    if key not in props:
        return default
    return _unwrap_ir_value(props[key])


def _unwrap_ir_value(value: Any) -> Any:
    if isinstance(value, dict) and len(value) == 1:
        tag, payload = next(iter(value.items()))
        if tag in {"Number", "String", "Identifier", "Boolean", "Vector3", "Matrix3", "List"}:
            if tag == "List" and isinstance(payload, list):
                return [_unwrap_ir_value(item) for item in payload]
            return payload
    return value


def _to_vector3(value: Any) -> list[float]:
    if isinstance(value, (list, tuple)) and len(value) >= 3:
        return [float(value[0]), float(value[1]), float(value[2])]
    if isinstance(value, (int, float)):
        scalar = float(value)
        return [scalar, scalar, scalar]
    return [0.0, 0.0, 0.0]


def _dot(a: list[float], b: list[float]) -> float:
    return a[0] * b[0] + a[1] * b[1] + a[2] * b[2]


def _norm(vector: list[float]) -> float:
    return sqrt(_dot(vector, vector))
