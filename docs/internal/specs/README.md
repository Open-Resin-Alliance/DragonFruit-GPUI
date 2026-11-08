# Porting specs

One spec per area, written from a full read of the TypeScript source. Each
follows the same shape: components in scope, then one block per surface with
source anchor, render shape, quoted classes and inline styles, every state
branch, data and callbacks, verbatim text with msgids, and a port target —
followed by numbered gotchas and open questions.

## Caveat on the port targets

These were written by read-only agents whose working directory had no `gpui/`
tree, so every **Port target** is a *proposal*, not a verified existing module.
Reconcile it against `gpui/src/**` before implementing; the gotchas, styling and
behaviour sections are the part that was verified against the source.

## Files

| Spec | Covers |
|---|---|
| `controls.md` | design-system primitives and the `.ui-*` CSS rules |
| `shell.md` | page shell, z-index map, mode routing, the hotkey table |
| `chrome.md` | top bar, tool rail, menus |
| `panels.md` | models panel, rows, groups, context menu, stacks |
| `modals.md` | every dialog and its trigger |
| `settings-a.md` | general, camera, workspace, multi-plate |
| `settings-b.md` | mesh, slicing, 3D mouse, UI and theme |
| `settings-c.md` | hotkeys, plugins, experiments, autosave |
| `settings-d.md` | backups, uvtools, logging, updates, about, sponsors |
| `auto-support.md` | auto-support panel, presets, settings dialog |
| `transforms.md` | transform controls, the three gizmos, arrange, smoothing, organic cut |
| `profiles.md` | printer and material profiles modal, the picker |
| `printing.md` | printing preview pane and layer slider |
| `scene-overlays.md` | toasts, empty scene, layer slider, gizmo HUD, plate widgets |
