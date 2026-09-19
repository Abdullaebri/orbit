const { core, event } = window.__TAURI__;

const RADIUS = 106;
const START_ANGLE = -90;
const MIN_SLOTS = 2;
const MAX_SLOTS = 12;

const ACTION_KINDS = [
  ["launch", "Open app or file"],
  ["url", "Open a URL"],
  ["keys", "Send keyboard shortcut"],
  ["command", "Run a command"],
  ["settings", "Open Orbit settings"],
  ["none", "Do nothing"],
];

const FIELDS = {
  launch: [
    ["path", "notepad.exe   or   C:\\Users\\me\\report.docx"],
    ["args", "arguments (optional)"],
  ],
  url: [["url", "https://chatgpt.com/"]],
  keys: [["combo", "win+shift+s"]],
  command: [["command", "rundll32.exe user32.dll,LockWorkStation"]],
  settings: [],
  none: [],
};

const el = (id) => document.getElementById(id);
const pslots = el("pslots");
const ringWrap = document.querySelector(".ring-wrap");
const ghost = el("ghost");

let config = null;
let selected = 0;

const PRESETS = [
  ["Screenshot", "screenshot", { kind: "keys", combo: "win+shift+s" }],
  ["Emoji Picker", "emoji", { kind: "keys", combo: "win+." }],
  ["Clipboard", "clipboard", { kind: "keys", combo: "win+v" }],
  ["Search", "search", { kind: "keys", combo: "win+s" }],
  ["Run", "terminal", { kind: "keys", combo: "win+r" }],
  ["Show Desktop", "browser", { kind: "keys", combo: "win+d" }],
  ["Play / Pause", "play_pause", { kind: "keys", combo: "media_play_pause" }],
  ["Mute", "volume", { kind: "keys", combo: "volume_mute" }],
  ["Lock PC", "lock", {
    kind: "launch",
    path: "rundll32.exe",
    args: ["user32.dll,LockWorkStation"],
  }],
  ["Task Manager", "taskmgr", { kind: "launch", path: "taskmgr.exe", args: [] }],
  ["File Explorer", "folder", { kind: "launch", path: "explorer.exe", args: [] }],
  ["Volume Mixer", "volume", { kind: "launch", path: "SndVol.exe", args: [] }],
  ["New Note", "note", { kind: "launch", path: "notepad.exe", args: [] }],
  ["Calculator", "calculator", { kind: "launch", path: "calc.exe", args: [] }],
  ["Copy", "copy", { kind: "keys", combo: "ctrl+c" }],
  ["Paste", "paste", { kind: "keys", combo: "ctrl+v" }],
  // Office executables are registered under App Paths, so these names resolve
  // on any machine with Office installed - no hard-coded install path.
  ["Word", "word", { kind: "launch", path: "winword.exe", args: [] }],
  ["Excel", "excel", { kind: "launch", path: "excel.exe", args: [] }],
  ["PowerPoint", "slides", { kind: "launch", path: "powerpnt.exe", args: [] }],
  ["Outlook", "mail", { kind: "launch", path: "outlook.exe", args: [] }],
  ["OneNote", "clipboard", { kind: "launch", path: "onenote.exe", args: [] }],
  ["Access", "app", { kind: "launch", path: "msaccess.exe", args: [] }],
  ["Ask AI", "ask_ai", { kind: "url", url: "https://chatgpt.com/" }],
  ["Translate", "translate", { kind: "url", url: "https://translate.google.com/" }],
  ["Mail", "mail", { kind: "url", url: "https://mail.google.com/" }],
  ["Settings", "settings", { kind: "settings" }],
];

function say(text, tone = "") {
  el("status").textContent = text;
  el("status").className = `status ${tone}`;
}

function touched() {
  say("Unsaved changes");
}

function iconSvg(name) {
  const path = window.RD_ICONS[name] || window.RD_ICONS.blank;
  return `<svg viewBox="0 0 24 24" aria-hidden="true">${path}</svg>`;
}

/// Mirrors the overlay so the preview is a true likeness of the real ring.
function slotSize(count) {
  if (count < 2) return 56;
  const spacing = 2 * RADIUS * Math.sin(Math.PI / count);
  return Math.max(34, Math.min(56, Math.round(spacing - 10)));
}

function splitArgs(text) {
  const out = [];
  const re = /"([^"]*)"|(\S+)/g;
  let m;
  while ((m = re.exec(text))) out.push(m[1] !== undefined ? m[1] : m[2]);
  return out;
}

function joinArgs(args) {
  return (args || []).map((a) => (/\s/.test(a) ? `"${a}"` : a)).join(" ");
}

function guessIcon(path) {
  const ext = (path.split(".").pop() || "").toLowerCase();
  if (!path.includes(".")) return "folder";
  if (["exe", "lnk", "bat", "cmd", "msi"].includes(ext)) return "app";
  if (["docx", "doc", "txt", "md", "pdf", "rtf"].includes(ext)) return "note";
  if (["png", "jpg", "jpeg", "gif", "webp", "bmp"].includes(ext)) return "camera";
  if (["mp3", "wav", "flac", "m4a", "ogg"].includes(ext)) return "music";
  if (["xlsx", "xls", "csv"].includes(ext)) return "calculator";
  if (["html", "htm", "url"].includes(ext)) return "browser";
  return "star";
}

function titleFromPath(path) {
  const name = path.replace(/[\\/]+$/, "").split(/[\\/]/).pop() || path;
  const stem = name.replace(/\.[^.]+$/, "");
  return stem.charAt(0).toUpperCase() + stem.slice(1);
}

function applyPath(slot, path) {
  slot.label = titleFromPath(path);
  slot.icon = guessIcon(path);
  slot.action = { kind: "launch", path, args: [] };
}

function applyPreset(index, [label, icon, action]) {
  const slot = config.slots[index];
  if (!slot) return;
  slot.label = label;
  slot.icon = icon;
  slot.action = JSON.parse(JSON.stringify(action));
  selected = index;
  renderRing();
  renderInspector();
  touched();
}

function blankSlot() {
  return { label: "New slot", icon: "blank", action: { kind: "none" } };
}

function renderRing() {
  pslots.replaceChildren();
  const n = config.slots.length;
  const size = slotSize(n);

  config.slots.forEach((slot, i) => {
    const angle = ((START_ANGLE + (360 / n) * i) * Math.PI) / 180;
    const node = document.createElement("div");
    node.className = "pslot" + (i === selected ? " selected" : "");
    node.dataset.index = String(i);
    node.title = slot.label;
    node.style.width = `${size}px`;
    node.style.height = `${size}px`;
    node.style.margin = `${-size / 2}px 0 0 ${-size / 2}px`;
    node.style.setProperty("--tx", `${(Math.cos(angle) * RADIUS).toFixed(2)}px`);
    node.style.setProperty("--ty", `${(Math.sin(angle) * RADIUS).toFixed(2)}px`);
    node.innerHTML = iconSvg(slot.icon);
    node.addEventListener("pointerdown", (e) => beginSlotDrag(e, i));
    pslots.appendChild(node);
  });

  el("phubLabel").textContent = `${n} slots`;
  el("removeSlot").disabled = n <= MIN_SLOTS;
  el("addSlot").disabled = n >= MAX_SLOTS;
}

function slotAt(clientX, clientY) {
  for (const node of pslots.children) {
    const r = node.getBoundingClientRect();
    const dx = clientX - (r.left + r.width / 2);
    const dy = clientY - (r.top + r.height / 2);
    if (Math.hypot(dx, dy) <= r.width / 2 + 6) return Number(node.dataset.index);
  }
  return null;
}

/// True anywhere inside the ring itself, including the gaps between slots.
function insideRing(clientX, clientY) {
  const r = el("preview").getBoundingClientRect();
  const dx = clientX - (r.left + r.width / 2);
  const dy = clientY - (r.top + r.height / 2);
  return Math.hypot(dx, dy) <= r.width / 2;
}

/// Appends an empty slot and selects it. Returns its index, or null when the
/// ring is already full.
function addSlot() {
  if (config.slots.length >= MAX_SLOTS) {
    say(`A ring holds at most ${MAX_SLOTS} slots.`, "bad");
    return null;
  }
  config.slots.push(blankSlot());
  selected = config.slots.length - 1;
  return selected;
}

function clearTargets() {
  for (const node of pslots.children) node.classList.remove("target");
}

function markTarget(clientX, clientY) {
  clearTargets();
  const i = slotAt(clientX, clientY);
  if (i !== null) pslots.children[i].classList.add("target");
  return i;
}

function showGhost(label, icon) {
  ghost.innerHTML = `${iconSvg(icon)}<span>${label}</span>`;
  ghost.hidden = false;
}

function moveGhost(x, y) {
  ghost.style.left = `${x}px`;
  ghost.style.top = `${y}px`;
}

function hideGhost() {
  ghost.hidden = true;
}

/// One pointer-drag helper for both ring slots and preset chips. Pointer
/// events rather than HTML5 drag-and-drop, because the window has OS file
/// dropping enabled and the two do not coexist on Windows.
function dragFrom(e, { label, icon, onDrop, onDropEmpty, onClick }) {
  if (e.button !== 0) return;
  e.preventDefault();

  const startX = e.clientX;
  const startY = e.clientY;
  let dragging = false;

  function onMove(ev) {
    if (!dragging && Math.hypot(ev.clientX - startX, ev.clientY - startY) > 5) {
      dragging = true;
      showGhost(label, icon);
    }
    if (dragging) {
      moveGhost(ev.clientX, ev.clientY);
      markTarget(ev.clientX, ev.clientY);
    }
  }

  function onUp(ev) {
    document.removeEventListener("pointermove", onMove);
    document.removeEventListener("pointerup", onUp);
    hideGhost();
    clearTargets();
    if (!dragging) {
      onClick?.();
      return;
    }
    const target = slotAt(ev.clientX, ev.clientY);
    if (target !== null) {
      onDrop(target);
    } else if (insideRing(ev.clientX, ev.clientY)) {
      // Dropped into the ring but not onto an existing slot: make a new one.
      onDropEmpty?.();
    }
  }

  document.addEventListener("pointermove", onMove);
  document.addEventListener("pointerup", onUp);
}

function beginSlotDrag(e, index) {
  const slot = config.slots[index];
  dragFrom(e, {
    label: slot.label,
    icon: slot.icon,
    onClick: () => {
      selected = index;
      renderRing();
      renderInspector();
    },
    onDrop: (target) => {
      if (target === index) return;
      const tmp = config.slots[index];
      config.slots[index] = config.slots[target];
      config.slots[target] = tmp;
      selected = target;
      renderRing();
      renderInspector();
      touched();
    },
  });
}

let installedApps = [];

/// Presets first, then anything the Start menu can launch. Store apps have no
/// .exe path, so they arrive as `shell:AppsFolder\<AUMID>` targets.
function candidates() {
  const query = el("search").value.trim().toLowerCase();
  const apps = installedApps.map((a) => [a.name, "app", {
    kind: "launch",
    path: a.target,
    args: [],
  }]);
  const all = [...PRESETS, ...apps];
  if (!query) return PRESETS;
  return all.filter(([label]) => label.toLowerCase().includes(query)).slice(0, 60);
}

function renderPresets() {
  const host = el("presets");
  host.replaceChildren();

  const list = candidates();
  if (!list.length) {
    const empty = document.createElement("p");
    empty.className = "hint tight";
    empty.textContent = "Nothing matched.";
    host.appendChild(empty);
    return;
  }

  for (const preset of list) {
    const [label, icon] = preset;
    const chip = document.createElement("div");
    chip.className = "chip";
    chip.innerHTML = `${iconSvg(icon)}<span>${label}</span>`;
    chip.addEventListener("pointerdown", (e) =>
      dragFrom(e, {
        label,
        icon,
        onClick: () => applyPreset(selected, preset),
        onDrop: (target) => applyPreset(target, preset),
        onDropEmpty: () => {
          const i = addSlot();
          if (i !== null) applyPreset(i, preset);
        },
      })
    );
    host.appendChild(chip);
  }
}

function fieldValue(action, key) {
  if (key === "args") return joinArgs(action.args);
  return action[key] ?? "";
}

function setFieldValue(action, key, value) {
  if (key === "args") action.args = splitArgs(value);
  else action[key] = value;
}

function blankAction(kind, previous) {
  const next = { kind };
  for (const [key] of FIELDS[kind]) {
    if (key === "args") next.args = previous?.args ?? [];
    else next[key] = previous?.[key] ?? "";
  }
  return next;
}

function field(labelText, control) {
  const wrap = document.createElement("div");
  wrap.className = "field";
  const lab = document.createElement("label");
  lab.textContent = labelText;
  wrap.append(lab, control);
  return wrap;
}

function renderInspector() {
  const host = el("inspector");
  host.replaceChildren();
  const slot = config.slots[selected];
  if (!slot) return;

  el("slotTitle").textContent = `Slot ${selected + 1} of ${config.slots.length}`;

  const label = document.createElement("input");
  label.type = "text";
  label.value = slot.label;
  label.placeholder = "Shown when you hover the slot";
  label.addEventListener("input", () => {
    slot.label = label.value;
    pslots.children[selected].title = label.value;
    touched();
  });
  host.appendChild(field("Label", label));

  const grid = document.createElement("div");
  grid.className = "icon-grid";
  for (const name of Object.keys(window.RD_ICONS)) {
    const opt = document.createElement("button");
    opt.type = "button";
    opt.className = "icon-opt" + (slot.icon === name ? " on" : "");
    opt.title = name.replace(/_/g, " ");
    opt.innerHTML = iconSvg(name);
    opt.addEventListener("click", () => {
      slot.icon = name;
      renderRing();
      renderInspector();
      touched();
    });
    grid.appendChild(opt);
  }
  host.appendChild(field("Icon", grid));

  const kindSel = document.createElement("select");
  for (const [kind, name] of ACTION_KINDS) {
    const option = document.createElement("option");
    option.value = kind;
    option.textContent = name;
    if (slot.action.kind === kind) option.selected = true;
    kindSel.appendChild(option);
  }
  kindSel.addEventListener("change", () => {
    slot.action = blankAction(kindSel.value, slot.action);
    renderInspector();
    touched();
  });
  host.appendChild(field("Action", kindSel));

  for (const [key, placeholder] of FIELDS[slot.action.kind] || []) {
    const input = document.createElement("input");
    input.type = "text";
    input.placeholder = placeholder;
    input.value = fieldValue(slot.action, key);
    input.addEventListener("input", () => {
      setFieldValue(slot.action, key, input.value);
      touched();
    });
    host.appendChild(field(key === "args" ? "Arguments" : key, input));
  }

  const test = document.createElement("button");
  test.type = "button";
  test.className = "btn small";
  test.textContent = "Test this slot";
  test.addEventListener("click", async () => {
    if (await persist()) {
      await core.invoke("run_slot", { index: selected });
      say(`Ran "${slot.label}".`, "ok");
    }
  });
  host.appendChild(test);
}

// --- Dropping files straight from Explorer -------------------------------

function toCss(position) {
  const dpr = window.devicePixelRatio || 1;
  return { x: position.x / dpr, y: position.y / dpr };
}

event.listen("tauri://drag-over", ({ payload }) => {
  ringWrap.classList.add("dropping");
  const p = toCss(payload.position);
  markTarget(p.x, p.y);
});

event.listen("tauri://drag-leave", () => {
  ringWrap.classList.remove("dropping");
  clearTargets();
});

event.listen("tauri://drag-drop", ({ payload }) => {
  ringWrap.classList.remove("dropping");
  clearTargets();

  const paths = payload.paths || [];
  if (!paths.length || !config) return;

  const p = toCss(payload.position);
  const landed = slotAt(p.x, p.y);

  paths.forEach((path, n) => {
    let index;
    if (n === 0 && landed !== null) {
      // Dropped squarely on a slot: replace that one.
      index = landed;
    } else {
      // Anywhere else in the ring, and every extra file, becomes a new slot.
      index = addSlot();
      if (index === null) return;
    }
    applyPath(config.slots[index], path);
    selected = index;
  });

  renderRing();
  renderInspector();
  say(`Added ${paths.length} item${paths.length === 1 ? "" : "s"} — press Save.`);
});

// --- Slot count ----------------------------------------------------------

el("addSlot").addEventListener("click", () => {
  if (addSlot() === null) return;
  renderRing();
  renderInspector();
  touched();
});

el("removeSlot").addEventListener("click", () => {
  if (config.slots.length <= MIN_SLOTS) return;
  config.slots.splice(selected, 1);
  selected = Math.min(selected, config.slots.length - 1);
  renderRing();
  renderInspector();
  touched();
});

// --- Trigger -------------------------------------------------------------

let armed = false;

function triggerText(t) {
  if (!t) return "—";
  if (t.kind === "mouse") {
    const known = {
      left: "Left mouse button",
      right: "Right mouse button",
      middle: "Middle mouse button",
    };
    return known[t.button] || `Mouse button ${String(t.button).toUpperCase()}`;
  }
  return `${t.key} key`;
}

function setArmed(on) {
  armed = on;
  el("capture").textContent = on ? "Cancel" : "Change";
  el("triggerNow").classList.toggle("armed", on);
  el("triggerNow").textContent = on
    ? "Press any key or button…"
    : triggerText(config.trigger);
}

el("capture").addEventListener("click", async () => {
  if (armed) {
    await core.invoke("cancel_trigger_capture");
    setArmed(false);
    return;
  }
  setArmed(true);
  await core.invoke("start_trigger_capture");
});

event.listen("trigger-captured", ({ payload }) => {
  const [kind, name] = payload;
  if (name) {
    config.trigger =
      kind === "mouse"
        ? { kind: "mouse", button: name }
        : { kind: "key", key: name };
  }
  setArmed(false);
  if (name) touched();
});

// --- Autostart: applied immediately, then read back from Windows ---------

el("autostart").addEventListener("change", async (e) => {
  const wanted = e.target.checked;
  try {
    const actual = await core.invoke("set_autostart", { enabled: wanted });
    e.target.checked = actual;
    config.autostart = actual;
    say(
      actual
        ? "RadialDock will start with Windows."
        : "RadialDock will no longer start with Windows.",
      "ok"
    );
  } catch (err) {
    e.target.checked = !wanted;
    say(String(err), "bad");
  }
});

// --- Save / reset --------------------------------------------------------

async function persist() {
  try {
    config = await core.invoke("save_config", { config });
    selected = Math.min(selected, config.slots.length - 1);
    renderRing();
    say("Saved.", "ok");
    return true;
  } catch (err) {
    say(String(err), "bad");
    return false;
  }
}

el("save").addEventListener("click", persist);

el("reset").addEventListener("click", async () => {
  try {
    config = await core.invoke("reset_config");
    selected = 0;
    hydrate();
    say("Reset to defaults.", "ok");
  } catch (err) {
    say(String(err), "bad");
  }
});

window.addEventListener("keydown", (e) => {
  if (e.key === "Escape" && !armed) core.invoke("close_settings");
  if (e.key === "s" && e.ctrlKey) {
    e.preventDefault();
    persist();
  }
});

function hydrate() {
  setArmed(false);
  renderRing();
  renderInspector();
}

el("search").addEventListener("input", renderPresets);

(async function start() {
  try {
    config = await core.invoke("get_config");
    el("where").textContent = await core.invoke("config_location");
    el("autostart").checked = await core.invoke("autostart_enabled");
    config.autostart = el("autostart").checked;
    renderPresets();
    hydrate();
    say("");
  } catch (err) {
    say(`Could not load settings: ${err}`, "bad");
  }

  // Slower (it asks Windows for the Start menu), so it lands separately.
  try {
    installedApps = await core.invoke("list_apps");
    el("searchHint").textContent =
      `Drag one onto a slot, or click to drop it into the selected slot. ` +
      `${installedApps.length} installed apps found.`;
    renderPresets();
  } catch (err) {
    console.error("[orbit] could not list apps", err);
  }
})();
