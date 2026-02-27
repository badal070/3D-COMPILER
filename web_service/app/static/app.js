import { SceneManager } from "./scene-manager.js";
import { createFeatureTree } from "./feature-tree.js";
import { createDescriptionPanel } from "./description-panel.js";
import { createDslEditor } from "./dsl-editor.js";
import { createGuiTools } from "./gui-tools.js";

const canvas = document.getElementById("viewport-canvas");
const sceneManager = new SceneManager(canvas);

const featureTree = createFeatureTree(document.getElementById("feature-tree"), {
  onSelect: (id) => {
    sceneManager.focusObject(id);
    dslEditor?.scrollToEntity(id);
  },
});

const descriptionPanel = createDescriptionPanel(document.getElementById("description-editor"), {
  onChange: (description) => {
    currentDescription = description;
    debouncedTranslate();
  },
});

const compileStatus = document.getElementById("compile-status");
const compileErrors = document.getElementById("compile-errors");
const properties = document.getElementById("properties");
const wsStatus = document.getElementById("ws-status");

let currentDescription = descriptionPanel.getDescription();
let currentIr = null;
let dslEditor = null;
let compileTimer = null;
let translateTimer = null;

bootstrap().catch((error) => {
  setCompileStatus(`Init failed: ${error.message || error}`, "err");
});

async function bootstrap() {
  dslEditor = await createDslEditor(document.getElementById("dsl-editor"), {
    onChange: () => {
      if (compileTimer) clearTimeout(compileTimer);
      compileTimer = setTimeout(() => compileCurrentDsl(), 800);
    },
    onSave: () => compileCurrentDsl(),
  });

  sceneManager.setCallbacks({
    onSelect: (entity) => {
      featureTree.setActive(entity?.id || null);
      renderProperties(entity);
    },
    onTransform: handleTransformFromViewport,
  });

  bindUi();
  initWebSocket();
  createGuiTools({
    sceneManager,
    onAddShape: addShape,
  });

  await translateDescription();
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
      setCompileStatus(`Saved ${response.name}`, "ok");
    } else {
      setCompileStatus("Save failed", "err");
    }
  });

  document.getElementById("llm-send")?.addEventListener("click", () => runLlmDescribe(false));
  document.getElementById("llm-refine")?.addEventListener("click", () => runLlmDescribe(true));
  document.getElementById("llm-explain")?.addEventListener("click", runLlmExplain);
}

function addShape(shape) {
  const description = descriptionPanel.getDescription();
  description.shapes = [...(description.shapes || []), shape];
  descriptionPanel.setDescription(description);
  currentDescription = description;
  translateDescription();
}

function handleTransformFromViewport(entity) {
  if (!entity?.id) return;
  const description = descriptionPanel.getDescription();
  const target = (description.shapes || []).find((shape) => shape.id === entity.id);
  if (!target) return;

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

  descriptionPanel.setDescription(description);
  currentDescription = description;
  debouncedTranslate();
}

async function runLlmDescribe(refine) {
  const promptInput = document.getElementById("llm-prompt");
  const prompt = promptInput?.value?.trim();
  if (!prompt) return;

  setCompileStatus(refine ? "Refining..." : "Describing...", "");

  const endpoint = refine ? "/api/llm/refine" : "/api/llm/describe";
  const payload = refine
    ? { prompt, current_description: currentDescription, unit_system: "SI", precision: currentDescription.precision || 0.01 }
    : {
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

  currentDescription = result.description;
  descriptionPanel.setDescription(currentDescription);
  if (Array.isArray(result.changed_fields)) {
    descriptionPanel.highlightFields(result.changed_fields);
  }

  await translateDescription();
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

function debouncedTranslate() {
  if (translateTimer) clearTimeout(translateTimer);
  translateTimer = setTimeout(() => translateDescription(), 550);
}

async function translateDescription() {
  const description = descriptionPanel.getDescription();
  currentDescription = description;

  const result = await api("/api/description/to_dsl", {
    method: "POST",
    body: { description },
  });

  if (typeof result.dsl === "string") {
    dslEditor.setValue(result.dsl);
  }

  renderCompileErrors(result.errors || []);
  dslEditor.setErrors(result.errors || []);

  if (result.ir) {
    applyIr(result.ir);
    setCompileStatus("Translated + compiled", "ok");
  } else if ((result.errors || []).length) {
    setCompileStatus("Translation produced errors", "err");
  }
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

  if (result.ok && result.ir) {
    applyIr(result.ir);
    setCompileStatus("Compile OK", "ok");
    return;
  }

  setCompileStatus("Compile failed", "err");
}

function applyIr(ir) {
  currentIr = ir;
  sceneManager.updateScene(ir);

  const features =
    Array.isArray(ir.modeling_tree) && ir.modeling_tree.length
      ? ir.modeling_tree
      : (ir.entities || []).map((entity) => ({ id: entity.id, operation_type: entity.kind || "entity" }));
  featureTree.render(features);
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

  const rows = [
    ["id", entity.id],
    ["kind", entity.kind],
  ];

  const transform = entity.components?.transform?.properties || {};
  rows.push(["position", JSON.stringify(unwrapIrValue(transform.position) || [0, 0, 0])]);
  rows.push(["rotation", JSON.stringify(unwrapIrValue(transform.rotation) || [0, 0, 0])]);

  for (const [label, value] of rows) {
    const key = document.createElement("div");
    key.className = "subtitle";
    key.textContent = label;
    const val = document.createElement("div");
    val.className = "mono";
    val.textContent = String(value);
    properties.appendChild(key);
    properties.appendChild(val);
  }
}

function setCompileStatus(text, state) {
  compileStatus.textContent = text;
  compileStatus.classList.remove("ok", "err");
  if (state) compileStatus.classList.add(state);
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
