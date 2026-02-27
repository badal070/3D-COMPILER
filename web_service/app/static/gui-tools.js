export function createGuiTools({
  sceneManager,
  onAddShape,
  onToggleSnap,
  onToggleMeasure,
  onToggleDrag,
  onSetTransformMode,
} = {}) {
  const snapButton = document.getElementById("toggle-snap");
  const measureButton = document.getElementById("toggle-measure");
  const dragButton = document.getElementById("toggle-drag");
  const modeButtons = {
    translate: document.getElementById("mode-translate"),
    rotate: document.getElementById("mode-rotate"),
    scale: document.getElementById("mode-scale"),
  };
  const shapeButtons = [...document.querySelectorAll("[data-shape]")];

  let snapEnabled = true;
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
  });

  setActiveMode(transformMode);
  setDragEnabled(false);
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
