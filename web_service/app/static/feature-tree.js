export function createFeatureTree(container, { onSelect, onMove, onToggleSuppress } = {}) {
  let selectedId = null;
  let featuresCache = [];

  function render(features) {
    featuresCache = Array.isArray(features) ? features : [];
    container.innerHTML = "";
    if (!featuresCache.length) {
      const empty = document.createElement("div");
      empty.className = "subtitle";
      empty.textContent = "No features yet.";
      container.appendChild(empty);
      return;
    }

    featuresCache.forEach((feature, index) => {
      const id = String(feature.id || feature.name || "item");
      const op = String(feature.operation_type || feature.kind || "entity");
      const row = document.createElement("div");
      row.className = `feature-row${feature.suppressed ? " suppressed" : ""}`;
      row.dataset.id = id;

      const shell = document.createElement("div");
      shell.className = `feature-item-shell${id === selectedId ? " active" : ""}`;

      const itemButton = document.createElement("button");
      itemButton.type = "button";
      itemButton.className = "feature-item";
      itemButton.dataset.id = id;
      itemButton.innerHTML = `<strong>${id}</strong><br><span class="subtitle mono">${op}</span>${feature.suppressed ? '<span class="feature-item-state">suppressed</span>' : ""}`;
      itemButton.addEventListener("click", () => {
        selectedId = id;
        updateActiveState();
        onSelect?.(id);
      });
      shell.appendChild(itemButton);

      const controls = document.createElement("div");
      controls.className = "feature-controls";

      const upButton = document.createElement("button");
      upButton.type = "button";
      upButton.textContent = "Up";
      upButton.disabled = !!feature.read_only || index === 0;
      upButton.addEventListener("click", (event) => {
        event.stopPropagation();
        onMove?.(id, -1);
      });
      controls.appendChild(upButton);

      const downButton = document.createElement("button");
      downButton.type = "button";
      downButton.textContent = "Down";
      downButton.disabled = !!feature.read_only || index === featuresCache.length - 1;
      downButton.addEventListener("click", (event) => {
        event.stopPropagation();
        onMove?.(id, 1);
      });
      controls.appendChild(downButton);

      const suppressButton = document.createElement("button");
      suppressButton.type = "button";
      suppressButton.textContent = feature.suppressed ? "Unsuppress" : "Suppress";
      suppressButton.disabled = !!feature.read_only;
      suppressButton.addEventListener("click", (event) => {
        event.stopPropagation();
        onToggleSuppress?.(id, !feature.suppressed);
      });
      controls.appendChild(suppressButton);

      row.appendChild(shell);
      row.appendChild(controls);
      container.appendChild(row);
    });
  }

  function updateActiveState() {
    for (const node of container.querySelectorAll(".feature-item-shell")) {
      node.classList.toggle("active", node.parentElement?.dataset.id === selectedId);
    }
  }

  function setActive(id) {
    selectedId = id ? String(id) : null;
    updateActiveState();
  }

  return { render, setActive };
}
