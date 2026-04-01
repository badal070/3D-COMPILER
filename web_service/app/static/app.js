import { SceneManager } from "./scene-manager.js";
import { createFeatureTree } from "./feature-tree.js";
import { createDescriptionPanel } from "./description-panel.js";
import { createDslEditor } from "./dsl-editor.js";
import { createGuiTools } from "./gui-tools.js";
import * as THREE from "https://esm.sh/three@0.160.1";
import {
  DescriptionHistory,
  clearIsolation,
  clearShapeFlag,
  cloneDescription,
  collectObjectStateMap,
  createStepLikeExport,
  groupShapes,
  hideShapes,
  isolateShapes,
  lockShapes,
  normalizeDescription,
  parseAutosaveEnvelope,
  selectedShapes,
  showAllShapes,
  ungroupShapes,
  unlockShapes,
} from "./model-state.mjs";

const canvas = document.getElementById("viewport-canvas");
const sceneManager = new SceneManager(canvas);

const compileStatus = document.getElementById("compile-status");
const compileErrors = document.getElementById("compile-errors");
const properties = document.getElementById("properties");
const wsStatus = document.getElementById("ws-status");
const booleanPairStatus = document.getElementById("boolean-pair-status");
const selectionStatus = document.getElementById("selection-status");
const measurementsPanel = document.getElementById("measurements");
const modelListSelect = document.getElementById("model-list");
const autosaveStatus = document.getElementById("autosave-status");
const checkpointList = document.getElementById("checkpoint-list");

const storage = {
  autosave: "edu3d.autosave.v1",
  checkpoints: "edu3d.checkpoints.v1",
};

let currentDescription = null;
let currentIr = null;
let dslEditor = null;
let compileTimer = null;
let translateTimer = null;
let autosaveTimer = null;

let selectedEntityId = null;
let selectedEntityIds = [];
let booleanSelection = { targetId: null, toolId: null };
let checkpoints = [];
let sectionEnabled = false;

const history = new DescriptionHistory(150);

const featureTree = createFeatureTree(document.getElementById("feature-tree"), {
  onSelect: handleFeatureTreeSelect,
  onMove: moveFeatureItem,
  onToggleSuppress: setFeatureSuppressed,
});

const descriptionPanel = createDescriptionPanel(document.getElementById("description-editor"), {
  onChange: (description) => {
    const normalized = normalizeDescription(description);
    commitDescription(normalized, {
      pushHistory: true,
      label: "JSON edit",
      translate: true,
      updateEditor: false,
      autosave: true,
    });
  },
});
currentDescription = normalizeDescription(descriptionPanel.getDescription());

bootstrap().catch((error) => {
  setCompileStatus(`Init failed: ${error.message || error}`, "err");
});

async function bootstrap() {
  dslEditor = await createDslEditor(document.getElementById("dsl-editor"), {
    onChange: () => {
      if (compileTimer) clearTimeout(compileTimer);
      compileTimer = setTimeout(() => compileCurrentDsl(), 800);
      scheduleAutosave("dsl edit");
    },
    onSave: () => compileCurrentDsl(),
  });

  sceneManager.setCallbacks({
    onSelect: (entity) => {
      selectedEntityId = entity?.id || null;
      if (selectedEntityId && !selectedEntityIds.includes(selectedEntityId)) {
        selectedEntityIds = [selectedEntityId];
      }
      featureTree.setActive(selectedEntityId || null);
      if (entity) renderProperties(entity);
      else renderDescriptionItemProperties(selectedEntityId);
      refreshBooleanPairStatus();
      refreshSelectionStatus();
    },
    onSelectionChange: (entities) => {
      selectedEntityIds = (entities || []).map((entity) => String(entity.id));
      if (selectedEntityId && !selectedEntityIds.includes(selectedEntityId)) {
        selectedEntityId = selectedEntityIds[0] || null;
      }
      refreshBooleanPairStatus();
      refreshSelectionStatus();
    },
    onTransform: handleTransformFromViewport,
  });

  bindUi();
  initWebSocket();
  createGuiTools({
    sceneManager,
    onAddShape: addShape,
    onMarkBooleanTarget: markBooleanTargetFromSelection,
    onMarkBooleanTool: markBooleanToolFromSelection,
    onApplyBooleanOperation: applyBooleanOperationFromSelection,
    onClearBooleanOperation: clearBooleanOperationFromSelection,
  });

  loadCheckpointsFromStorage();
  await refreshModelList();

  restoreAutosaveIfAvailable();
  syncViewportStates();
  updateTransformSpaceButton();
  refreshCheckpointList();

  await translateDescription();
  refreshBooleanPairStatus();
  refreshSelectionStatus();
}

function bindUi() {
  document.getElementById("compile-dsl")?.addEventListener("click", () => compileCurrentDsl());
  document
    .getElementById("translate-description")
    ?.addEventListener("click", () => translateDescription());

  document.getElementById("save-model")?.addEventListener("click", async () => {
    const name = window.prompt("Model filename", currentDescription.name || "model") || "model";
    const response = await api("/api/model/save", {
      method: "POST",
      body: { name, dsl_source: dslEditor.getValue() },
    });

    if (response.ok) {
      await refreshModelList(response.name);
      setCompileStatus(`Saved ${response.name}`, "ok");
    } else {
      setCompileStatus("Save failed", "err");
    }
  });

  document.getElementById("refresh-models")?.addEventListener("click", () => refreshModelList());
  document.getElementById("load-model")?.addEventListener("click", () => loadSelectedModel());

  document.getElementById("llm-send")?.addEventListener("click", () => runLlmDescribe(false));
  document.getElementById("llm-refine")?.addEventListener("click", () => runLlmDescribe(true));
  document.getElementById("llm-explain")?.addEventListener("click", runLlmExplain);

  document.getElementById("undo-action")?.addEventListener("click", () => undoDescription());
  document.getElementById("redo-action")?.addEventListener("click", () => redoDescription());

  document.getElementById("select-all")?.addEventListener("click", () => {
    sceneManager.selectAll(true);
  });
  document.getElementById("clear-selection")?.addEventListener("click", () => {
    sceneManager.clearSelection(true);
    selectedEntityId = null;
    selectedEntityIds = [];
    renderDescriptionItemProperties(null);
    refreshSelectionStatus();
  });

  document.getElementById("group-selection")?.addEventListener("click", () => {
    const ids = selectedShapeIds();
    if (ids.length < 2) {
      setCompileStatus("Select at least two shapes to group", "err");
      return;
    }
    const next = groupShapes(currentDescription, ids);
    commitDescription(next, { pushHistory: true, label: "Group selection", translate: true, updateEditor: true });
    setCompileStatus(`Grouped ${ids.length} shapes`, "ok");
  });

  document.getElementById("ungroup-selection")?.addEventListener("click", () => {
    const ids = selectedShapeIds();
    if (!ids.length) {
      setCompileStatus("Select grouped shapes to ungroup", "err");
      return;
    }
    const next = ungroupShapes(currentDescription, ids);
    commitDescription(next, { pushHistory: true, label: "Ungroup selection", translate: true, updateEditor: true });
    setCompileStatus(`Ungrouped ${ids.length} shapes`, "ok");
  });

  document.getElementById("hide-selection")?.addEventListener("click", () => {
    const ids = selectedShapeIds();
    if (!ids.length) {
      setCompileStatus("Select shapes to hide", "err");
      return;
    }
    const next = hideShapes(currentDescription, ids);
    commitDescription(next, { pushHistory: true, label: "Hide selection", translate: true, updateEditor: true });
    sceneManager.clearSelection(true);
    setCompileStatus(`Hidden ${ids.length} shapes`, "ok");
  });

  document.getElementById("show-all")?.addEventListener("click", () => {
    const shown = showAllShapes(currentDescription);
    const next = clearIsolation(shown);
    commitDescription(next, { pushHistory: true, label: "Show all", translate: true, updateEditor: true });
    setCompileStatus("All shapes visible", "ok");
  });

  document.getElementById("lock-selection")?.addEventListener("click", () => {
    const ids = selectedShapeIds();
    if (!ids.length) {
      setCompileStatus("Select shapes to lock", "err");
      return;
    }
    const next = lockShapes(currentDescription, ids);
    commitDescription(next, { pushHistory: true, label: "Lock selection", translate: true, updateEditor: true });
    setCompileStatus(`Locked ${ids.length} shapes`, "ok");
  });

  document.getElementById("unlock-selection")?.addEventListener("click", () => {
    const ids = selectedShapeIds();
    if (!ids.length) {
      setCompileStatus("Select shapes to unlock", "err");
      return;
    }
    const next = unlockShapes(currentDescription, ids);
    commitDescription(next, { pushHistory: true, label: "Unlock selection", translate: true, updateEditor: true });
    setCompileStatus(`Unlocked ${ids.length} shapes`, "ok");
  });

  document.getElementById("isolate-selection")?.addEventListener("click", () => {
    const ids = selectedShapeIds();
    if (!ids.length) {
      setCompileStatus("Select shapes to isolate", "err");
      return;
    }
    const next = isolateShapes(currentDescription, ids);
    commitDescription(next, { pushHistory: true, label: "Isolate selection", translate: true, updateEditor: true });
    setCompileStatus(`Isolated ${ids.length} shapes`, "ok");
  });

  document.getElementById("clear-isolate")?.addEventListener("click", () => {
    const next = clearIsolation(currentDescription);
    commitDescription(next, { pushHistory: true, label: "Clear isolate", translate: true, updateEditor: true });
    setCompileStatus("Isolate cleared", "ok");
  });

  document.getElementById("toggle-transform-space")?.addEventListener("click", () => {
    const next = sceneManager.getTransformSpace() === "local" ? "world" : "local";
    sceneManager.setTransformSpace(next);
    updateTransformSpaceButton();
  });

  document.getElementById("view-iso")?.addEventListener("click", () => sceneManager.setCameraPreset("iso", selectedEntityIds));
  document.getElementById("view-front")?.addEventListener("click", () => sceneManager.setCameraPreset("front", selectedEntityIds));
  document.getElementById("view-top")?.addEventListener("click", () => sceneManager.setCameraPreset("top", selectedEntityIds));
  document.getElementById("view-right")?.addEventListener("click", () => sceneManager.setCameraPreset("right", selectedEntityIds));
  document.getElementById("frame-selected")?.addEventListener("click", () => sceneManager.frameSelected(selectedEntityIds));
  document.getElementById("frame-all")?.addEventListener("click", () => sceneManager.frameAll());

  document.getElementById("display-solid")?.addEventListener("click", () => sceneManager.setDisplayMode("solid"));
  document.getElementById("display-wireframe")?.addEventListener("click", () => sceneManager.setDisplayMode("wireframe"));
  document.getElementById("display-xray")?.addEventListener("click", () => sceneManager.setDisplayMode("xray"));

  document.getElementById("section-toggle")?.addEventListener("click", () => {
    sectionEnabled = !sectionEnabled;
    updateSectionUi();
    applySectionState();
  });
  document.getElementById("section-axis")?.addEventListener("change", () => applySectionState());
  document.getElementById("section-offset")?.addEventListener("input", () => applySectionState());

  document.getElementById("import-geometry")?.addEventListener("click", () => {
    const input = document.getElementById("import-file");
    if (input) input.click();
  });
  document.getElementById("import-file")?.addEventListener("change", async (event) => {
    const input = event.target;
    const file = input?.files?.[0];
    if (!file) return;
    await importGeometryFile(file);
    input.value = "";
  });

  document.getElementById("export-obj")?.addEventListener("click", () => exportGeometry("obj"));
  document.getElementById("export-stl")?.addEventListener("click", () => exportGeometry("stl"));
  document.getElementById("export-step")?.addEventListener("click", () => exportGeometry("step"));

  document.getElementById("save-checkpoint")?.addEventListener("click", () => saveCheckpoint());
  document.getElementById("restore-checkpoint")?.addEventListener("click", () => restoreCheckpoint());
  document.getElementById("clear-checkpoints")?.addEventListener("click", () => clearCheckpoints());

  window.addEventListener("keydown", (event) => {
    const isEditable = event.target instanceof HTMLElement && ["INPUT", "TEXTAREA"].includes(event.target.tagName);
    if (isEditable && !(event.ctrlKey || event.metaKey)) return;

    if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === "z" && !event.shiftKey) {
      event.preventDefault();
      undoDescription();
      return;
    }
    if ((event.ctrlKey || event.metaKey) && (event.key.toLowerCase() === "y" || (event.key.toLowerCase() === "z" && event.shiftKey))) {
      event.preventDefault();
      redoDescription();
      return;
    }
    if (event.key === "Delete") {
      const ids = selectedShapeIds();
      if (ids.length) {
        const next = hideShapes(currentDescription, ids);
        commitDescription(next, { pushHistory: true, label: "Hide (Delete)", translate: true, updateEditor: true });
        sceneManager.clearSelection(true);
      }
    }
  });

  updateSectionUi();
}

function handleFeatureTreeSelect(id) {
  selectedEntityId = id ? String(id) : null;
  if (!selectedEntityId) {
    sceneManager.clearSelection(true);
    renderDescriptionItemProperties(null);
    return;
  }

  if (sceneManager.hasObject(selectedEntityId)) {
    sceneManager.selectMultiple([selectedEntityId], true, selectedEntityId);
    sceneManager.focusObject(selectedEntityId);
  } else {
    renderDescriptionItemProperties(selectedEntityId);
  }
  dslEditor?.scrollToEntity(selectedEntityId);
  refreshBooleanPairStatus();
  refreshSelectionStatus();
}

function addShape(shape) {
  const next = cloneDescription(currentDescription);
  next.shapes = [...(next.shapes || []), shape];
  commitDescription(next, { pushHistory: true, label: "Add shape", translate: true, updateEditor: true });
}

function moveFeatureItem(id, direction) {
  const next = cloneDescription(currentDescription);
  const moved = moveById(next.shapes, id, direction) || moveById(next.features, id, direction);
  if (!moved) {
    setCompileStatus(`Cannot reorder '${id}'`, "err");
    return;
  }

  commitDescription(next, { pushHistory: true, label: "Reorder feature", translate: true, updateEditor: true });
  setCompileStatus(`Reordered ${id}`, "ok");
}

function setFeatureSuppressed(id, suppressed) {
  const next = cloneDescription(currentDescription);
  const item =
    (next.shapes || []).find((shape) => String(shape.id) === String(id)) ||
    (next.features || []).find((feature) => String(feature.id) === String(id)) ||
    (next.constraints || []).find((constraint) => String(constraint.id) === String(id));

  if (!item) {
    setCompileStatus(`Feature '${id}' not found`, "err");
    return;
  }

  item.suppressed = !!suppressed;
  if (booleanSelection.targetId === id) booleanSelection.targetId = null;
  if (booleanSelection.toolId === id) booleanSelection.toolId = null;

  commitDescription(next, {
    pushHistory: true,
    label: suppressed ? "Suppress feature" : "Unsuppress feature",
    translate: true,
    updateEditor: true,
  });
  setCompileStatus(`${suppressed ? "Suppressed" : "Unsuppressed"} ${id}`, "ok");
}

function markBooleanTargetFromSelection() {
  const shape = getSelectedActiveShape();
  if (!shape) {
    setCompileStatus("Select an active shape to set as boolean target", "err");
    return;
  }

  booleanSelection.targetId = String(shape.id);
  if (booleanSelection.toolId === booleanSelection.targetId) {
    booleanSelection.toolId = null;
  }
  refreshBooleanPairStatus();
  setCompileStatus(`Boolean target: ${booleanSelection.targetId}`, "ok");
}

function markBooleanToolFromSelection() {
  const shape = getSelectedActiveShape();
  if (!shape) {
    setCompileStatus("Select an active shape to set as boolean tool", "err");
    return;
  }

  booleanSelection.toolId = String(shape.id);
  if (booleanSelection.targetId === booleanSelection.toolId) {
    booleanSelection.targetId = null;
  }
  refreshBooleanPairStatus();
  setCompileStatus(`Boolean tool: ${booleanSelection.toolId}`, "ok");
}

function applyBooleanOperationFromSelection(operation) {
  const description = cloneDescription(currentDescription);
  syncBooleanSelectionWithDescription(description);

  const targetId = booleanSelection.targetId;
  const toolId = booleanSelection.toolId;
  if (!targetId || !toolId) {
    setCompileStatus("Set both target and tool before applying boolean", "err");
    return;
  }

  if (targetId === toolId) {
    setCompileStatus("Boolean target and tool must be different", "err");
    return;
  }

  const shapes = Array.isArray(description.shapes) ? description.shapes : [];
  const tool = shapes.find((shape) => String(shape.id) === toolId);
  const target = shapes.find((shape) => String(shape.id) === targetId);
  if (!tool || !target || tool.suppressed || target.suppressed) {
    setCompileStatus("Boolean pair is invalid or suppressed", "err");
    return;
  }

  tool.operation = operation;
  tool.target = targetId;

  commitDescription(description, {
    pushHistory: true,
    label: `Boolean ${operation}`,
    translate: true,
    updateEditor: true,
  });
  refreshBooleanPairStatus();
  setCompileStatus(`Boolean ${operation}: ${tool.id} -> ${targetId}`, "ok");
}

function clearBooleanOperationFromSelection() {
  const description = cloneDescription(currentDescription);
  const toolId = booleanSelection.toolId || selectedEntityId;
  if (!toolId) {
    setCompileStatus("Select or mark a tool object first", "err");
    return;
  }

  const targetShape = (description.shapes || []).find((shape) => String(shape.id) === String(toolId));
  if (!targetShape) {
    setCompileStatus("Selected tool is not in the description", "err");
    return;
  }

  delete targetShape.operation;
  delete targetShape.target;
  commitDescription(description, {
    pushHistory: true,
    label: "Clear boolean",
    translate: true,
    updateEditor: true,
  });
  refreshBooleanPairStatus();
  setCompileStatus(`Cleared boolean operation for ${targetShape.id}`, "ok");
}

function handleTransformFromViewport(entity) {
  if (!entity?.id) return;
  const next = cloneDescription(currentDescription);
  const target = (next.shapes || []).find((shape) => shape.id === entity.id);
  if (!target || target.locked) return;

  const transform = entity.components?.transform?.properties || {};
  const position = unwrapIrValue(transform.position);
  const rotation = unwrapIrValue(transform.rotation);
  const scale = unwrapIrValue(transform.scale);

  if (Array.isArray(position) && position.length === 3) {
    target.position = position.map((value) => Number(value));
  }
  if (Array.isArray(rotation) && rotation.length === 3) {
    target.rotation = rotation.map((value) => Number(value));
  }
  if (Array.isArray(scale) && scale.length === 3) {
    target.scale = scale.map((value) => Number(value));
  }

  commitDescription(next, {
    pushHistory: true,
    label: `Transform ${entity.id}`,
    translate: true,
    updateEditor: true,
  });
}

async function runLlmDescribe(refine) {
  const promptInput = document.getElementById("llm-prompt");
  const prompt = promptInput?.value?.trim();
  if (!prompt) return;

  setCompileStatus(refine ? "Refining..." : "Describing...", "");

  const endpoint = refine ? "/api/llm/refine" : "/api/llm/describe";
  const payload = {
    prompt,
    current_description: currentDescription,
    unit_system: "SI",
    precision: currentDescription.precision || 0.01,
  };

  const result = await api(endpoint, { method: "POST", body: payload });
  if (!result.description) {
    setCompileStatus("LLM request failed", "err");
    return;
  }

  if (Array.isArray(result.changed_fields)) {
    descriptionPanel.highlightFields(result.changed_fields);
  }

  commitDescription(normalizeDescription(result.description), {
    pushHistory: true,
    label: refine ? "LLM refine" : "LLM describe",
    translate: true,
    updateEditor: true,
  });
  setCompileStatus(refine ? "Refinement applied" : "Description applied", "ok");
}

async function runLlmExplain() {
  const result = await api("/api/llm/explain", {
    method: "POST",
    body: { description: currentDescription },
  });
  if (result.explanation) {
    window.alert(result.explanation);
  }
}

function commitDescription(nextDescription, {
  pushHistory = true,
  label = "Edit",
  translate = true,
  updateEditor = true,
  autosave = true,
} = {}) {
  const normalized = normalizeDescription(nextDescription);
  if (pushHistory) {
    history.push(currentDescription, label);
  }

  currentDescription = normalized;
  if (updateEditor) {
    descriptionPanel.setDescription(currentDescription);
  }

  syncBooleanSelectionWithDescription(currentDescription);
  syncViewportStates();
  featureTree.render(buildFeatureTreeItems(currentDescription, currentIr));

  if (translate) {
    debouncedTranslate();
  }
  if (autosave) {
    scheduleAutosave(label);
  }

  refreshBooleanPairStatus();
  refreshSelectionStatus();
}

function debouncedTranslate() {
  if (translateTimer) clearTimeout(translateTimer);
  translateTimer = setTimeout(() => translateDescription(), 480);
}

async function translateDescription() {
  const description = cloneDescription(currentDescription);
  syncBooleanSelectionWithDescription(description);
  refreshBooleanPairStatus();
  if (description?.unit) {
    sceneManager.setMeasurementUnit(String(description.unit));
  }

  const result = await api("/api/description/to_dsl", {
    method: "POST",
    body: { description },
  });

  if (typeof result.dsl === "string") {
    dslEditor.setValue(result.dsl);
  }

  renderCompileErrors(result.errors || []);
  dslEditor.setErrors(result.errors || []);
  renderMeasurements(result.measurements || null);

  if (result.ir) {
    applyIr(result.ir, description?.unit);
    setCompileStatus("Translated + compiled", "ok");
  } else if ((result.errors || []).length) {
    featureTree.render(buildFeatureTreeItems(description, null));
    setCompileStatus("Translation produced errors", "err");
  }

  scheduleAutosave("translate");
}

async function compileCurrentDsl() {
  const source = dslEditor.getValue();
  setCompileStatus("Compiling...", "");

  const result = await api("/api/model/compile", {
    method: "POST",
    body: { dsl_source: source },
  });

  renderCompileErrors(result.errors || []);
  dslEditor.setErrors(result.errors || []);
  renderMeasurements(result.measurements || null);

  if (result.ok && result.ir) {
    applyIr(result.ir);
    setCompileStatus("Compile OK", "ok");
  } else {
    setCompileStatus("Compile failed", "err");
  }

  scheduleAutosave("compile");
}

function applyIr(ir, unitHint = null) {
  currentIr = ir;
  const unitLabel = unitHint || unitFromMetadata(ir?.metadata);
  if (unitLabel) sceneManager.setMeasurementUnit(unitLabel);

  sceneManager.updateScene(ir);
  syncViewportStates();

  if (selectedEntityIds.length) {
    sceneManager.selectMultiple(selectedEntityIds, false, selectedEntityId);
  }

  featureTree.render(buildFeatureTreeItems(currentDescription, ir));

  if (selectedEntityId && !sceneManager.hasObject(selectedEntityId)) {
    renderDescriptionItemProperties(selectedEntityId);
  }
}

function syncViewportStates() {
  sceneManager.setObjectStateMap(collectObjectStateMap(currentDescription));
  updateTransformSpaceButton();
  applySectionState();
}

function renderCompileErrors(errors) {
  compileErrors.innerHTML = "";
  for (const error of errors || []) {
    const item = document.createElement("li");
    const code = error.code || "ERR";
    const location = Number.isInteger(error.line) ? ` (line ${error.line})` : "";
    item.textContent = `${code}: ${error.message || "Unknown error"}${location}`;
    compileErrors.appendChild(item);
  }
}

function renderProperties(entity) {
  properties.innerHTML = "";
  if (!entity) {
    properties.innerHTML = `<div class="subtitle">Select an object in the viewport.</div>`;
    return;
  }

  const descriptionShape = (currentDescription.shapes || []).find((shape) => String(shape.id) === String(entity.id)) || {};
  const transform = entity.components?.transform?.properties || {};
  appendPropertyReadOnly("id", entity.id);
  appendPropertyReadOnly("kind", entity.kind);
  appendPropertyReadOnly("group", descriptionShape.group_id || "-");
  appendPropertyReadOnly("hidden", String(!!descriptionShape.hidden));
  appendPropertyReadOnly("locked", String(!!descriptionShape.locked));

  appendVectorEditor("position", toFixedVector(unwrapIrValue(transform.position) || [0, 0, 0]), (next) =>
    updateShapeTransform(entity.id, "position", next),
  );
  appendVectorEditor("rotation", toFixedVector(unwrapIrValue(transform.rotation) || [0, 0, 0]), (next) =>
    updateShapeTransform(entity.id, "rotation", next),
  );
  appendVectorEditor("scale", toFixedVector(unwrapIrValue(transform.scale) || [1, 1, 1]), (next) =>
    updateShapeTransform(entity.id, "scale", next),
  );
}

function renderDescriptionItemProperties(id) {
  const description = currentDescription;
  properties.innerHTML = "";
  if (!id) {
    properties.innerHTML = `<div class="subtitle">Select an object or feature.</div>`;
    return;
  }

  const shape = (description.shapes || []).find((item) => String(item.id) === String(id));
  if (shape) {
    appendPropertyReadOnly("id", shape.id);
    appendPropertyReadOnly("type", shape.type || "shape");
    appendPropertyReadOnly("group", shape.group_id || "-");
    appendPropertyReadOnly("hidden", String(!!shape.hidden));
    appendPropertyReadOnly("locked", String(!!shape.locked));
    appendPropertyReadOnly("suppressed", String(!!shape.suppressed));
    appendPropertyReadOnly("boolean", shape.operation ? `${shape.operation} -> ${shape.target || "-"}` : "none");
    return;
  }

  const feature = (description.features || []).find((item) => String(item.id) === String(id));
  if (feature) {
    appendPropertyReadOnly("id", feature.id || id);
    appendPropertyReadOnly("type", feature.type || "feature");
    appendPropertyReadOnly("suppressed", String(!!feature.suppressed));
    properties.innerHTML += `<div class="subtitle" style="grid-column: 1 / -1;">Feature is non-solid and edited via description JSON.</div>`;
    return;
  }

  properties.innerHTML = `<div class="subtitle">Selected item is not available in the current description.</div>`;
}

function appendPropertyReadOnly(label, value) {
  const key = document.createElement("div");
  key.className = "subtitle";
  key.textContent = label;
  const val = document.createElement("div");
  val.className = "mono";
  val.textContent = String(value);
  properties.appendChild(key);
  properties.appendChild(val);
}

function appendVectorEditor(label, values, onCommit) {
  const key = document.createElement("div");
  key.className = "subtitle";
  key.textContent = label;
  const editor = document.createElement("div");
  editor.style.display = "flex";
  editor.style.gap = "6px";

  const axes = ["x", "y", "z"];
  const inputs = axes.map((axis, index) => {
    const input = document.createElement("input");
    input.type = "number";
    input.step = String(Math.max(Number(currentDescription.precision || 0.01), 0.0001));
    input.value = Number(values[index] ?? 0).toString();
    input.title = axis;
    input.className = "mono";
    input.style.width = "100%";
    input.style.padding = "4px 6px";
    input.style.border = "1px solid rgba(76, 64, 45, 0.28)";
    input.style.borderRadius = "6px";
    input.style.background = "rgba(255,255,255,0.92)";
    input.addEventListener("change", () => {
      const next = inputs.map((node) => Number(node.value));
      if (next.some((value) => Number.isNaN(value))) return;
      onCommit(next);
    });
    return input;
  });
  inputs.forEach((input) => editor.appendChild(input));

  properties.appendChild(key);
  properties.appendChild(editor);
}

function updateShapeTransform(entityId, field, values) {
  const next = cloneDescription(currentDescription);
  const shape = (next.shapes || []).find((item) => item.id === entityId);
  if (!shape || shape.locked) return;
  shape[field] = values.map((value) => Number(value));
  commitDescription(next, { pushHistory: true, label: `Inspector ${field}`, translate: true, updateEditor: true });
}

function undoDescription() {
  const previous = history.undo(currentDescription);
  if (!previous) {
    setCompileStatus("Nothing to undo", "err");
    return;
  }
  commitDescription(previous, { pushHistory: false, label: "Undo", translate: true, updateEditor: true });
  setCompileStatus("Undo applied", "ok");
}

function redoDescription() {
  const next = history.redo(currentDescription);
  if (!next) {
    setCompileStatus("Nothing to redo", "err");
    return;
  }
  commitDescription(next, { pushHistory: false, label: "Redo", translate: true, updateEditor: true });
  setCompileStatus("Redo applied", "ok");
}

function refreshBooleanPairStatus() {
  if (!booleanPairStatus) return;
  syncBooleanSelectionWithDescription(currentDescription);

  const targetId = booleanSelection.targetId;
  const toolId = booleanSelection.toolId;
  booleanPairStatus.classList.remove("ok", "err", "pending", "warn");
  if (targetId && toolId) {
    booleanPairStatus.classList.add("ok");
  } else if (targetId || toolId) {
    booleanPairStatus.classList.add("warn");
  } else {
    booleanPairStatus.classList.add("pending");
  }
  booleanPairStatus.textContent = `Target: ${targetId || "-"} / Tool: ${toolId || "-"}`;
}

function refreshSelectionStatus() {
  if (!selectionStatus) return;
  const count = selectedEntityIds.length;
  selectionStatus.classList.remove("ok", "err", "pending", "warn");
  if (count === 0) selectionStatus.classList.add("pending");
  else if (count === 1) selectionStatus.classList.add("ok");
  else selectionStatus.classList.add("warn");
  selectionStatus.textContent = `Selection: ${count}`;
}

function syncBooleanSelectionWithDescription(description) {
  const availableShapeIds = new Set(
    (Array.isArray(description?.shapes) ? description.shapes : [])
      .filter((shape) => !shape?.suppressed && shape?.id)
      .map((shape) => String(shape.id)),
  );

  if (!availableShapeIds.has(String(booleanSelection.targetId || ""))) {
    booleanSelection.targetId = null;
  }
  if (!availableShapeIds.has(String(booleanSelection.toolId || ""))) {
    booleanSelection.toolId = null;
  }
  if (booleanSelection.targetId && booleanSelection.targetId === booleanSelection.toolId) {
    booleanSelection.toolId = null;
  }
}

function selectedShapeIds() {
  const set = new Set(selectedEntityIds.map((id) => String(id)));
  return (currentDescription.shapes || [])
    .filter((shape) => set.has(String(shape.id)) && !shape.suppressed)
    .map((shape) => String(shape.id));
}

function getSelectedActiveShape() {
  const shapes = selectedShapes(currentDescription, selectedEntityIds);
  const active = shapes.find((shape) => !shape.suppressed);
  if (active) return active;
  if (!selectedEntityId) return null;
  return (currentDescription.shapes || []).find(
    (shape) => String(shape.id) === String(selectedEntityId) && !shape.suppressed,
  );
}

function buildFeatureTreeItems(description, ir) {
  const items = [];
  for (const shape of Array.isArray(description?.shapes) ? description.shapes : []) {
    const tags = [];
    if (shape.group_id) tags.push(`group:${shape.group_id}`);
    if (shape.hidden) tags.push("hidden");
    if (shape.locked) tags.push("locked");
    const op = shape.operation ? `boolean_${shape.operation}` : shape.type || "shape";
    items.push({
      id: String(shape.id),
      operation_type: `${op}${tags.length ? ` • ${tags.join(" • ")}` : ""}`,
      suppressed: !!shape.suppressed,
      read_only: false,
    });
  }

  for (const feature of Array.isArray(description?.features) ? description.features : []) {
    items.push({
      id: String(feature.id || feature.type || "feature"),
      operation_type: String(feature.type || "feature"),
      suppressed: !!feature.suppressed,
      read_only: false,
    });
  }

  const known = new Set(items.map((item) => item.id));
  const irFeatures =
    Array.isArray(ir?.modeling_tree) && ir.modeling_tree.length
      ? ir.modeling_tree
      : Array.isArray(ir?.entities)
        ? ir.entities.map((entity) => ({ id: entity.id, operation_type: entity.kind || "entity" }))
        : [];

  if (!items.length) {
    return irFeatures.map((feature) => ({
      id: String(feature.id || feature.name || "item"),
      operation_type: String(feature.operation_type || feature.kind || "entity"),
      suppressed: false,
      read_only: true,
    }));
  }

  for (const feature of irFeatures) {
    const id = String(feature.id || feature.name || "item");
    if (known.has(id)) continue;
    items.push({
      id,
      operation_type: String(feature.operation_type || feature.kind || "entity"),
      suppressed: false,
      read_only: true,
    });
  }

  return items;
}

function moveById(list, id, direction) {
  if (!Array.isArray(list)) return false;
  const index = list.findIndex((item) => String(item.id) === String(id));
  if (index < 0) return false;
  const nextIndex = index + Number(direction);
  if (nextIndex < 0 || nextIndex >= list.length) return false;
  const [item] = list.splice(index, 1);
  list.splice(nextIndex, 0, item);
  return true;
}

function renderMeasurements(measurements) {
  if (!measurementsPanel) return;
  if (!measurements || typeof measurements !== "object") {
    measurementsPanel.textContent = "No measurements yet.";
    return;
  }

  const lines = [];
  const bounds = measurements.bounding_box;
  if (bounds?.min && bounds?.max) {
    lines.push(`Bounds min: [${formatVector(bounds.min)}]`);
    lines.push(`Bounds max: [${formatVector(bounds.max)}]`);
  }

  const volumes = Object.entries(measurements.entity_volumes || {})
    .filter(([, value]) => Number(value) > 0)
    .sort((a, b) => Number(b[1]) - Number(a[1]))
    .slice(0, 8);
  if (volumes.length) {
    lines.push("Top volumes:");
    for (const [id, value] of volumes) {
      lines.push(`- ${id}: ${Number(value).toFixed(3)}`);
    }
  }

  const nearestPairs = Array.isArray(measurements.entity_distances)
    ? [...measurements.entity_distances]
        .sort((a, b) => Number(a.distance || 0) - Number(b.distance || 0))
        .slice(0, 4)
    : [];
  if (nearestPairs.length) {
    lines.push("Nearest pairs:");
    for (const pair of nearestPairs) {
      lines.push(
        `- ${pair.entity_a} <> ${pair.entity_b}: d=${Number(pair.distance || 0).toFixed(3)}, angle=${Number(pair.angle || 0).toFixed(2)}deg`,
      );
    }
  }

  measurementsPanel.textContent = lines.length ? lines.join("\n") : "No measurements yet.";
}

function formatVector(value) {
  if (!Array.isArray(value)) return "0, 0, 0";
  return value.slice(0, 3).map((item) => Number(item || 0).toFixed(3)).join(", ");
}

function updateTransformSpaceButton() {
  const button = document.getElementById("toggle-transform-space");
  if (!button) return;
  button.textContent = `Space: ${sceneManager.getTransformSpace() === "world" ? "World" : "Local"}`;
}

function updateSectionUi() {
  const button = document.getElementById("section-toggle");
  if (!button) return;
  button.textContent = `Section: ${sectionEnabled ? "On" : "Off"}`;
}

function applySectionState() {
  const axis = document.getElementById("section-axis")?.value || "y";
  const offset = Number(document.getElementById("section-offset")?.value || 0);
  sceneManager.setSectionState({ enabled: sectionEnabled, axis, offset });
}

async function refreshModelList(preferred = null) {
  if (!modelListSelect) return;
  const result = await api("/api/model/list");
  const models = Array.isArray(result.models) ? result.models : [];

  modelListSelect.innerHTML = "";
  if (!models.length) {
    const option = document.createElement("option");
    option.value = "";
    option.textContent = "No saved models";
    modelListSelect.appendChild(option);
    modelListSelect.disabled = true;
    return;
  }

  modelListSelect.disabled = false;
  for (const modelName of models) {
    const option = document.createElement("option");
    option.value = modelName;
    option.textContent = modelName;
    modelListSelect.appendChild(option);
  }

  if (preferred && models.includes(preferred)) {
    modelListSelect.value = preferred;
  }
}

async function loadSelectedModel() {
  if (!modelListSelect) return;
  const selected = String(modelListSelect.value || "");
  if (!selected) {
    setCompileStatus("Select a model to load", "err");
    return;
  }

  const source = await apiText(`/api/model/load/${encodeURIComponent(selected)}`);
  if (source == null) {
    setCompileStatus(`Failed to load ${selected}`, "err");
    return;
  }

  history.push(currentDescription, "Load model");
  dslEditor.setValue(source);
  await compileCurrentDsl();
  setCompileStatus(`Loaded ${selected}`, "ok");
}

async function exportGeometry(format) {
  const selected = selectedShapeIds();
  const ids = selected.length ? selected : sceneManager.getVisibleObjectIds();
  if (!ids.length) {
    setCompileStatus("No visible geometry to export", "err");
    return;
  }

  if (format === "step") {
    const subset = cloneDescription(currentDescription);
    if (selected.length) {
      const idSet = new Set(selected);
      subset.shapes = (subset.shapes || []).filter((shape) => idSet.has(String(shape.id)));
    }
    const text = createStepLikeExport(subset);
    downloadBlob(`${subset.name || "model"}.step`, text, "text/plain;charset=utf-8");
    setCompileStatus("STEP export complete", "ok");
    return;
  }

  const result = await sceneManager.exportMeshes(format, ids);
  if (!result?.data) {
    setCompileStatus(`Export ${format.toUpperCase()} failed`, "err");
    return;
  }

  const name = `${currentDescription.name || "model"}.${result.format}`;
  downloadBlob(name, result.data, result.mime || "text/plain");
  setCompileStatus(`${format.toUpperCase()} export complete`, "ok");
}

async function importGeometryFile(file) {
  const name = String(file.name || "import");
  const lower = name.toLowerCase();

  if (lower.endsWith(".obj")) {
    const text = await file.text();
    await importObjText(text, name);
    return;
  }
  if (lower.endsWith(".stl")) {
    const buffer = await file.arrayBuffer();
    await importStlBuffer(buffer, name);
    return;
  }
  if (lower.endsWith(".step") || lower.endsWith(".stp")) {
    const text = await file.text();
    importStepText(text, name);
    return;
  }
  setCompileStatus("Unsupported format. Use OBJ, STL, or STEP", "err");
}

async function importObjText(text, fileName) {
  try {
    const module = await import("https://esm.sh/three@0.160.1/examples/jsm/loaders/OBJLoader.js");
    const loader = new module.OBJLoader();
    const root = loader.parse(text);
    const shapes = importedShapesFromObject(root, fileName, "obj");
    if (!shapes.length) {
      setCompileStatus("OBJ import produced no mesh", "err");
      return;
    }
    const next = cloneDescription(currentDescription);
    next.shapes = [...(next.shapes || []), ...dedupeImportedShapes(next.shapes || [], shapes)];
    commitDescription(next, { pushHistory: true, label: "Import OBJ", translate: true, updateEditor: true });
    setCompileStatus(`Imported ${shapes.length} OBJ mesh(es)`, "ok");
  } catch (error) {
    setCompileStatus(`OBJ import failed: ${error.message || error}`, "err");
  }
}

async function importStlBuffer(buffer, fileName) {
  try {
    const module = await import("https://esm.sh/three@0.160.1/examples/jsm/loaders/STLLoader.js");
    const loader = new module.STLLoader();
    const geometry = loader.parse(buffer);
    geometry.computeBoundingBox();
    const box = geometry.boundingBox;
    if (!box || box.isEmpty()) {
      setCompileStatus("STL import produced empty geometry", "err");
      return;
    }
    const size = box.getSize(new THREE.Vector3());
    const center = box.getCenter(new THREE.Vector3());
    const baseId = normalizeIdentifier(fileName.replace(/\.[^.]+$/, ""));
    const nextShape = {
      id: uniqueShapeId(currentDescription.shapes || [], `${baseId}_stl`),
      type: "box",
      label: `${fileName} (STL bbox)`,
      dimensions: { width: clampDimension(size.x), height: clampDimension(size.y), depth: clampDimension(size.z) },
      position: [center.x, center.y, center.z],
      rotation: [0, 0, 0],
      scale: [1, 1, 1],
      material: "steel",
      source_file: fileName,
      import_format: "stl",
    };

    const next = cloneDescription(currentDescription);
    next.shapes = [...(next.shapes || []), nextShape];
    commitDescription(next, { pushHistory: true, label: "Import STL", translate: true, updateEditor: true });
    setCompileStatus("Imported STL as parametric bounding box", "ok");
  } catch (error) {
    setCompileStatus(`STL import failed: ${error.message || error}`, "err");
  }
}

function importStepText(text, fileName) {
  const parsed = parseStepLikeImport(text, fileName);
  if (!parsed.length) {
    setCompileStatus("STEP import unsupported for generic B-Rep. Use STEP exported by this tool or OBJ/STL.", "err");
    return;
  }

  const next = cloneDescription(currentDescription);
  next.shapes = [...(next.shapes || []), ...dedupeImportedShapes(next.shapes || [], parsed)];
  commitDescription(next, { pushHistory: true, label: "Import STEP", translate: true, updateEditor: true });
  setCompileStatus(`Imported ${parsed.length} STEP shape(s)`, "ok");
}

function parseStepLikeImport(text, fileName) {
  const lines = String(text || "").split(/\r?\n/);
  const shapes = [];
  for (let i = 0; i < lines.length; i += 1) {
    const line = lines[i];
    const match = line.match(/SHAPE\s+([^\s]+)\s+TYPE=([^\s*]+)/i);
    if (!match) continue;
    const id = normalizeIdentifier(match[1]);
    const type = normalizeIdentifier(match[2]);
    const details = lines[i + 1] || "";

    const posMatch = details.match(/POS=\(([-0-9.+eE]+),([-0-9.+eE]+),([-0-9.+eE]+)\)/);
    const dimsMatch = details.match(/DIMS=(\{.*\})/);

    let dimensions = { width: 10, height: 10, depth: 10 };
    if (dimsMatch) {
      try {
        const raw = JSON.parse(dimsMatch[1]);
        if (raw && typeof raw === "object") dimensions = raw;
      } catch {
        // keep fallback dimensions
      }
    }

    const position = posMatch
      ? [Number(posMatch[1]), Number(posMatch[2]), Number(posMatch[3])]
      : [0, 0, 0];

    shapes.push({
      id: uniqueShapeId(shapes, `${normalizeIdentifier(fileName)}_${id}`),
      type: normalizeImportedType(type),
      label: `${fileName} (${type})`,
      dimensions,
      position,
      rotation: [0, 0, 0],
      scale: [1, 1, 1],
      material: "steel",
      source_file: fileName,
      import_format: "step",
    });
  }
  return shapes;
}

function importedShapesFromObject(root, fileName, format) {
  const shapes = [];
  let index = 1;
  root.traverse((node) => {
    if (!node?.isMesh) return;
    const box = new THREE.Box3().setFromObject(node);
    if (box.isEmpty()) return;
    const size = box.getSize(new THREE.Vector3());
    const center = box.getCenter(new THREE.Vector3());
    shapes.push({
      id: `${normalizeIdentifier(fileName.replace(/\.[^.]+$/, ""))}_${format}_${index}`,
      type: "box",
      label: `${fileName} mesh ${index}`,
      dimensions: {
        width: clampDimension(size.x),
        height: clampDimension(size.y),
        depth: clampDimension(size.z),
      },
      position: [center.x, center.y, center.z],
      rotation: [0, 0, 0],
      scale: [1, 1, 1],
      material: "steel",
      source_file: fileName,
      import_format: format,
    });
    index += 1;
  });
  return shapes;
}

function dedupeImportedShapes(existingShapes, imported) {
  const ids = new Set((existingShapes || []).map((shape) => String(shape.id)));
  return imported.map((shape) => {
    const next = { ...shape };
    next.id = uniqueShapeId([...ids].map((id) => ({ id })), next.id);
    ids.add(next.id);
    return next;
  });
}

function uniqueShapeId(shapes, base) {
  const normalized = normalizeIdentifier(base || "shape");
  const used = new Set((shapes || []).map((shape) => String(shape.id)));
  if (!used.has(normalized)) return normalized;
  let i = 2;
  while (used.has(`${normalized}_${i}`)) i += 1;
  return `${normalized}_${i}`;
}

function normalizeIdentifier(raw) {
  const text = String(raw || "item")
    .trim()
    .toLowerCase()
    .replace(/[^a-z0-9_]+/g, "_")
    .replace(/_+/g, "_")
    .replace(/^_+|_+$/g, "");
  if (!text) return "item";
  if (/^[0-9]/.test(text)) return `_${text}`;
  return text;
}

function normalizeImportedType(type) {
  const value = String(type || "box").toLowerCase();
  if (["box", "sphere", "cylinder", "cone", "torus", "plane"].includes(value)) return value;
  return "box";
}

function clampDimension(value) {
  const number = Number(value);
  if (!Number.isFinite(number)) return 0.001;
  return Math.max(number, 0.001);
}

function downloadBlob(filename, payload, mimeType) {
  const blob = payload instanceof Blob ? payload : new Blob([payload], { type: mimeType || "text/plain" });
  const url = URL.createObjectURL(blob);
  const anchor = document.createElement("a");
  anchor.href = url;
  anchor.download = filename;
  document.body.appendChild(anchor);
  anchor.click();
  anchor.remove();
  URL.revokeObjectURL(url);
}

function scheduleAutosave(reason = "edit") {
  if (autosaveTimer) clearTimeout(autosaveTimer);
  autosaveTimer = setTimeout(() => persistAutosave(reason), 650);
}

function persistAutosave(reason = "edit") {
  const envelope = {
    saved_at: new Date().toISOString(),
    reason,
    description: currentDescription,
    dsl: dslEditor?.getValue?.() || "",
  };
  try {
    localStorage.setItem(storage.autosave, JSON.stringify(envelope));
    if (autosaveStatus) autosaveStatus.textContent = `Autosave: ${reason} @ ${new Date().toLocaleTimeString()}`;
  } catch {
    if (autosaveStatus) autosaveStatus.textContent = "Autosave: failed (storage unavailable)";
  }
}

function restoreAutosaveIfAvailable() {
  const raw = localStorage.getItem(storage.autosave);
  const envelope = parseAutosaveEnvelope(raw);
  if (!envelope) {
    if (autosaveStatus) autosaveStatus.textContent = "Autosave: idle";
    return;
  }

  const savedAt = String(envelope.saved_at || "");
  const ask = window.confirm(`Restore autosave from ${savedAt || "previous session"}?`);
  if (!ask) {
    if (autosaveStatus) autosaveStatus.textContent = "Autosave: available but skipped";
    return;
  }

  currentDescription = normalizeDescription(envelope.description);
  descriptionPanel.setDescription(currentDescription);
  if (typeof envelope.dsl === "string" && dslEditor) {
    dslEditor.setValue(envelope.dsl);
  }
  if (autosaveStatus) autosaveStatus.textContent = `Autosave: restored ${savedAt || ""}`;
}

function loadCheckpointsFromStorage() {
  try {
    const raw = localStorage.getItem(storage.checkpoints);
    const parsed = JSON.parse(raw || "[]");
    if (Array.isArray(parsed)) {
      checkpoints = parsed.filter((item) => item && typeof item === "object" && item.description);
      return;
    }
  } catch {
    // ignore malformed checkpoints
  }
  checkpoints = [];
}

function persistCheckpoints() {
  try {
    localStorage.setItem(storage.checkpoints, JSON.stringify(checkpoints));
  } catch {
    // ignore storage failures
  }
}

function refreshCheckpointList(selectedId = null) {
  if (!checkpointList) return;
  checkpointList.innerHTML = "";
  if (!checkpoints.length) {
    const option = document.createElement("option");
    option.value = "";
    option.textContent = "No checkpoints";
    checkpointList.appendChild(option);
    checkpointList.disabled = true;
    return;
  }

  checkpointList.disabled = false;
  for (const checkpoint of checkpoints) {
    const option = document.createElement("option");
    option.value = checkpoint.id;
    option.textContent = `${checkpoint.label} (${new Date(checkpoint.saved_at).toLocaleString()})`;
    checkpointList.appendChild(option);
  }

  if (selectedId) checkpointList.value = selectedId;
}

function saveCheckpoint() {
  const label = window.prompt("Checkpoint label", "Manual checkpoint") || "Manual checkpoint";
  const checkpoint = {
    id: `cp_${Date.now()}`,
    label,
    saved_at: new Date().toISOString(),
    description: cloneDescription(currentDescription),
    dsl: dslEditor?.getValue?.() || "",
  };
  checkpoints = [checkpoint, ...checkpoints].slice(0, 30);
  persistCheckpoints();
  refreshCheckpointList(checkpoint.id);
  setCompileStatus(`Saved checkpoint: ${label}`, "ok");
}

function restoreCheckpoint() {
  const id = String(checkpointList?.value || "");
  if (!id) {
    setCompileStatus("Select checkpoint to restore", "err");
    return;
  }
  const checkpoint = checkpoints.find((item) => item.id === id);
  if (!checkpoint) {
    setCompileStatus("Checkpoint not found", "err");
    return;
  }

  history.push(currentDescription, "Before checkpoint restore");
  commitDescription(checkpoint.description, {
    pushHistory: false,
    label: "Restore checkpoint",
    translate: true,
    updateEditor: true,
    autosave: true,
  });
  if (typeof checkpoint.dsl === "string" && dslEditor) {
    dslEditor.setValue(checkpoint.dsl);
  }
  setCompileStatus(`Restored checkpoint: ${checkpoint.label}`, "ok");
}

function clearCheckpoints() {
  checkpoints = [];
  persistCheckpoints();
  refreshCheckpointList();
  setCompileStatus("Cleared checkpoints", "ok");
}

function unitFromMetadata(metadata) {
  const unitSystem = String(metadata?.unit_system || "").toLowerCase();
  if (unitSystem === "imperial") return "in";
  if (unitSystem === "si") return "m";
  return null;
}

function setCompileStatus(text, state) {
  compileStatus.textContent = text;
  compileStatus.classList.remove("ok", "err", "warn");
  if (state) compileStatus.classList.add(state);
}

function toFixedVector(value) {
  if (!Array.isArray(value) || value.length < 3) return [0, 0, 0];
  return value.slice(0, 3).map((item) => Number(item));
}

function initWebSocket() {
  const protocol = window.location.protocol === "https:" ? "wss" : "ws";
  const socket = new WebSocket(`${protocol}://${window.location.host}/ws`);

  socket.addEventListener("open", () => {
    wsStatus.textContent = "WS Online";
    wsStatus.classList.add("ok");
  });

  socket.addEventListener("message", (event) => {
    try {
      const payload = JSON.parse(event.data);
      if (payload && payload.entities) {
        applyIr(payload);
      }
    } catch {
      // Ignore malformed websocket payloads.
    }
  });

  socket.addEventListener("close", () => {
    wsStatus.textContent = "WS Offline";
    wsStatus.classList.remove("ok");
    setTimeout(initWebSocket, 1400);
  });
}

async function api(path, { method = "GET", body } = {}) {
  const response = await fetch(path, {
    method,
    headers: body ? { "Content-Type": "application/json" } : undefined,
    body: body ? JSON.stringify(body) : undefined,
  });

  let payload = {};
  try {
    payload = await response.json();
  } catch {
    payload = {};
  }

  if (!response.ok) {
    return { ok: false, ...payload };
  }
  return payload;
}

async function apiText(path) {
  const response = await fetch(path);
  if (!response.ok) {
    return null;
  }
  return response.text();
}

function unwrapIrValue(value) {
  if (!value || typeof value !== "object" || Array.isArray(value)) return value;
  const keys = Object.keys(value);
  if (keys.length !== 1) return value;
  const tag = keys[0];
  if (["Number", "String", "Identifier", "Boolean", "Vector3", "Matrix3"].includes(tag)) {
    return value[tag];
  }
  if (tag === "List" && Array.isArray(value[tag])) {
    return value[tag].map((item) => unwrapIrValue(item));
  }
  return value;
}
