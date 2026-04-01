export function cloneDescription(value) {
  return structuredClone(value || defaultDescription());
}

export function defaultDescription() {
  return {
    name: "new_model",
    unit: "mm",
    precision: 0.01,
    summary: "",
    shapes: [],
    features: [],
    constraints: [],
    notes: "",
  };
}

export function normalizeDescription(input) {
  const description = cloneDescription(input);
  if (!Array.isArray(description.shapes)) description.shapes = [];
  if (!Array.isArray(description.features)) description.features = [];
  if (!Array.isArray(description.constraints)) description.constraints = [];
  return description;
}

export class DescriptionHistory {
  constructor(limit = 120) {
    this.limit = Math.max(10, Number(limit) || 120);
    this.undoStack = [];
    this.redoStack = [];
  }

  push(snapshot, label = "edit") {
    const payload = { snapshot: cloneDescription(snapshot), label: String(label || "edit"), at: Date.now() };
    this.undoStack.push(payload);
    if (this.undoStack.length > this.limit) this.undoStack.shift();
    this.redoStack = [];
  }

  undo(current) {
    if (!this.undoStack.length) return null;
    const previous = this.undoStack.pop();
    this.redoStack.push({ snapshot: cloneDescription(current), label: previous.label, at: Date.now() });
    return cloneDescription(previous.snapshot);
  }

  redo(current) {
    if (!this.redoStack.length) return null;
    const next = this.redoStack.pop();
    this.undoStack.push({ snapshot: cloneDescription(current), label: next.label, at: Date.now() });
    return cloneDescription(next.snapshot);
  }

  canUndo() {
    return this.undoStack.length > 0;
  }

  canRedo() {
    return this.redoStack.length > 0;
  }
}

export function selectedShapes(description, ids) {
  const set = new Set((ids || []).map((id) => String(id)));
  return (description.shapes || []).filter((shape) => set.has(String(shape.id)));
}

export function applyShapeFlags(description, ids, patch) {
  const next = normalizeDescription(description);
  const set = new Set((ids || []).map((id) => String(id)));
  for (const shape of next.shapes) {
    if (!set.has(String(shape.id))) continue;
    Object.assign(shape, patch);
  }
  return next;
}

export function clearShapeFlag(description, ids, keys) {
  const next = normalizeDescription(description);
  const set = new Set((ids || []).map((id) => String(id)));
  const keyList = Array.isArray(keys) ? keys : [keys];
  for (const shape of next.shapes) {
    if (!set.has(String(shape.id))) continue;
    for (const key of keyList) {
      delete shape[key];
    }
  }
  return next;
}

export function hideShapes(description, ids) {
  return applyShapeFlags(description, ids, { hidden: true });
}

export function showAllShapes(description) {
  const next = normalizeDescription(description);
  for (const shape of next.shapes) {
    delete shape.hidden;
    delete shape.isolate;
  }
  return next;
}

export function lockShapes(description, ids) {
  return applyShapeFlags(description, ids, { locked: true });
}

export function unlockShapes(description, ids) {
  return clearShapeFlag(description, ids, "locked");
}

export function isolateShapes(description, ids) {
  const next = normalizeDescription(description);
  const set = new Set((ids || []).map((id) => String(id)));
  for (const shape of next.shapes) {
    if (set.has(String(shape.id))) {
      shape.isolate = true;
      delete shape.hidden;
    } else {
      delete shape.isolate;
      shape.hidden = true;
    }
  }
  return next;
}

export function clearIsolation(description) {
  const next = normalizeDescription(description);
  for (const shape of next.shapes) {
    delete shape.isolate;
  }
  return next;
}

export function groupShapes(description, ids) {
  const next = normalizeDescription(description);
  const set = new Set((ids || []).map((id) => String(id)));
  if (set.size < 2) return next;
  const groupId = `group_${Date.now()}`;
  for (const shape of next.shapes) {
    if (!set.has(String(shape.id))) continue;
    shape.group_id = groupId;
  }
  return next;
}

export function ungroupShapes(description, ids) {
  const next = normalizeDescription(description);
  const set = new Set((ids || []).map((id) => String(id)));
  for (const shape of next.shapes) {
    if (!set.has(String(shape.id))) continue;
    delete shape.group_id;
  }
  return next;
}

export function collectObjectStateMap(description) {
  const map = new Map();
  const shapes = Array.isArray(description?.shapes) ? description.shapes : [];
  for (const shape of shapes) {
    map.set(String(shape.id), {
      hidden: !!shape.hidden,
      locked: !!shape.locked,
      isolate: !!shape.isolate,
    });
  }
  return map;
}

export function createStepLikeExport(description) {
  const next = normalizeDescription(description);
  const lines = [
    "ISO-10303-21;",
    "HEADER;",
    "FILE_DESCRIPTION(('edu3d lightweight export'),'2;1');",
    `FILE_NAME('${String(next.name || "model")}.step','${new Date().toISOString()}',('edu3d'),('edu3d'),'codex','edu3d','');`,
    "ENDSEC;",
    "DATA;",
  ];

  let index = 1;
  for (const shape of next.shapes) {
    if (shape.hidden || shape.suppressed) continue;
    const dims = shape.dimensions || {};
    const px = vectorAt(shape.position, 0);
    const py = vectorAt(shape.position, 1);
    const pz = vectorAt(shape.position, 2);
    lines.push(`#${index}=/* SHAPE ${shape.id} TYPE=${shape.type} */;`);
    lines.push(
      `#${index + 1}=/* POS=(${px.toFixed(4)},${py.toFixed(4)},${pz.toFixed(4)}) DIMS=${JSON.stringify(dims)} */;`,
    );
    index += 2;
  }

  lines.push("ENDSEC;");
  lines.push("END-ISO-10303-21;");
  return lines.join("\n");
}

function vectorAt(value, index) {
  if (!Array.isArray(value)) return 0;
  return Number(value[index] || 0);
}

export function parseAutosaveEnvelope(raw) {
  if (!raw) return null;
  try {
    const value = JSON.parse(raw);
    if (!value || typeof value !== "object") return null;
    if (!value.description || typeof value.description !== "object") return null;
    return value;
  } catch {
    return null;
  }
}
