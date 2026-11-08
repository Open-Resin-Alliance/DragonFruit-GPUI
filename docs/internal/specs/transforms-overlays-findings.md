# Transforms + scene overlays — audit findings

Audit of the ported UI in this area against `docs/internal/specs/transforms.md`
and `docs/internal/specs/scene-overlays.md`. Files owned by this audit:

```
gpui/src/features/{transform_panel,arrange_panels,smoothing_panel,organic_cut_panel}.rs
gpui/src/gizmo/**
gpui/src/widgets/**            (orientation_panel.rs is new; prepare_stack.rs is deleted)
gpui/src/features/support_panels/**
gpui/crates/gpui-ui/src/widgets/card.rs   (the sub-card builder, taken on the parent's call)
```
`app.rs` was touched once, surgically, for the Auto Orientation `PanelDef` (fix #11).

Verdicts: `correct` (matches the spec), `deliberate` (differs, justified by a
`PORT DEVIATIONS` / module note in the port — the note is named below),
`wrong` (differs, no justification — fixed here where the fix is in my files),
`absent` (the spec has it, the port has nothing), `dead` (rendered but inert).

Nothing in this audit was verified by running cargo (the parent gates); the
checks are source-to-source, plus `gpui/scripts/scenario.mjs` left to the parent.

---

## 1. `TransformControls` — `gpui/src/features/transform_panel.rs`

| Surface | Verdict | Evidence |
|---|---|---|
| Card shell + collapse chevron + header | `deliberate` | Body-only; the card, chevron and drag handle live in `panels::list` (module doc, lines 1–4) |
| Move: 3-column grid, axis labels `#f87171/#4ade80/#60a5fa`, `mm` unit at 11px, values as `mm`, `Center Selection` full-width secondary | `correct` | `move_section`; `axis_label` + `Unit::new("mm", 11.0)`; `on_change` writes all three axes (`with_axis`), matching transforms.md:169-176 |
| Move commit path | `deliberate` | A stepper press stands in for blur; `on_position_commit` is called (transforms.md:177-187, gotcha 5). Documented in the new deviation bullet |
| Local toggle rendered twice (Move + Rotate), one flag | `correct` | `local_space_toggle` called from both sections (transforms.md gotcha 2) |
| Local toggle **effect** | `dead` | `on_local_space_change` writes `store.transform.local_space` (`tool_panels.rs:525`), which nothing reads: the gizmo is world-aligned by design (`gizmo/mod.rs` deviation "Local space is not wired"). The button turns its own accent on and changes nothing else |
| Lift: Auto toggle, distance field (`mm`, 10px), Lift/Drop inline colours, `disabled` when no bbox with `opacity 0.5` | `correct` | `lift_section`; colours are `mix(ACCENT, SURFACE_1, 0.10)` = 10% accent over surface-1, `mix(ACCENT, BORDER_SUBTLE, 0.62)` = accent 62% / border 38%, `mix(ACCENT, TEXT_STRONG, 0.70)` (transforms.md:186-189). **Fixed:** an invented 8%-white hover was removed (below) |
| Auto (auto-lift) toggle effect | `dead` | The flag is stored and never consumed: no auto-lift follows moves/rotations/scaling. Same class of gap as Local; the source's `useAutoLift` is scene work |
| Rotate: wrap into `(-180, 180]`, `°` at 14px, complete-then-commit order, `Reset Rotation` | `correct` | `wrap_rotation_degrees` (bit-identical to `((((deg+180)%360)+360)%360)-180` with `-0 → 0`), `complete(); commit();` order matches transforms.md:196-213 |
| Group toggle gated on `selection_rotation_mode` | `correct` | `state.selection_rotation_mode.map(...)` — renders only for a multi-selection |
| Group toggle reachability | `absent` | The shell never sets `selection_rotation_mode` (default `None`, no multi-selection path in `tool_panels.rs`), so the button cannot appear. Single-selection port |
| Scale: `%` row (labelled) + `mm` row (unlabelled), uniform rewrites all three, `Reset Scale` | `correct` | `scale_section`; non-uniform writes `with_axis(scale, axis, next)` so Y leaves X/Z alone (transforms.md:225-247, gotcha 7) |
| `mm` row zero guard | `deliberate` | The port rewrites the dead `originalSize ? … : 0` ternary as `if size == 0.0 { return; }`. `original_size` defaults to `[1,1,1]` and the shell sets `[1,1,1]` without a mesh, so the branch is unreachable — as the source's is (gotcha 6) |
| `onPlatform` | `correct` | Not modelled at all, as the source never calls it (gotcha 8) |
| Section tints (`#4f8cff`, `#8f6cff`, `#2eb67d`, `#f59e0b`) backgrounds | `correct` | `sub_card_with(Some((tint, 0.06/0.05/0.05/0.07)), 8.0)` → 94/95/95/93% surface-1 (transforms.md:59-72) |
| Section tint **borders** | `correct` after fix | `sub_card_tinted` now takes the border share per call site: Move 22%, Lift 24%, Rotate 20%, Scale 20% (`transforms.md:59-72`). Shared helper, no second card implementation. See fix #10 |
| `title` tooltips (Local, Auto-lift, Group) | `deliberate` | Not rendered — there is no hover layer in a panel body; the three sentences are now recorded verbatim in the module deviations |
| Text entry, `vh` cap, `transition-colors`, `tracking-wide` | `deliberate` | Module deviations |
| i18n (`<Trans>`/`t`) | `deliberate` (global) | The whole port renders English literals; no per-panel catalog exists |

### `AutoRotationPanel` — `orientation_body`

| Surface | Verdict | Evidence |
|---|---|---|
| Paint card (`Blocker Painting Mode`, lead copy, `Press Clear… / Done…`), `min-h-[4.5rem]`, accent-secondary-action border/bg | `correct` | `--accent-secondary-action-border` is `mix(accent-secondary, border-subtle 42%)` = 58% accent → `mix(ACCENT_SECONDARY, BORDER_SUBTLE, 0.58)`; bg-92 is 8% accent (`globals.css:36-39`); lead string is `PAINT_LEAD` (`AutoRotationPanel.tsx:77`), not the Blockers tooltip |
| Orient / Blockers buttons, `Analyzing…`, accent inline style, gate on `!activeModelId` / `!onToggleBlockers` | `correct` | `AutoRotationPanel.tsx:228-252` |
| Objective select (centred, `--surface-1`) | `correct` | `select_field(..., SURFACE_1, true, ...)` |
| Brush row (`ui-meta` label, `fmt.quantity(mm,1)`, `.ui-range`) | `deliberate` | Rendered as a static track + knob; the knob's press adds one 0.5 step, dragging is not wired (module deviation) |
| Clear / Done row (`busy`/`blockedCount` gates, Done accent) | `correct` | `AutoRotationPanel.tsx:300-320` |
| Error line (`#f87171`, 11px) | `correct` | `AXIS_X` = `#f87171` |
| Orientation sweep, busy modal, `onBeforeOrientApply`, module-level busy flag | `deliberate` | Scene work; the body reports intent (module deviation) |
| Panel reachability | `correct` after fix | `widgets/orientation_panel.rs` builds the `PanelDef` and `app.rs` pushes it beside the Transform card on the source's three gates (see fix #11). Its actions are honest stubs: `Orient` has no advisor, `Clear` has no blocker store — `dead`, documented |

---

## 2. Gizmo — `gpui/src/gizmo/**`

| Surface | Verdict | Evidence |
|---|---|---|
| `GizmoMove`: gradient shaft (`0.02 × 6.0`, 8 segments), head cone `0.24 × 0.6` straddling the tip, outline at `1.15 × 0.3` (`0.7` of dimmed), hover/active scales `1.1 / 1.18` | `correct` | `geometry::arrow`; `constants::{ARROW_*, HEAD_*, SHAFT_GRADIENT}` |
| Arrow side flip (`pointsTowardCamera` on the axis direction, same rule for arrows and cubes) | `correct` | `View::axis_sign` = `axis · (eye − center) > 0`; used by `arrow_tip` and `scale_handle` (gotcha 10) |
| Per-handle alphas (`0.9` arrow, `0.55` centre, `0.72/0.95` ring, `0.9` cube, `0.15` dimmed) | `correct` | `constants::*_ALPHA`, `geometry::opacity` |
| Picking | `deliberate` | Pixel-distance tests instead of GPU/raycast proxies (`pick.rs`, module doc) |
| Arrow tip pick radius `max(0.14, headRadius·2.35)` = `0.564`; scale box `0.5·2.3`; ring target = diamond `0.432` (+1px) | `correct` | `pick.rs`; ring arc deliberately not a target (gotcha 14) |
| Axis drag maths (closest approach, **press-captured** origin, `1e-10` skip) | `correct` | `layout::axis_param`, `Drag::begin` seeds `axis_param` and `update` measures against `self.anchor` (gotcha 11) |
| `GizmoCenter`: disc `1.05R`, border ring `1.0–1.05R` white, halo `1.1–1.32R` only when lit, `0.55` at rest, no point light | `correct` | `geometry::centre`; module table |
| Centre drag: world-XY plane at the grab's Z, `delta.z` dropped | `correct` | `Drag::plane_point` + `Handle::Center` arm (gotcha 12) |
| Centre 24px screen hover radius | `correct` | `CENTER_PICK_PX` in `pick`; the source's second hover source collapses into the same test |
| `GizmoRotation`: 90° bright arc + complement, arc `×1.02` while held, diamond cones `0.36r × r` at `±0.52r`, outline `1.08`, secondary cone `lerp(hover, start, 0.35)` | `correct` | `geometry::ring` |
| Ring pick = diamond only; `DIAL_MOVE_SLOP_PX` 3; `ROTATION_CENTER_SINGULARITY_PX` 8; sweep from the press angle, one Euler component | `correct` | `Drag::update` `Handle::Ring`; sign documented in-line (gotcha 15/16/20) |
| Ring dim remainder | `deliberate` | 270° complement that turns with the camera instead of the source's fixed 180°; alpha `0.028` is a twice-halved eye-tuned value. **Fixed:** the two doc comments claimed `0.055` |
| Protractor dial, dial model/magnet, `SnapAngleReadout`, `RotationHintTooltip` | `absent` (expected) | No 2D overlay layer exists; `gizmo/mod.rs` deviation "No protractor dial" + `drag.rs` header. New work |
| `GizmoScale`: cube of half-size `0.25·highlight`, `1.08 / 1.14` hover/active, 12 edge prisms at `×0.3` | `correct` | `geometry::scale_cube` |
| Scale drag = screen-distance ratio along the press direction, clamp `0.01…100`, `applyScaleFactor` floor `0.0001`, uniform multiply | `correct` | `Drag::update` `Handle::Scale` (gotchas 28/29) |
| Uniform-only scale | `deliberate` | Module deviation; `Transform::scale` keeps the vector for a future per-axis mode |
| Per-handle point lights (`GIZMO_LIGHTING`) | `deliberate` | Module deviation ("No handle lighting") |
| Retarget damping, `AXIS_SUPPRESS_MS`, view culling | `deliberate` | Module deviations |
| Ring writes one Euler component (not a premultiplied quaternion) | `deliberate` | Module deviation ("Euler components, not a quaternion") |
| Scale drag holds the model's underside (`preserveModelBottom`) | `deliberate` | `Drag::base_bottom` + doc comment; the source does this in the scene, the port in the drag |
| `vertices` key/memoisation | `correct` | `geometry::Key` = centre + scale + eye + hover/active |

---

## 3. Arrange / Duplicate — `gpui/src/features/arrange_panels.rs`

| Surface | Verdict | Evidence |
|---|---|---|
| Three accent cards (Layout / Plate / Arrange Mode), distance + anchor + array cards plain | `correct` | `ArrangePanel.tsx:155,182,212` accent; `:245,263,296` plain |
| Header badge (12px accent glyph + 10px muted count, `# model`/`# models`) | `correct` | `header_badge`, `format_model_count`; badge moved into the body (documented) |
| Mode pills: `activeModeStyle` fill, `disabledModeStyle` only for `isApplying`, labels `Auto/Manual`, `Multi-Plate/This Plate`, `Standard/High-Precision` | `correct` | `mode_button`, `active_mode_style()` = (bg 15% accent, border 70% accent, `--text-strong`) |
| High-Precision also turns Z-rotation on, same click | `correct` | `precision(...); rotate(true);` (gotcha 35) |
| Arrange Distance: `ScrollableNumberField`, `mm`, step 0.1, clamped `[-50, 5]`, disabled while applying | `correct` | `scrollable_field(..., 0.1, -50.0, 5.0, is_applying, ...)` (gotcha 34) |
| Array grid: `24px | Count | Gap` header, `X:/Y:/Z:` rows, counts `1..64`, gaps `-120..120` | `correct` | `array_grid` (`ArrangePanel.tsx:263-295`) |
| Placement Anchor `select_field` with the five options | `correct` | `select_field` with `SURFACE_0` |
| Allow Z-rotation row: accent card while on, `rotate-cw` glyph muted under high precision, pill `ON (Required)/ON/OFF` with 12% accent fill, disabled at `opacity 0.6` | `correct` | `allow_z_rotation`; `rgba(ACCENT, 0x1f)` = 12% |
| Apply buttons: `Primary`/`Accent` when enabled, `Secondary` when not, two-layer disabling, `Arranging…` + spinner replaces the label | `correct` | `is_all_disabled`/`is_selected_disabled`; `busy_label` (gotcha 36/37) |
| Apply button **effect** | `dead` | `tool_panels.rs:707-711` — "PORT GAP: no arrange engine — the action only re-renders". Every mode/field control writes real state; the two verbs do nothing |
| `title` tooltips (12 of them) | `deliberate` | Module deviation ("`title` tooltips are not ported … wording recorded on each control") |
| `uppercase`, steppers-not-typing, `select_field`, no transitions | `deliberate` | Module deviations |
| `DuplicatePanel` (no spec surface) | `correct` (spot-checked) | Counts `1..32`, gaps `−120..120`, copies `1..128`, spacing `−5..5` — `DuplicatePanel.tsx:110-122,239-294`. Confirm/Fill are `dead` for the same PORT GAP |

---

## 4. Mesh Smoothing — `gpui/src/features/smoothing_panel.rs`

| Surface | Verdict | Evidence |
|---|---|---|
| Brush card tint `#4f8cff`, Highlight card `#8f6cff`, body `px-2.5 pb-2.5 space-y-2` | `correct` | `sub_card_with((MOVE_TINT, 0.06))`, `(ROTATE_TINT, 0.05)`; 94%/95% surface-1 |
| Fields: brush `0.1–0.5 mm` step `0.01`; strength `0–1` step `0.01`, suffix `Strength`; iterations `1–20` rounded, step 1, suffix `Iterations` | `correct` | `stepped_field(...)` with `MESH_SMOOTHING_BRUSH_*`; `iterations(next.round().clamp(1.0, 20.0))` (transforms.md:1304-1312, gotcha 41) |
| Falloff row: `min-h 36`, `rounded-md`, 12px bold uppercase, 3 columns, active = 90% accent border / 84% surface-1 / 25% text-strong | `correct` | `falloff_button`; `mix(ACCENT, SURFACE_1, 0.16)`, `mix(ACCENT, TEXT_STRONG, 0.75)` |
| Highlight: swatch (`border-subtle`+8% white), hex box showing `#RRGGBB` upper case, 112px picker container on 94% `--surface-0` | `correct` | `swatch`, `hex_field`, `picker`; `clamped_color_input` |
| Hex field editable / picker draggable | `deliberate` | Module deviation: no text entry, no drag surface |
| Picker knob press walks the hue by 30° | `deliberate`, but **invented** | Documented in the code comment. It is an affordance the source does not have — flagged for a product decision rather than removed, since without it the colour has no editor at all |
| Restore defaults (`rotate-ccw` 12px + label, secondary, full width) | `correct` | `button_with(ButtonSpec::new(Secondary, Sm).full().gap(6).icon(...))` |
| Store clamps / localStorage persistence | `deliberate` | Module deviation ("persistence is the caller's job"); the panel clamps the brush explicitly and relies on the caller for the rest |

---

## 5. Organic Cut (Split) — `gpui/src/features/organic_cut_panel.rs`

| Surface | Verdict | Evidence |
|---|---|---|
| Cut Mode accent card, `Contour`/`Flat` pills | `correct` | `cut_accent_card` + `mode_pill` |
| Cut settings: `Seam Smoothing` `0–2`/`0.05`, `Joint Clearance` `0–1.5 mm`, `Cut Smoothing` `0–2`, `Cut Resolution` `1–4 ×` step 0.5; contour-only fields hide in flat mode | `correct` | `compact_field` calls; `OrganicCutPanel.tsx:520-600` |
| Tenon shape pills (`No Tenon`/`Frustum`/`Dome`) with the source's disable rules, fillet re-clamp on Frustum, dome floor lift `1.5`, preview restore | `correct` | `tenon_card`; `OrganicCutPanel.tsx:560-612` |
| Recentre row (`Moved off centre` + `Center` at `h-6`, disabled while busy) | `correct` | `OrganicCutPanel.tsx:614-630` |
| Width/Depth with the dome link (`link-2`/`link-2-off`, factor clamp) and `Edge Fillet` (`0…maxTenonFilletMm`, step 0.1), `Fit Tolerance` (`0…1`) | `correct` | `set_dome_dim` clamps the factor (gotcha 46); `max_tenon_fillet_mm` recomputed from live dims (gotcha 45) |
| Flip Tenon button | `correct` after fix | **Fixed:** label colour was 55% accent; spec/React are `text-strong 55%` = accent 45% (transforms.md:1529, `OrganicCutPanel.tsx:740`) |
| Tenon `Reset` | `wrong` → **fixed** | It built a whole default `OrganicCutState`, so it also reset the cut mode, clearance, both smoothings and the resolution. The source spreads `DEFAULT_TENON_SETTINGS` (tenon keys only) and keeps `generateTenon` (transforms.md:1532-1533, gotcha 44, `OrganicCutPanel.tsx:750`). Now `with_default_tenon_settings()` over the current state |
| Tenon `Reset` enabled border | `correct` after fix | **Fixed:** border was 45% red; `#f87171` at 45% of `--border-subtle` means 55% red (transforms.md:1533, `OrganicCutPanel.tsx:754`) |
| `Fair Tolerance` dome branch | `wrong` (dead code) → **fixed** | The two arms of the `if is_dome` were identical; collapsed with a comment |
| Loops card: `Loops (n)`, trash gated on `canRemoveLoop`, chips (`!h-7 !min-w-7`), dashed + muted under 3 points, active fill wins over dashed, `+` chip | `correct` | `loops_card`; `OrganicCutPanel.tsx:840-...`; transforms.md:1542-1557 |
| Snap to Edges (full-width secondary, gated on `disabled || isApplying || !canSnapToEdges`) | `correct` | `panel_button(...)`; effect is a `PORT GAP` in `tool_panels.rs:815` → `dead` until geometry snapping lands |
| Footer: error banner (`role=alert` recipe, `triangle-alert` `#ff6b6b`), `Clear All` gated on a loop with points, `Cut` accent + `Cutting...` spinner, tenon blocking the verb | `correct` | footer block; `tenon_blocks_cut` (gotcha 48) |
| `Clear All` reset of the active loop | `correct` | Clears the active loop summary (`on_clear_loop`), matching `clearLoop` |
| `Cut` effect | `dead` | `tool_panels.rs:853` — "PORT GAP: no cut engine" |
| Cut reset (`Reset cut settings to defaults`) scoped to the five cut keys, disabled unless dirty | `correct` after fix | Now `with_default_cut_settings()`; the previous inline copy was right, the helper it leaned on was not (see the tenon fix) |
| Header controls (eye + `rotate-ccw` reset) at the top of the body | `deliberate` | Module deviation (no header slot in a body) |
| `grayscale(0.7)`, one-shot reset spin, `vh` cap, tooltips | `deliberate` | Module deviations; `tenon_fillet_title`/`loop_title` are now explicitly marked as recorded-not-rendered wording |
| Panel title | `correct` | `app.rs` now mounts it as `Split`, matching the source title and the rail (transforms.md:1600-1602, `OrganicCutPanel.tsx:392`). Fixed by the parent during the audit |

---

## 6. Scene overlays — `gpui/src/widgets/**` (+ the `gpui-ui` atoms)

| Surface | Verdict | Evidence |
|---|---|---|
| `Toast` tone table (`border` = 50/50 tone·token, `background` = 10% tone over `--surface-0`), `px-4 py-2 text-sm font-semibold shadow-lg`, pill/rounded | `correct` | `gpui-ui/src/widgets/feedback.rs::toast`; scene-overlays.md tone table |
| `Toast` fade-and-rise, `pulse`, `animated` mount frame | `deliberate` | `feedback.rs` doc + `notifications.rs` deviation ("Static here: `shown == visible`") |
| `ToastViewport` (`offset` 20 stable, 72 stacked; `pointer-events-none`) | `correct` | `TOAST_OFFSET`/`TOAST_OFFSET_STACKED`; no hitbox on the wrapper |
| Toast 1 save (`Saving…`/`Autosaving…`, `RefreshCw` spinner, info) | `correct` | `notification_stack`, first block; spinner static (deviation) |
| Toast 2 history (`warning` undo / `info` redo, `Undo2`/`Redo2`, label) | `correct` | `HistoryDirection` match |
| Toast 3 printing error (+ `4.5rem` when history/import/orientation present) | `correct` | `stacked` condition copied verbatim (gotcha 4) |
| Toast 4 scene import (tone map, `warning`→`triangle-alert`, clickable only for `openMeshRepairReport` with pointer/cursor/focus + Enter/Space) | `correct` | `SceneImportClickAction` decoration |
| Toast 5/6 export success (`Saved to: {path}`) / error | `correct` | as above |
| Toast 7 orientation (`Successfully oriented {name}` / `Already optimal for this goal.`, `4.5rem` when history/import) | `correct` | as above |
| Plate-locked, new-input-device and leaf-fanning toasts | `absent` | No counterpart anywhere in `gpui/src` (grep for `is locked` / `New input device` / `Leaf Fanning`) |
| Marquee mode toast | `absent` | Ditto (`Selection Mode: Contact/Enclosed`) |
| `EmptySceneState` | not this area | `gpui/src/panels/empty_scene.rs` (another agent's file) |
| `LayerSlider`, `CrossSliderPanel` | `absent` | No module; the port has no cross-section rail |
| `ZUpGizmoHelper`, `ZUpGizmoViewcube` | `absent` | No view cube in the port |
| `plateWidgetDraw` | `deliberate` | Re-expressed as 2D GPUI widgets at the same CSS metrics; `CSS_PIXELS_PER_WORLD_UNIT` and the world anchors belong to the viewport (module deviations) |
| `PlateNameLabel` display (name in `--accent`, placeholder muted, `fitFontToWidth` with the 8px floor, pencil chip at 55%) | `correct` | `plate_name_label`, `pencil_tile`, `fit_font_to_width` |
| `PlateNameLabel` editor (box width from characters, 53px tall, 0.9 `--surface-0`, 0.3 accent selection wash, caret at 2×45, border in accent) | `correct` | `editor` |
| Hidden `<input>`, `keepEditingRef`, caret blink | `deliberate` | Module deviation: the caller owns the draft/selection/blink (`CARET_BLINK_MS`) |
| `commit_plate_name` trim-and-compare-only-if-changed | `correct` | document-level helper |
| `PlateSideButtons`: 104px tiles, 56px icons, `5.5px` radius, locked = 85/15 surface-1·accent + accent border at 70%, disabled = 45% panel / 40% border / 55% element opacity, bin red wash 12% + 60% border, hover lift 8% white (skipped when red) | `correct` after fix | `plate_button`; **fixed** the bin's hover wash base token (`--surface-0`, not `--surface-1`) |
| Side-button wording props (`addLabel`, `*Title`) | `deliberate` | Only `add_coming_soon_title` survives as a field; a mesh carries no tooltip (source comment quoted in the spec) |
| Widget world placement / occlusion / `PLATE_WIDGET_USER_DATA` thumbnail exclusion | `absent` | No 3D plate widgets; the shapes are 2D and the caller places them |
| `PreparePanelStack` gating | `dead` → **deleted** | Nothing constructed `PrepareStack` (`grep` for the module name: only its own file, two docs and this audit). Its gating logic matched `PreparePanelStack.tsx:220-345` — `scene.geom` for transform/orientation/smoothing/split, `models.length > 0` for arrange/duplicate, `orientationPanel` for orientation — and `app.rs` already builds its own `PanelDef`s with the same gates. Removed as dead weight (fix #12); the gating now lives in `app.rs` |

---

## 7. Fixes applied (with citation)

| # | File | Change | Why |
|---|---|---|---|
| 1 | `features/organic_cut_panel.rs` | `default_cut_settings()` / `default_tenon_settings()` → `with_default_cut_settings()` / `with_default_tenon_settings()`, applied over the current state; both Reset buttons use them | The tenon Reset used a whole default state and silently reset the cut settings. Spec: transforms.md:1532-1533 (`Reset` → `setState({ ...DEFAULT_TENON_SETTINGS, generateTenon })`) and gotcha 44 (transforms.md:1840-1843); React `OrganicCutPanel.tsx:750` |
| 2 | `features/organic_cut_panel.rs` | Flip Tenon label colour `mix(ACCENT_SECONDARY, TEXT_STRONG, 0.55)` → `0.45` | Spec's `color-mix(accent-secondary, text-strong 55%)` is 45% accent (transforms.md:1529); React `OrganicCutPanel.tsx:740` |
| 3 | `features/organic_cut_panel.rs` | Tenon Reset border `mix(AXIS_X, BORDER_SUBTLE, 0.45)` → `0.55` | `#f87171` at `border-subtle 45%` = 55% red (transforms.md:1533); React `OrganicCutPanel.tsx:754` |
| 4 | `features/organic_cut_panel.rs` | `Fit Tolerance` cell: identical `if is_dome` arms collapsed | Dead branch; the dome's field is the row's only child either way |
| 5 | `features/transform_panel.rs` | Lift/Drop `ButtonSpec`: dropped the invented `.hover(8% white)` | The source's colours sit in the inline `style`, which outranks `.ui-button-secondary:hover` — the buttons have no hover fill (transforms.md:186-189; React `TransformControls.tsx:306-329`) |
| 6 | `features/transform_panel.rs` | Module deviations: tooltip wording recorded + blur→stepper commit mapping stated | Makes the two remaining differences `deliberate` per the audit contract |
| 7 | `gizmo/constants.rs`, `gizmo/mod.rs` | `RING_BACK_ALPHA` docs corrected: `0.028` = a quarter of parity `0.11` (twice halved at the user's request), not "half of it / 0.055" | Code and comments disagreed; `git show a4e92314b` confirms `0.028` is intended and the text was stale. Value unchanged |
| 8 | `features/organic_cut_panel.rs` | `tenon_fillet_title` / `loop_title` marked `#[allow(dead_code)]` with a note that they hold the recorded tooltip wording | They are unreferenced (no tooltip layer) and would otherwise warn; deleting them would drop the only place the interpolated sentences exist |
| 9 | `widgets/plate_widgets.rs` | Bin hover wash over `SURFACE_0` instead of `SURFACE_1` | Source: `fillStyle = locked ? surface1 : surface0` then the 0.12 danger wash (scene-overlays.md `paintButtonPanel`); the bin is never the locked plate's |
| 10 | `crates/gpui-ui/src/widgets/card.rs` + `features/transform_panel.rs` + `features/smoothing_panel.rs` | New `sub_card_tinted(tone, background_share, border_share)` (and `TINT_BORDER_SHARE` = 0.24 as the default `sub_card_with` keeps); Move/Brush pass 0.22, Lift 0.24, Rotate/Scale/Highlight 0.20. One `card_shell` implementation, no second card | The shared helper hard-coded the `accentCardStyle` border share for everyone; the four section cards state their own pair in the source (transforms.md:59-72, `TransformControls.tsx` `moveCardStyle`/`rotateCardStyle`/`scaleCardStyle`/`liftCardStyle`; `MeshSmoothingSettingsPanel.tsx` `brushCardStyle`/`highlightCardStyle`) |
| 11 | `widgets/orientation_panel.rs` (new), `widgets/mod.rs`, `app.rs` | The Auto Orientation panel now has a `PanelDef`: state kept in the module, actions wired, body wrapped so a click repaints. `app.rs` pushes it beside the Transform card on the source's gates | The spec has the surface (`AutoRotationPanel`), the body was ported, and nothing mounted it. `PreparePanelStack.tsx:280-283`, `page.tsx:10775` (the `auto-rotation` experiment) |
| 12 | `widgets/prepare_stack.rs` (deleted), `widgets/mod.rs` | Removed the unreferenced `PreparePanelStack` port (`grep` for `prepare_stack`/`PrepareStack` finds only its own file, two docs and this audit). Its gating is what `app.rs` already does | Dead weight: nothing constructed `PrepareStack`, and `app.rs` builds its own `PanelDef`s |

## 8. Cross-file issues (not my files — for the owners)

1. **Engines.** `features/tool_panels.rs` marks the verbs `PORT GAP: … the action only
   re-renders`: Arrange All/Selected, Duplicate Confirm/Fill Plate, Split `Cut` and
   `Snap to Edges`. The panels are correct; the buttons are `dead` until the engines land.
2. **No multi-selection wiring.** `arrange.model_count` / `selected_model_count` are
   `1/0` from the single mesh (`tool_panels.rs:411-419`), and
   `transform.selection_rotation_mode` is never `Some`, so the Group toggle cannot appear
   and "Arrange Selected" only ever tracks one model.
3. **`features/tool_panels.rs:77-81`'s note is now stale.** It says "The port has no
   experiment flag, so `orientation_body` is not stacked here; the caller can mount it
   itself" — `app.rs` now does, through `widgets/orientation_panel::orientation_panel_def`.
   The experiment read is cached in that module ([`set_experiment_enabled`]); the settings
   owner should call it when the `auto-rotation` toggle changes (see the module deviations).
4. **`panels.md:649,710` cite `gpui-ui/src/widgets/prepare_stack.rs`.** That path never
   existed — the module was `gpui/src/widgets/prepare_stack.rs`, and it is now deleted
   (its gating is in `app.rs`). Another area's spec; left for its owner.

## 9. New work (absent)

| Surface | Spec | Note |
|---|---|---|
| `RotationDial` + `rotationDialModel` + magnet (`DIAL_ANATOMY`, `DIAL_STEP_DEG`, `resolveDialAngle`, `rayToRingLocal` beyond the port's `ring_hit`) | transforms.md:744-1042 | Needs a 2D overlay layer; `layout.rs` already has `wrap_angle`/`shortest_angle_delta` and `View::ring_hit` |
| `SnapAngleReadout` (degree chip, `formatSweepDegrees`, `exceptZero`) | transforms.md:1044-1070 | 2D overlay text |
| `RotationHintTooltip` (`Drag to rotate` / `Dial marks pull at 5° / 10° / 45°`) | transforms.md:1072-1096 | 2D overlay text |
| Per-handle point lights (`GIZMO_LIGHTING`) | transforms.md:opening table | Unlit viewport today |
| `MeshShaderPreviewSlot` / `MeshShaderPreviewCanvas` | transforms.md:1100-1240 | `features/settings/mesh.rs` (another area): slot is a labelled stand-in |
| Plate-locked toast (`{plate} is locked.` / `Unlock it to add or move models.`) | scene-overlays.md "page-level toasts" | No counterpart |
| New-input-device toast (+ `Dismiss`) | ditto | No counterpart |
| Leaf-fanning toast (`Leaf Fanning Active: …`) | ditto | No counterpart |
| Marquee mode toast (`Selection Mode: Contact/Enclosed …`) | scene-overlays.md "marquee mode toast" | Also needs the marquee rect |
| `LayerSlider` / `CrossSliderPanel` | scene-overlays.md "LayerSlider" | Whole surface; `railClassName`-twice, thumb thresholds, shift latch, glide, popover side, 530 ms caret all unported |
| `ZUpGizmoHelper` / `ZUpGizmoViewcube` (+ quarter-turn arrows, home chip, arrow fade) | scene-overlays.md | Whole surface |
| 3D plate widgets (world placement, occlusion, `PLATE_WIDGET_USER_DATA` thumbnail exclusion) | scene-overlays.md "plateWidgetDraw" / call sites | The 2D widgets exist; the plane anchors do not |
| `EmptySceneState` | scene-overlays.md | Exists in `panels/empty_scene.rs` — check it in that area's audit |

## 10. Rendered but inert — the dead list

Every control in this area that draws live and does nothing. The field names are the
state they leave behind and the reason it can never arrive:

| Control | Writes | Why it has no effect |
|---|---|---|
| Transform · **Local** (×2: Move and Rotate headers) | `store.transform.local_space` (`tool_panels.rs:525-528`) | No reader. The gizmo is built from the world axes only (`gizmo/mod.rs`, "Local space is not wired"); `gizmoFrameRotation` is unported |
| Transform · **Auto** (auto-lift) | `store.transform.auto_lift` (`tool_panels.rs:531-534`) | No reader. Nothing re-lifts or preserves height after a move/rotate/scale; the source's `useAutoLift` is unported. (`auto_support_settings::auto_lift` is a *different* store and does not read this one either) |
| Transform · **Group** | would write `store.transform.selection_rotation_mode` (`tool_panels.rs:571-574`) | Cannot render: the field starts `None` and is only set from this button, so the gate `selection_rotation_mode` is never satisfied. Needs the multi-selection data model |
| Arrange · **Arrange All** / **Arrange Selected** | `mark_notify` only | `tool_panels.rs:707-711` — "PORT GAP: no arrange engine" |
| Duplicate · **Confirm Duplicate** / **Fill Plate** | `mark_notify` only | `tool_panels.rs:774-777` — "PORT GAP: no duplicate engine" |
| Split · **Cut** | `mark_notify` only | `tool_panels.rs:853-855` — "PORT GAP: no cut engine" |
| Split · **Snap to Edges** | `mark_notify` only | `tool_panels.rs:815-817` — "PORT GAP: no geometry snapping" |
| Orientation · **Orient Model** | clears `error` | No advisor (this audit's module note); the sweep is scene work |
| Orientation · **Clear** (blockers) | sets `blocked_count = 0` | No blocker store, so `blocked_count` is always 0 and the button is disabled (`busy || blocked_count == 0`) |
| Orientation · **Blockers** / **Done** | toggles `blockers_active` | The mode itself works; what it paints into does not (same missing store) |
| Smoothing · hex box / picker | `highlight_color` via the knob click only | The field is a value box, not an input, and the picker has no drag surface (`smoothing_panel.rs` deviation) |
| Support rail · **Rename / Export / Import** (presets), Islands **Scan**, Hollowing **Apply / Remove / Solid Areas / Done / Clear**, Hole Punch **Apply / Reset** | own flags | `support_panels/mod.rs` items 12, 23, 25 and `support_settings.rs` item 4 — no engines, unmounted bodies |

The same class as the support rail's `support_mode`: state without a consumer. None of
these is fixed here; several are waiting on the multi-model data model, the rest on scene
services.

## 11. Support-mode panel bodies (`features/support_panels/**`)

Neither spec in this area owns these surfaces — their sources are
`IslandsPanel.tsx`, `IslandListCard.tsx`, `AutoSupportSettingsBody.tsx`,
`HollowingPanel.tsx` and `HolePunchPanel.tsx`, and the module carries its own 25 numbered
PORT DEVIATIONS (`support_panels/mod.rs`). Audit result, for the record:

| Body | Mounted? | Verdict |
|---|---|---|
| `islands_body` | yes — Support tool 0 (`tool_panels.rs:294`) | `deliberate` — shell-fed counts, Scan starts nothing (deviation 12) |
| `support_settings_body` | yes — Support tool 1 (`tool_panels.rs:300`) | `deliberate` — preset strip + Density/Stability only; Rename/Export/Import inert (18-22) |
| `hollowing_body` | yes — Support rail's Hollowing (`app.rs:1977`) | `deliberate` — no engine (23-24) |
| `hole_punch_body` | **no** — no call site | `absent` from the app (`tool_panels.rs`'s own note asks the caller to mount it) |
| `island_list_body` | **no** — no call site | `absent` from the app |
| `island_scan_settings_dialog` | **no** — no call site | `absent` from the app |
| `studio_presets` | data module (read by `support_settings_body` and `auto_support_settings`) | n/a |

No wrong/dead verdicts beyond the recorded gaps: the three mounted bodies are substantive,
their inert verbs are the documented engine gaps, and the three unmounted bodies are the
shell's to place.

## 12. Notes

- **`Local`, `Auto` and `Group`** are the three controls that look live and are not; all
  three are in §10 with the field each one leaves behind.
- **`mix(a, b, t)` = `a·t + b·(1−t)`** in this port, which is why the spec's
  `color-mix(A, B N%)` always appears as `mix(tone, TOKEN, 1 − N)`. Every ratio in this
  document was read that way; the one place it had drifted is fix #2.
- The smoothing picker's knob click (30° hue step) is the only interactive affordance in
  this area that exists in neither the spec nor the React source. It is documented in the
  file; a product decision is needed if it should go.
