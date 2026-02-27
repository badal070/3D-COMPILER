export function createDescriptionPanel(container, { onChange } = {}) {
  const textarea = document.createElement("textarea");
  textarea.spellcheck = false;
  textarea.style.width = "100%";
  textarea.style.height = "220px";
  textarea.className = "mono";
  container.appendChild(textarea);

  let description = defaultDescription();
  let changeTimer = null;

  function setDescription(nextDescription) {
    description = structuredClone(nextDescription || defaultDescription());
    textarea.value = JSON.stringify(description, null, 2);
  }

  function getDescription() {
    return structuredClone(description);
  }

  function highlightFields(paths) {
    if (!paths?.length) return;
    textarea.style.borderColor = "rgba(204, 84, 39, 0.5)";
    textarea.title = `Changed: ${paths.slice(0, 6).join(", ")}${paths.length > 6 ? "..." : ""}`;
    setTimeout(() => {
      textarea.style.borderColor = "";
      textarea.title = "";
    }, 1500);
  }

  textarea.addEventListener("input", () => {
    if (changeTimer) clearTimeout(changeTimer);
    changeTimer = setTimeout(() => {
      try {
        const parsed = JSON.parse(textarea.value);
        description = parsed;
        onChange?.(structuredClone(description));
      } catch {
        // Keep editor lenient while typing; errors surface on translate.
      }
    }, 300);
  });

  setDescription(description);
  return { setDescription, getDescription, highlightFields, textarea };
}

function defaultDescription() {
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
