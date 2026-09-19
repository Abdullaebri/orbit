const { event, core } = window.__TAURI__;

const ring = document.getElementById("ring");
const stage = document.getElementById("stage");
const slotsEl = document.getElementById("slots");
const hub = document.getElementById("hub");
const hubLabel = document.getElementById("hubLabel");

const RADIUS = 106; // px from the ring centre to a slot centre
const START_ANGLE = -90; // slot 0 sits at 12 o'clock

let slots = [];

function close() {
  core.invoke("hide_overlay");
}

function iconFor(name) {
  return window.RD_ICONS[name] || window.RD_ICONS.blank;
}

function showLabel(text) {
  hubLabel.textContent = text;
  ring.classList.add("labelled");
}

function clearLabel() {
  ring.classList.remove("labelled");
}

/// Buttons shrink as the ring fills up, so twelve of them still fit without
/// overlapping. Shared with the settings preview.
function slotSize(count) {
  if (count < 2) return 56;
  const spacing = 2 * RADIUS * Math.sin(Math.PI / count);
  return Math.max(34, Math.min(56, Math.round(spacing - 10)));
}

function render() {
  slotsEl.replaceChildren();
  if (!slots.length) return;

  const size = slotSize(slots.length);

  slots.forEach((slot, i) => {
    const angle = ((START_ANGLE + (360 / slots.length) * i) * Math.PI) / 180;
    const el = document.createElement("button");
    el.className = "slot";
    el.type = "button";
    el.title = slot.label;
    el.setAttribute("aria-label", slot.label);
    el.style.width = `${size}px`;
    el.style.height = `${size}px`;
    el.style.margin = `${-size / 2}px 0 0 ${-size / 2}px`;
    el.style.setProperty("--tx", `${(Math.cos(angle) * RADIUS).toFixed(2)}px`);
    el.style.setProperty("--ty", `${(Math.sin(angle) * RADIUS).toFixed(2)}px`);
    el.style.animationDelay = `${i * 22}ms`;
    el.innerHTML = `<svg viewBox="0 0 24 24" aria-hidden="true">${iconFor(
      slot.icon
    )}</svg>`;

    el.addEventListener("mouseenter", () => showLabel(slot.label));
    el.addEventListener("mouseleave", clearLabel);
    // mousedown, not click: the window hides on the press, so the matching
    // mouseup never lands on this element.
    el.addEventListener("mousedown", (e) => {
      if (e.button !== 0) return;
      e.preventDefault();
      core.invoke("run_slot", { index: i });
    });

    slotsEl.appendChild(el);
  });
}

async function load() {
  try {
    const config = await core.invoke("get_config");
    slots = config.slots || [];
    render();
  } catch (err) {
    console.error("[radialdock] could not load slots", err);
  }
}

event.listen("config-changed", load);

// Re-read on every open: cheap, and it means the ring is never stale even if
// the very first load raced the backend starting up.
event.listen("overlay-open", async () => {
  clearLabel();
  ring.style.animation = "none";
  void ring.offsetWidth;
  ring.style.animation = "";
  await load();
});

event.listen("overlay-close", clearLabel);

hub.addEventListener("mousedown", (e) => {
  if (e.button === 0) close();
});

// The window is a square; presses in the transparent corners dismiss too.
stage.addEventListener("mousedown", (e) => {
  if (!ring.contains(e.target)) close();
});

window.addEventListener("keydown", (e) => {
  if (e.key === "Escape") close();
});

window.addEventListener("contextmenu", (e) => e.preventDefault());

load();
