export function createFeatureTree(container, { onSelect } = {}) {
  let selectedId = null;

  function render(features) {
    container.innerHTML = "";
    if (!features?.length) {
      const empty = document.createElement("div");
      empty.className = "subtitle";
      empty.textContent = "No features yet.";
      container.appendChild(empty);
      return;
    }

    for (const feature of features) {
      const id = String(feature.id || feature.name || "item");
      const op = String(feature.operation_type || feature.kind || "entity");
      const el = document.createElement("button");
      el.type = "button";
      el.className = `feature-item${id === selectedId ? " active" : ""}`;
      el.dataset.id = id;
      el.innerHTML = `<strong>${id}</strong><br><span class="subtitle mono">${op}</span>`;
      el.addEventListener("click", () => {
        selectedId = id;
        updateActiveState();
        onSelect?.(id);
      });
      container.appendChild(el);
    }
  }

  function updateActiveState() {
    for (const node of container.querySelectorAll(".feature-item")) {
      node.classList.toggle("active", node.dataset.id === selectedId);
    }
  }

  function setActive(id) {
    selectedId = id ? String(id) : null;
    updateActiveState();
  }

  return { render, setActive };
}
