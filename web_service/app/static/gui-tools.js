export function createGuiTools({
  sceneManager,
  onAddShape,
  onToggleSnap,
  onToggleVertexSnap,
  onToggleEdgeSnap,
  onToggleMeasure,
  onToggleDrag,
  onSetTransformMode,
  onMarkBooleanTarget,
  onMarkBooleanTool,
  onApplyBooleanOperation,
  onClearBooleanOperation,
} = {}) {
  const snapButton = document.getElementById("toggle-snap");
  const snapVertexButton = document.getElementById("toggle-snap-vertex");
  const snapEdgeButton = document.getElementById("toggle-snap-edge");
  const measureButton = document.getElementById("toggle-measure");
  const dragButton = document.getElementById("toggle-drag");
  const boolSubtractButton = document.getElementById("bool-subtract");
  const boolUnionButton = document.getElementById("bool-union");
  const boolIntersectButton = document.getElementById("bool-intersect");
  const boolClearButton = document.getElementById("bool-clear");
  const boolSetTargetButton = document.getElementById("bool-set-target");
  const boolSetToolButton = document.getElementById("bool-set-tool");
  const modeButtons = {
    translate: document.getElementById("mode-translate"),
    rotate: document.getElementById("mode-rotate"),
    scale: document.getElementById("mode-scale"),
  };
  const shapeButtons = [...document.querySelectorAll("[data-shape]")];

  let snapEnabled = true;
  let vertexSnapEnabled = false;
  let edgeSnapEnabled = false;
  let measureEnabled = false;
  let dragEnabled = false;
  let transformMode = "translate";

  function setActiveMode(nextMode) {
    transformMode = nextMode;
    sceneManager?.setTransformMode(nextMode);
    onSetTransformMode?.(nextMode);
    for (const [mode, button] of Object.entries(modeButtons)) {
      if (!button) continue;
      button.classList.toggle("primary", mode === nextMode);
      button.classList.toggle("ghost", mode !== nextMode);
    }
  }

  function setDragEnabled(enabled) {
    dragEnabled = !!enabled;
    sceneManager?.setDragMode(dragEnabled);
    if (dragButton) dragButton.textContent = dragEnabled ? "Drag: On" : "Drag";
    onToggleDrag?.(dragEnabled);
  }

  snapButton?.addEventListener("click", () => {
    snapEnabled = !snapEnabled;
    sceneManager?.setSnapEnabled(snapEnabled);
    snapButton.textContent = `Snap: ${snapEnabled ? "On" : "Off"}`;
    onToggleSnap?.(snapEnabled);
  });
  snapVertexButton?.addEventListener("click", () => {
    vertexSnapEnabled = !vertexSnapEnabled;
    sceneManager?.setVertexSnapEnabled(vertexSnapEnabled);
    snapVertexButton.textContent = `Vertex Snap: ${vertexSnapEnabled ? "On" : "Off"}`;
    onToggleVertexSnap?.(vertexSnapEnabled);
  });
  snapEdgeButton?.addEventListener("click", () => {
    edgeSnapEnabled = !edgeSnapEnabled;
    sceneManager?.setEdgeSnapEnabled(edgeSnapEnabled);
    snapEdgeButton.textContent = `Edge Snap: ${edgeSnapEnabled ? "On" : "Off"}`;
    onToggleEdgeSnap?.(edgeSnapEnabled);
  });

  measureButton?.addEventListener("click", () => {
    measureEnabled = !measureEnabled;
    if (measureEnabled && dragEnabled) setDragEnabled(false);
    sceneManager?.setMeasureMode(measureEnabled);
    measureButton.textContent = measureEnabled ? "Measure: On" : "Measure";
    onToggleMeasure?.(measureEnabled);
  });

  dragButton?.addEventListener("click", () => {
    setDragEnabled(!dragEnabled);
  });

  for (const [mode, button] of Object.entries(modeButtons)) {
    button?.addEventListener("click", () => setActiveMode(mode));
  }

  shapeButtons.forEach((button) => {
    button.addEventListener("click", () => {
      const shapeType = String(button.dataset.shape || "").toLowerCase();
      const shape = buildShape(shapeType);
      if (shape) onAddShape?.(shape);
    });
  });
  boolSubtractButton?.addEventListener("click", () => onApplyBooleanOperation?.("subtract"));
  boolUnionButton?.addEventListener("click", () => onApplyBooleanOperation?.("union"));
  boolIntersectButton?.addEventListener("click", () => onApplyBooleanOperation?.("intersect"));
  boolClearButton?.addEventListener("click", () => onClearBooleanOperation?.());
  boolSetTargetButton?.addEventListener("click", () => onMarkBooleanTarget?.());
  boolSetToolButton?.addEventListener("click", () => onMarkBooleanTool?.());

  window.addEventListener("keydown", (event) => {
    const key = event.key.toLowerCase();
    if (key === "g") {
      snapEnabled = !snapEnabled;
      sceneManager?.setSnapEnabled(snapEnabled);
      if (snapButton) snapButton.textContent = `Snap: ${snapEnabled ? "On" : "Off"}`;
      onToggleSnap?.(snapEnabled);
    }
    if (key === "m") {
      measureEnabled = !measureEnabled;
      if (measureEnabled && dragEnabled) setDragEnabled(false);
      sceneManager?.setMeasureMode(measureEnabled);
      if (measureButton) measureButton.textContent = measureEnabled ? "Measure: On" : "Measure";
      onToggleMeasure?.(measureEnabled);
    }
    if (key === "d") setDragEnabled(!dragEnabled);
    if (key === "w") setActiveMode("translate");
    if (key === "e") setActiveMode("rotate");
    if (key === "r") setActiveMode("scale");
    if (key === "v") {
      vertexSnapEnabled = !vertexSnapEnabled;
      sceneManager?.setVertexSnapEnabled(vertexSnapEnabled);
      if (snapVertexButton) {
        snapVertexButton.textContent = `Vertex Snap: ${vertexSnapEnabled ? "On" : "Off"}`;
      }
      onToggleVertexSnap?.(vertexSnapEnabled);
    }
    if (key === "b") {
      edgeSnapEnabled = !edgeSnapEnabled;
      sceneManager?.setEdgeSnapEnabled(edgeSnapEnabled);
      if (snapEdgeButton) {
        snapEdgeButton.textContent = `Edge Snap: ${edgeSnapEnabled ? "On" : "Off"}`;
      }
      onToggleEdgeSnap?.(edgeSnapEnabled);
    }
  });

  setActiveMode(transformMode);
  setDragEnabled(false);
  sceneManager?.setVertexSnapEnabled(false);
  sceneManager?.setEdgeSnapEnabled(false);
  if (snapVertexButton) snapVertexButton.textContent = "Vertex Snap: Off";
  if (snapEdgeButton) snapEdgeButton.textContent = "Edge Snap: Off";
}

function buildShape(type) {
  const timestamp = Date.now();
  const templates = {
    box: {
      id: `box_${timestamp}`,
      type: "box",
      label: "Box",
      dimensions: { width: 10, height: 10, depth: 10 },
      position: [0, 0, 0],
      material: "steel",
    },
    cylinder: {
      id: `cylinder_${timestamp}`,
      type: "cylinder",
      label: "Cylinder",
      dimensions: { radius: 5, depth: 12 },
      position: [0, 0, 0],
      material: "steel",
    },
    sphere: {
      id: `sphere_${timestamp}`,
      type: "sphere",
      label: "Sphere",
      dimensions: { radius: 6 },
      position: [0, 0, 0],
      material: "steel",
    },
    cone: {
      id: `cone_${timestamp}`,
      type: "cone",
      label: "Cone",
      dimensions: { radius: 5, depth: 12 },
      position: [0, 0, 0],
      material: "steel",
    },
    torus: {
      id: `torus_${timestamp}`,
      type: "torus",
      label: "Torus",
      dimensions: { major_radius: 8, minor_radius: 2 },
      position: [0, 0, 0],
      material: "steel",
    },
  };
  return templates[type] ? structuredClone(templates[type]) : null;
}
