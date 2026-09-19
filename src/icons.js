// Stroke icons, 24x24 viewBox. Keyed by Slot.icon from the Rust side.
window.RD_ICONS = {
  play_pause: '<path d="M4 5v14M8.5 5v14"/><path d="M13.5 5l7.5 7-7.5 7z"/>',
  folder:
    '<path d="M3 7.5A2.5 2.5 0 0 1 5.5 5h3.2l2 2.2h7.8A2.5 2.5 0 0 1 21 9.7v7.8A2.5 2.5 0 0 1 18.5 20h-13A2.5 2.5 0 0 1 3 17.5z"/>',
  note:
    '<path d="M6 3h7.5L19 8.5V20a1 1 0 0 1-1 1H6a1 1 0 0 1-1-1V4a1 1 0 0 1 1-1z"/><path d="M13.5 3v5.5H19"/><path d="M9 14.5h6M12 11.5v6"/>',
  emoji:
    '<circle cx="12" cy="12" r="8.6"/><path d="M8.4 14.2a4.4 4.4 0 0 0 7.2 0"/><path d="M9.3 9.6h.01M14.7 9.6h.01"/>',
  screenshot:
    '<path d="M7.5 3v13.5A1.5 1.5 0 0 0 9 18h12"/><path d="M3 7.5h13.5A1.5 1.5 0 0 1 18 9v12"/>',
  lock:
    '<rect x="4.2" y="10.2" width="15.6" height="10.6" rx="2.2"/><path d="M8.2 10.2V7.4a3.8 3.8 0 0 1 7.6 0v2.8"/>',
  volume:
    '<path d="M4 9.2h3.2L12 5.2v13.6L7.2 14.8H4z"/><path d="M16.2 9.4a4.6 4.6 0 0 1 0 5.2"/><path d="M18.9 6.8a8.4 8.4 0 0 1 0 10.4"/>',
  taskmgr: '<path d="M4.5 20V11M9.5 20V4.5M14.5 20v-6.5M19.5 20V8"/>',
  settings:
    '<circle cx="12" cy="12" r="3.1"/><path d="M12 3.2l1.5 2.3 2.7-.5.6 2.7 2.5 1.1-1.2 2.5 1.2 2.5-2.5 1.1-.6 2.7-2.7-.5L12 20.8l-1.5-2.3-2.7.5-.6-2.7-2.5-1.1L5.9 12 4.7 9.5l2.5-1.1.6-2.7 2.7.5z"/>',
  ask_ai:
    '<path d="M4 6.5A2.5 2.5 0 0 1 6.5 4h11A2.5 2.5 0 0 1 20 6.5v7A2.5 2.5 0 0 1 17.5 16H9l-5 4z"/><path d="M12 7.2l1.15 2.4 2.4 1.15-2.4 1.15L12 14.3l-1.15-2.4L8.45 10.75l2.4-1.15z"/>',
  translate:
    '<path d="M3.5 6h8M7.5 4.2V6M9.4 6c0 3.4-2.6 6.6-6 8.2M5.2 9.6c1 2 2.9 3.6 5.2 4.4"/><path d="M12.5 20l4-9.5 4 9.5M13.9 16.8h5.2"/>',
  browser:
    '<circle cx="12" cy="12" r="8.6"/><path d="M3.6 9.6h16.8M3.6 14.4h16.8"/><path d="M12 3.4c2.2 2.3 3.3 5.2 3.3 8.6S14.2 18.3 12 20.6c-2.2-2.3-3.3-5.2-3.3-8.6S9.8 5.7 12 3.4z"/>',
  mail:
    '<rect x="3" y="5.2" width="18" height="13.6" rx="2.2"/><path d="M3.8 6.6l8.2 6 8.2-6"/>',
  terminal:
    '<rect x="3" y="4.5" width="18" height="15" rx="2.2"/><path d="M7 9.5l3 2.5-3 2.5M12.5 15h4.5"/>',
  calculator:
    '<rect x="5" y="3.2" width="14" height="17.6" rx="2.2"/><path d="M8.2 7.4h7.6M8.6 12h.01M12 12h.01M15.4 12h.01M8.6 16h.01M12 16h.01M15.4 16h.01"/>',
  clipboard:
    '<path d="M9 4.6H7.5a2 2 0 0 0-2 2v12a2 2 0 0 0 2 2h9a2 2 0 0 0 2-2v-12a2 2 0 0 0-2-2H15"/><rect x="9" y="2.8" width="6" height="3.6" rx="1.2"/>',
  search:
    '<circle cx="10.8" cy="10.8" r="6.4"/><path d="M15.6 15.6L20.5 20.5"/>',
  music:
    '<path d="M9 18V6.5l10-2v11"/><circle cx="6.5" cy="18" r="2.5"/><circle cx="16.5" cy="15.5" r="2.5"/>',
  camera:
    '<path d="M3.5 8.5A2 2 0 0 1 5.5 6.5h2l1.4-2h6.2l1.4 2h2a2 2 0 0 1 2 2v8a2 2 0 0 1-2 2h-13a2 2 0 0 1-2-2z"/><circle cx="12" cy="12.5" r="3.4"/>',
  power:
    '<path d="M12 3.6v8.2"/><path d="M7.4 6.4a7.6 7.6 0 1 0 9.2 0"/>',
  star:
    '<path d="M12 3.6l2.6 5.5 5.9.8-4.3 4.2 1.1 6-5.3-2.9-5.3 2.9 1.1-6L3.5 9.9l5.9-.8z"/>',
  app:
    '<rect x="3.8" y="3.8" width="16.4" height="16.4" rx="4.4"/><circle cx="12" cy="12" r="2.7"/>',
  slides:
    '<rect x="3" y="4.4" width="18" height="12.2" rx="2"/><path d="M12 16.6v3M8.6 19.6h6.8"/>',
  copy:
    '<rect x="9" y="9" width="11" height="11" rx="2.2"/><path d="M15 9V6.2A2.2 2.2 0 0 0 12.8 4H6.2A2.2 2.2 0 0 0 4 6.2v6.6A2.2 2.2 0 0 0 6.2 15H9"/>',
  paste:
    '<path d="M9 4.6H7.5a2 2 0 0 0-2 2v12a2 2 0 0 0 2 2h9a2 2 0 0 0 2-2v-12a2 2 0 0 0-2-2H15"/><rect x="9" y="2.8" width="6" height="3.6" rx="1.2"/><path d="M9.2 13.4h5.6M9.2 16.6h3.6"/>',
  word:
    '<rect x="4" y="3.4" width="16" height="17.2" rx="2.2"/><path d="M7.6 8.4l1.8 7 2.6-5 2.6 5 1.8-7"/>',
  excel:
    '<rect x="4" y="3.4" width="16" height="17.2" rx="2.2"/><path d="M8.4 8.6l7.2 6.8M15.6 8.6l-7.2 6.8"/>',
  blank: '<circle cx="12" cy="12" r="6" stroke-dasharray="2 3"/>',
};
