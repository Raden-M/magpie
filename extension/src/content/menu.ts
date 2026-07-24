// The in-page capture UI: a floating pill at the selection (shade picker + Keep
// + Note) and a result toast. Rendered in a Shadow DOM so the host page's CSS
// can never touch it. The visual language echoes the app - neutral grey chrome
// that sits calmly over any page, with the blue shine reserved for actions.

export interface Ui {
  /** Show the pill for a *new* selection: reset shade/note/keep state, position. */
  showPill(rect: DOMRect, shade: string): void;
  /** Reposition the already-shown pill for the same selection, keeping its state. */
  repositionPill(rect: DOMRect): void;
  hidePill(): void;
  /** True while the note input holds focus, so a transient selection loss shouldn't hide it. */
  isEngaged(): boolean;
  showToast(text: string, tone: "ok" | "error"): void;
  setBusy(busy: boolean): void;
}

const SHADES: { id: string; grad: string }[] = [
  { id: "iridescent", grad: "linear-gradient(101deg,#7ad0ff,#52a0ff,#4f7cf7)" },
  { id: "blue", grad: "#5b7bff" },
  { id: "teal", grad: "#2cd8d0" },
  { id: "purple", grad: "#a76bff" },
];

const STYLE = `
:host { all: initial; }
.pill, .toast { font-family: system-ui, -apple-system, "Segoe UI", Roboto, sans-serif; }
.pill {
  position: fixed; z-index: 2147483647; display: flex; align-items: center; gap: 8px;
  padding: 7px 9px; border-radius: 12px; background: #2e3035; color: #f1f2f4;
  box-shadow: 0 8px 28px rgba(0,0,0,.34), 0 0 0 1px rgba(255,255,255,.09);
  opacity: 0; transform: translateY(4px); transition: opacity .12s, transform .12s;
}
.pill.show { opacity: 1; transform: translateY(0); }
.shades { display: flex; gap: 6px; }
.shade { width: 16px; height: 16px; border-radius: 50%; border: 2px solid transparent; cursor: pointer; padding: 0; }
.shade.active { border-color: #f1f2f4; }
.sep { width: 1px; height: 20px; background: rgba(255,255,255,.16); }
.btn {
  font-size: 12.5px; font-weight: 600; border: none; border-radius: 8px; padding: 6px 11px;
  cursor: pointer; color: #0c0d11; background: linear-gradient(101deg,#7ad0ff,#52a0ff,#4f7cf7);
}
.btn.ghost { background: transparent; color: #c3c5cb; }
.btn.ghost:hover { color: #f1f2f4; }
.btn:disabled { opacity: .6; cursor: default; }
.note { display: none; }
.note.show { display: block; }
.note input {
  font-size: 12.5px; color: #f1f2f4; background: #212328; border: 1px solid rgba(255,255,255,.14);
  border-radius: 8px; padding: 6px 9px; width: 180px; outline: none;
}
.note input:focus { border-color: #52a0ff; }
.toast {
  position: fixed; left: 50%; bottom: 26px; transform: translate(-50%, 8px); z-index: 2147483647;
  background: #2e3035; color: #f1f2f4; padding: 11px 16px; border-radius: 12px; font-size: 13px;
  box-shadow: 0 8px 28px rgba(0,0,0,.34), 0 0 0 1px rgba(255,255,255,.09);
  opacity: 0; transition: opacity .18s, transform .18s; pointer-events: none; max-width: 80vw;
}
.toast.show { opacity: 1; transform: translate(-50%, 0); }
.toast .dot { display: inline-block; width: 8px; height: 8px; border-radius: 50%; margin-right: 8px;
  background: linear-gradient(101deg,#7ad0ff,#52a0ff,#4f7cf7); vertical-align: middle; }
.toast.error .dot { background: #ff6b6b; }
`;

export function createUi(
  onKeep: (color: string, note: string | null) => void,
  onDismiss?: () => void,
): Ui {
  const host = document.createElement("div");
  host.style.cssText = "all: initial;";
  const root = host.attachShadow({ mode: "open" });
  const style = document.createElement("style");
  style.textContent = STYLE;
  root.appendChild(style);

  let currentShade = "iridescent";
  let toastTimer: number | undefined;

  const pill = document.createElement("div");
  pill.className = "pill";
  // Preserve the page selection when interacting with the pill - except on the
  // note input, which needs the click to focus and place its caret.
  pill.addEventListener("mousedown", (e) => {
    if (e.target !== noteInput) e.preventDefault();
  });

  const shades = document.createElement("div");
  shades.className = "shades";
  const shadeButtons: HTMLButtonElement[] = [];
  for (const shade of SHADES) {
    const dot = document.createElement("button");
    dot.className = "shade";
    dot.style.background = shade.grad;
    dot.title = shade.id;
    dot.addEventListener("click", () => setShade(shade.id));
    shades.appendChild(dot);
    shadeButtons.push(dot);
  }

  const sep = document.createElement("div");
  sep.className = "sep";

  const noteWrap = document.createElement("span");
  noteWrap.className = "note";
  const noteInput = document.createElement("input");
  noteInput.type = "text";
  noteInput.placeholder = "Add a note, then Keep";
  noteInput.addEventListener("keydown", (e) => {
    if (e.key === "Enter") keep();
  });
  noteWrap.appendChild(noteInput);

  const noteBtn = document.createElement("button");
  noteBtn.className = "btn ghost";
  noteBtn.textContent = "＋ Note";
  noteBtn.addEventListener("click", () => {
    noteWrap.classList.toggle("show");
    if (noteWrap.classList.contains("show")) noteInput.focus();
  });

  const keepBtn = document.createElement("button");
  keepBtn.className = "btn";
  keepBtn.textContent = "Keep";
  keepBtn.addEventListener("click", () => keep());

  pill.append(shades, sep, noteWrap, noteBtn, keepBtn);

  const toast = document.createElement("div");
  toast.className = "toast";
  const toastDot = document.createElement("span");
  toastDot.className = "dot";
  const toastText = document.createElement("span");
  toast.append(toastDot, toastText);

  root.append(pill, toast);
  document.documentElement.appendChild(host);

  function hide(): void {
    pill.classList.remove("show");
    noteWrap.classList.remove("show");
  }

  function dismiss(): void {
    if (!pill.classList.contains("show")) return;
    hide();
    onDismiss?.();
  }

  // Dismiss on any pointer press outside the pill - including while the note is
  // open, when a lost selection alone can't be trusted to hide it. composedPath
  // sees through the shadow boundary, so presses inside the pill are excluded.
  document.addEventListener(
    "pointerdown",
    (e) => {
      if (e.composedPath().includes(host)) return;
      dismiss();
    },
    true,
  );

  // A page that stops pointerdown in the capture phase never lets the press above
  // reach us, and while the note is open a collapsed selection can't stand in for
  // it either. Focus leaving the note is the one signal a page can't intercept.
  // relatedTarget is retargeted across the shadow boundary, so let focus settle
  // and then ask the shadow root where it actually landed.
  noteInput.addEventListener("focusout", () => {
    setTimeout(() => {
      // Collapsing the note hides the focused input, which blurs it too - that
      // close is deliberate and must leave the pill standing.
      if (!noteWrap.classList.contains("show")) return;
      if (!root.activeElement) dismiss();
    }, 0);
  });

  function setShade(id: string): void {
    currentShade = id;
    shadeButtons.forEach((btn, i) => btn.classList.toggle("active", SHADES[i].id === id));
  }

  function keep(): void {
    const note = noteWrap.classList.contains("show") ? noteInput.value.trim() || null : null;
    onKeep(currentShade, note);
  }

  // Position above the selection, clamped to the viewport.
  function positionPill(rect: DOMRect): void {
    pill.style.visibility = "hidden";
    pill.classList.add("show");
    const width = pill.offsetWidth || 240;
    const top = Math.max(8, rect.top - (pill.offsetHeight || 40) - 8);
    const left = Math.min(Math.max(8, rect.left), window.innerWidth - width - 8);
    pill.style.top = `${top}px`;
    pill.style.left = `${left}px`;
    pill.style.visibility = "visible";
  }

  return {
    showPill(rect, shade) {
      setShade(shade);
      noteWrap.classList.remove("show");
      noteInput.value = "";
      keepBtn.disabled = false;
      keepBtn.textContent = "Keep";
      positionPill(rect);
    },
    repositionPill(rect) {
      positionPill(rect);
    },
    hidePill() {
      hide();
    },
    isEngaged() {
      return root.activeElement === noteInput;
    },
    setBusy(busy) {
      keepBtn.disabled = busy;
      keepBtn.textContent = busy ? "Keeping…" : "Keep";
    },
    showToast(text, tone) {
      toastText.textContent = text;
      toast.classList.toggle("error", tone === "error");
      toast.classList.add("show");
      window.clearTimeout(toastTimer);
      toastTimer = window.setTimeout(() => toast.classList.remove("show"), 3800);
    },
  };
}
