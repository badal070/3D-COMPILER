export async function createDslEditor(container, { onChange, onSave } = {}) {
  try {
    const [{ EditorState }, { EditorView, keymap, lineNumbers }, { defaultKeymap }, { defaultHighlightStyle, syntaxHighlighting }] =
      await Promise.all([
        import("https://esm.sh/@codemirror/state@6.4.1"),
        import("https://esm.sh/@codemirror/view@6.28.6"),
        import("https://esm.sh/@codemirror/commands@6.6.0"),
        import("https://esm.sh/@codemirror/language@6.10.2"),
      ]);

    const saveKey = {
      key: "Mod-s",
      preventDefault: true,
      run: () => {
        onSave?.();
        return true;
      },
    };

    const state = EditorState.create({
      doc: "",
      extensions: [
        lineNumbers(),
        syntaxHighlighting(defaultHighlightStyle),
        keymap.of([...defaultKeymap, saveKey]),
        EditorView.updateListener.of((update) => {
          if (update.docChanged) {
            onChange?.(update.state.doc.toString());
          }
        }),
      ],
    });

    const view = new EditorView({ state, parent: container });

    return {
      setValue(value) {
        const doc = view.state.doc.toString();
        if (doc === value) return;
        view.dispatch({ changes: { from: 0, to: doc.length, insert: value } });
      },
      getValue() {
        return view.state.doc.toString();
      },
      setErrors(errors) {
        const first = errors?.find((error) => Number.isInteger(error.line));
        if (!first) return;
        const line = Math.max(1, first.line);
        const target = view.state.doc.line(Math.min(line, view.state.doc.lines));
        view.dispatch({ selection: { anchor: target.from }, scrollIntoView: true });
      },
      scrollToEntity(entityId) {
        if (!entityId) return;
        const text = view.state.doc.toString();
        const index = text.indexOf(`entity ${entityId}`);
        if (index < 0) return;
        view.dispatch({ selection: { anchor: index }, scrollIntoView: true });
      },
    };
  } catch {
    const textarea = document.createElement("textarea");
    textarea.className = "mono";
    textarea.style.width = "100%";
    textarea.style.height = "240px";
    container.appendChild(textarea);

    textarea.addEventListener("input", () => onChange?.(textarea.value));
    textarea.addEventListener("keydown", (event) => {
      if ((event.metaKey || event.ctrlKey) && event.key.toLowerCase() === "s") {
        event.preventDefault();
        onSave?.();
      }
    });

    return {
      setValue(value) {
        textarea.value = value;
      },
      getValue() {
        return textarea.value;
      },
      setErrors() {},
      scrollToEntity(entityId) {
        const index = textarea.value.indexOf(`entity ${entityId}`);
        if (index < 0) return;
        textarea.focus();
        textarea.setSelectionRange(index, index);
      },
    };
  }
}
