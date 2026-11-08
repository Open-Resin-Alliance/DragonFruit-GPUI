//! Panel bodies ported from the React panels.
//!
//! Sources: `src/components/controls/ModelManagerPanel.tsx`,
//! `src/components/controls/AutoSupportPanel.tsx` (through
//! `autoSupport/AutoSupportPresetRow.tsx` and `AutoSupportPresetSelect.tsx`),
//! `src/features/export/components/ExportPanel.tsx` and
//! `src/features/printing/components/PrintingPanel.tsx`.
//!
//! Only the body is ported: the panel shell, its collapse chevron and its
//! context-menu host belong to the panel module.
//!
//! # PORT DEVIATIONS
//!
//! ## Shared
//!
//! 1. `panels.rs::card_header` has no right slot, so the Models panel's plus is
//!    rendered at the top of the body, right-aligned, where the header slot
//!    would have put it. The Auto Supports gear has no destination (its dialog
//!    is not ported) and the Export panel's "Meshes" badge has no body
//!    equivalent, so neither is rendered.
//! 2. GPUI has no CSS transitions, `backdrop-blur`, portals or `letter-spacing`:
//!    hover/active fills apply instantly, popovers are absolute children of their
//!    trigger, and `tracking-wide` labels are not tracked.
//! 3. `space-y-*` is a flex `gap` on the same container.
//! 4. Tooltips use [`hint`], a local popover. `ui::tooltip` constrains its
//!    popover to the trigger's width, which would wrap an icon button's label
//!    into a 28px column; GPUI's native tooltip needs a view builder and a
//!    window context these bodies do not receive. [`hint`] is a fixed 200px box
//!    hanging from the trigger's right edge, where the source's portal centres
//!    an auto-width box under the trigger and clamps it to the viewport.
//! 5. Tailwind radii: bare `rounded` is 4px, `rounded-md` is 10px (the overrides
//!    in `globals.css`), `rounded-sm` on `.ui-button` is 6px.
//!
//! ## Models — `ModelManagerPanel.tsx`
//!
//! 6. The app holds one mesh (`DragonFruit::mesh`), not a model list: one row,
//!    no build plates, no import groups and no multi-body 3MF. `body_row`,
//!    `folder_row` and `models_context_menu` are ported but unreachable; the
//!    `rounded-md border p-1` wrapper that groups a folder's rows is described
//!    on `folder_row` and not built.
//! 7. No per-model visibility: the eye button is inert and always renders `eye`.
//! 8. No details dialog: the info button is inert. Its tooltip carries the
//!    source's `${size} · ${polygons}` reduced to the polygon clause, because
//!    `stl::MeshData` has no file size.
//! 9. No rename state: the row renders the read-only name only.
//! 10. No delete: the trash button is inert.
//! 11. No context-menu state (`PanelContextMenuState`): the menu cannot open.
//! 12. Row click: the source calls `onSelect(id, 'single')`; the app's selection
//!     is one boolean, so the row sets it true and the viewport's hit test can
//!     clear it. Shift-range and ctrl-toggle have no multi-model equivalent.
//! 13. The row has no second line: the previous port invented a triangle line,
//!     which the source does not have. The count lives in the info tooltip.
//! 14. The list's `overflow-y-auto` is not applied: the card grows to its content
//!     in this shell, so a scroll container would collapse.
//! 15. The inert row buttons swallow their clicks (`stopPropagation` in the
//!     source), so pressing one does not select the model.
//!
//! ## Auto Supports — `AutoSupportPanel.tsx`
//!
//! 16. No island scanner: Voxel and Minima are 0, Total is the generated support
//!     count the caller passes (not `filteredIslands`), and the `…` scanning
//!     state never renders.
//! 17. No busy flag: the run button never shows "Running…" and never takes
//!     `disabled:opacity-50`.
//! 18. No preset store and no auto-lift flag: the preset row renders the
//!     no-preset trigger label and the Auto-Lift OFF state, both inert.
//! 19. The header's settings gear is not rendered: the full-screen
//!     `AutoSupportSettingsBody` dialog is not ported, so the button would have
//!     no destination.
//! 20. No debug mode, diagnostics or forest report: not rendered.
//! 21. The hint's condition is `!hasGeometry` in the source; the body only
//!     receives the support count, so it is keyed on `count == 0`.
//! 22. `canRun` also needs `hasGeometry` and `activeModelId`, neither of which
//!     reaches this body: the button is always enabled.
//!
//! ## Export — `ExportPanel.tsx`
//!
//! The action reports through the shell's toast layer, which is the source's
//! `exportSuccessToast` / `exportErrorToast` (`Saved to: <path>` / the error).
//!
//! 23. `export_body` receives no model list, so the source's
//!     "No meshes loaded yet…" empty state cannot be selected; the full panel
//!     always renders.
//! 24. The app writes STL only (`stl::export_stl`, binary): VOXL and 3MF render
//!     in the source's disabled-option style, and so does ASCII encoding. The
//!     source leaves all of them selectable.
//! 25. No model picker under "Active Model" (no per-model selection state) and
//!     no plate-scope row (one plate).
//! 26. Layout renders the single-plate STL pair: the `plates` value relabelled
//!     "Bundle" by the source, and "Separate". Separate is inert.
//! 27. The include rows are display-only at the React defaults (all on): the STL
//!     writer emits the mesh only.
//! 28. No exporting state: no spinner, no "Exporting…", no disabled action.
//! 29. The save dialog suggests `export.stl`; the source suggests the plate or
//!     model base name, which this shell has no naming module for.
//!
//! ## Printing — `PrintingPanel.tsx`
//!
//! 30. No slicer, printer or profile state: every value is the source's
//!     empty-state label ("No printer selected", "No resin selected", "—",
//!     "No generated file yet", "— • —").
//! 31. Download and Send to Printer render in the `.ui-button:disabled` recipe
//!     (`canDownload` / `canSendToPrinter` are false).
//! 32. No saved-path card, send-target picker, cancel/retry action, UVTools row
//!     or send status: none of their inputs exist.
//! 33. The previous port's "Slice a model to inspect layers." hint is not in
//!     `PrintingPanel.tsx`; it is removed.

use gpui::{
    BoxShadow, Div, FontWeight, IntoElement, SharedString, Stateful, WeakEntity, div, prelude::*,
    px, relative, rgb, rgba,
};

use gpui_core::icons::icon;
use gpui_core::theme::*;
use gpui_ui::ui::{
    ButtonSize, ButtonSpec, ButtonVariant, ContextMenuEntry, IconButtonSize, IconButtonVariant,
    IconTone, SettingDensity, SettingSurface, button_with, button_with_content, context_menu,
    icon_button_toned, setting_row, sub_card, sub_card_with, toggle,
};
use crate::DragonFruit;
use gpui_core::stl;

/// `#ffffff` — the ghost hover and the `color-mix(..., white 86%)` borders.
const WHITE: u32 = 0xffffff;
/// `#000000` — the base of every `color-mix(..., black N%)` in the sources.
const BLACK: u32 = 0x000000;

/// Tailwind's bare `rounded`: 4px.
const RADIUS_BARE: f32 = 4.0;

// ---------------------------------------------------------------------------
// Shared pieces
// ---------------------------------------------------------------------------

/// The source's `Tooltip`, in the one shape this shell can draw: an absolutely
/// positioned popover hanging from its trigger's right edge, revealed by the
/// trigger's hover.
///
/// `ui::tooltip` is not reused because its popover is `left-0 right-0` on the
/// trigger, and every trigger here is a 28px icon button — the label would wrap
/// into a 28px column. The fixed width keeps the box inside the panel.
fn hint(
    id: impl Into<SharedString>,
    trigger: impl IntoElement,
    text: impl Into<SharedString>,
) -> Stateful<Div> {
    let group: SharedString = id.into();
    div()
        .id(group.clone())
        .relative()
        .flex()
        .group(group.clone())
        .child(trigger)
        .child(
            div()
                .absolute()
                .right(px(0.))
                .top(relative(1.))
                .mt(px(28.))
                .w(px(200.))
                .opacity(0.)
                .group_hover(group, |style| style.opacity(1.))
                .rounded(px(RADIUS_BARE))
                .px(px(8.))
                .py(px(6.))
                .text_size(px(11.))
                .line_height(px(11. * LINE_HEIGHT_TIGHT))
                .font_weight(FontWeight::MEDIUM)
                .text_color(rgb(TEXT_STRONG))
                .bg(rgba(0x181818fa))
                .border_1()
                .border_color(rgb(ACCENT))
                .whitespace_normal()
                .shadow_lg()
                .child(text.into()),
        )
}

/// `IconButton variant="ghost"`: a bare hit target that shows `bg-white/10` on
/// hover, `rounded` and `!p-1` (4px) at `size="sm"`.
///
/// The source's row buttons all `stopPropagation` so pressing one never selects
/// the model; these are inert, so the click is swallowed and nothing else.
fn ghost_icon(id: &'static str, icon_name: &'static str, size: f32) -> Stateful<Div> {
    icon_button_toned(
        id,
        icon_name,
        size,
        IconButtonVariant::Ghost,
        IconButtonSize::Sm,
        IconTone::Neutral,
        false,
    )
    .on_click(|_, _, cx| cx.stop_propagation())
}

/// `rounded-md border p-2` on `--border-subtle` / `--surface-1` — the neutral
/// sub-card (`panelCardStyle` in the React panels).
fn section_card() -> Div {
    sub_card().border_color(rgb(BORDER_SUBTLE))
}

/// `grid grid-cols-2 gap-1.5`.
fn grid2(a: impl IntoElement, b: impl IntoElement) -> Div {
    div().flex().gap(px(6.)).child(a).child(b)
}

/// `grid grid-cols-3 gap-1.5`.
fn grid3(a: impl IntoElement, b: impl IntoElement, c: impl IntoElement) -> Div {
    div().flex().gap(px(6.)).child(a).child(b).child(c)
}

// ---------------------------------------------------------------------------
// Models — `ModelManagerPanel.tsx`
// ---------------------------------------------------------------------------

/// The Models body: the add button, the model row and the empty state.
pub fn models_body(
    mesh_name: String,
    triangles: usize,
    selected: bool,
    entity: WeakEntity<DragonFruit>,
) -> impl IntoElement {
    // `text-xs text-center py-2 italic`, muted.
    let empty = || {
        div()
            .text_size(px(FONT_XS))
            .text_center()
            .py(px(8.))
            .italic()
            .text_color(rgb(TEXT_MUTED))
            .child("No models loaded")
    };

    // `space-y-1 overflow-y-auto pr-0.5 flex-1 min-h-0`. With one ungrouped
    // model `buildModelGroups` yields a single entry with no header, so the list
    // is the row alone.
    let list = div()
        .flex()
        .flex_col()
        .gap(px(4.))
        .when(triangles == 0, |this| this.child(empty()))
        .when(triangles > 0, |this| {
            this.child(model_row(mesh_name, triangles, selected, entity))
        });

    // `px-2.5 pt-1 pb-2.5 space-y-2 flex flex-col flex-1 min-h-0`.
    div()
        .px(px(10.))
        .pt(px(4.))
        .pb(px(10.))
        .flex()
        .flex_col()
        .gap(px(8.))
        .child(list)
}

/// `CardHeader`'s right slot: the Models panel's add button.
pub fn models_add_button(entity: WeakEntity<DragonFruit>) -> impl IntoElement {
    let add_entity = entity;
    hint(
        "models-add-tip",
        button_with_content(
            "models-add",
            // `IconButton size="sm"` with `<Plus className="h-3.5 w-3.5" />`
            // (14px), but the 2px stroke at that size reads thin next to the
            // 28px tile, so the glyph is drawn at the tile's own optical size.
            // `IconButton size="sm"` is `h-8 w-8 !p-0` with a `h-3.5 w-3.5`
            // glyph.
            &ButtonSpec::new(ButtonVariant::Secondary, ButtonSize::Sm).icon_tile(32.0),
            icon("plus", 14.0, TEXT_MUTED),
        )
        .on_click(move |_, _, cx| {
            add_entity.update(cx, |this, cx| this.open_stl(cx)).ok();
        }),
        "Add models to the plate",
    )
}

/// One model's row, as `renderModelRow` builds it for an ungrouped model.
///
/// `px-2 py-1 rounded border transition-colors flex items-center gap-2
/// cursor-pointer`, with the selected fills from the row's inline style.
fn model_row(
    name: String,
    triangles: usize,
    selected: bool,
    entity: WeakEntity<DragonFruit>,
) -> impl IntoElement {
    let click_entity = entity.clone();

    // `min-w-0 text-sm font-medium truncate`, wrapped in the source's
    // `Tooltip content={model.name} fullWidth`.
    let name_trigger = div()
        .flex_1()
        .min_w(px(0.))
        .truncate()
        .text_size(px(FONT_MD))
        .font_weight(FontWeight::MEDIUM)
        .text_color(rgb(TEXT_STRONG))
        .child(name.clone());
    let name_cell = div()
        .flex_1()
        .min_w(px(0.))
        .child(hint("model-name-tip", name_trigger, name));

    // `formatModelInfoTooltip`: `${size} · ${polygons}`, or the polygons alone
    // when there is no size. `MeshData` has no file size, so only the polygon
    // clause survives.
    let polygons = if triangles == 1 {
        "1 polygon".to_owned()
    } else {
        format!("{triangles} polygons")
    };

    div()
        .id("model-row")
        .flex()
        .items_center()
        .gap(px(8.))
        .px(px(8.))
        .py(px(4.))
        .rounded(px(RADIUS_BARE))
        .border_1()
        .border_color(rgb(if selected {
            // `color-mix(in srgb, var(--accent), var(--border-subtle) 40%)`.
            mix(ACCENT, BORDER_SUBTLE, 0.60)
        } else {
            BORDER_SUBTLE
        }))
        .bg(rgb(if selected {
            // `color-mix(in srgb, var(--accent), var(--surface-1) 92%)`.
            mix(ACCENT, SURFACE_1, 0.08)
        } else {
            SURFACE_1
        }))
        .cursor_pointer()
        .child(name_cell)
        .child(
            // `flex items-center gap-0.5`.
            div()
                .flex()
                .items_center()
                .gap(px(2.))
                // PORT GAP: no details dialog — the info button is inert.
                .child(hint(
                    "model-info-tip",
                    ghost_icon("model-info", "info", 14.),
                    polygons,
                ))
                // PORT GAP: no per-model visibility — the eye is inert and the
                // model is always visible.
                .child(hint(
                    "model-eye-tip",
                    ghost_icon("model-eye", "eye", 16.),
                    "Hide",
                ))
                // PORT GAP: deleting a model is not implemented.
                .child(hint(
                    "model-delete-tip",
                    ghost_icon("model-delete", "trash-2", 14.),
                    "Delete model",
                )),
        )
        .on_click(move |_, _, cx| {
            // `onSelect(model.id, 'single')`: the app's selection is one flag.
            click_entity
                .update(cx, |this, cx| {
                    this.selected.set(true);
                    cx.notify();
                })
                .ok();
        })
}

/// One row of a multi-body 3MF's drawer: `flex items-center justify-between gap-2
/// rounded border px-2 py-1 text-sm cursor-pointer transition-colors`, the body's
/// colour swatch and "Body N" on the left, its polygon count on the right.
///
/// Unreachable: `stl::MeshData` has no `splitBodies`, so no model row carries the
/// drawer chevron (`IconButton` ghost sm, `!-ml-1 shrink-0`, chevron at 3.5) that
/// opens the `ml-1.5 space-y-1 pl-1` indent these rows live in.
#[allow(dead_code)]
fn body_row(index: usize, polygons: usize, color: Option<u32>, selected: bool) -> impl IntoElement {
    // `formatPolygonCountCompact` steps to thousands at 1 000 and millions at
    // 1 000 000 through the locale formatter; this shell has no locale
    // formatter, so the count is rendered in full.
    let count = format!("{polygons}");

    let mut left = div().flex().min_w(px(0.)).items_center().gap(px(6.));
    if let Some(color) = color {
        // `h-3.5 w-3.5 shrink-0 rounded-[4px]` with
        // `box-shadow: inset 0 0 0 1px rgba(0, 0, 0, 0.28)`.
        left = left.child(
            div()
                .w(px(14.))
                .h(px(14.))
                .flex_shrink_0()
                .rounded(px(RADIUS_BARE))
                .bg(rgb(color))
                .shadow(vec![
                    BoxShadow::new(px(0.), px(0.), rgba(0x00000047).into())
                        .blur_radius(px(0.))
                        .spread_radius(px(1.))
                        .inset(),
                ]),
        );
    }
    left = left.child(
        div()
            .truncate()
            .font_weight(FontWeight::MEDIUM)
            .text_color(rgb(if selected { TEXT_STRONG } else { TEXT_MUTED }))
            .child(format!("Body {}", index + 1)),
    );

    div()
        .flex()
        .items_center()
        .justify_between()
        .gap(px(8.))
        .rounded(px(RADIUS_BARE))
        .border_1()
        .border_color(rgb(if selected {
            // `color-mix(in srgb, var(--accent), var(--border-subtle) 40%)`.
            mix(ACCENT, BORDER_SUBTLE, 0.60)
        } else {
            BORDER_SUBTLE
        }))
        .bg(rgb(if selected {
            // `color-mix(in srgb, var(--accent), var(--surface-1) 92%)`.
            mix(ACCENT, SURFACE_1, 0.08)
        } else {
            SURFACE_2
        }))
        .px(px(8.))
        .py(px(4.))
        .text_size(px(FONT_MD))
        .cursor_pointer()
        .child(left)
        .child(
            // `shrink-0 text-[11px] tabular-nums`.
            div()
                .flex_shrink_0()
                .text_size(px(11.))
                .text_color(rgb(TEXT_MUTED))
                .child(count),
        )
}

/// Which glyph and tint a folder header wears.
#[allow(dead_code)]
#[derive(Clone, Copy, PartialEq, Eq)]
enum FolderKind {
    /// A normal import group: folder / folder-open in the accent.
    Folder,
    /// A build plate: `LayoutGrid` in the accent.
    Plate,
    /// The automatic "Outside plate" folder: `AlertTriangle` in `#ff7c88`.
    System,
}

/// How much of a folder is selected, which is what tints its header.
#[allow(dead_code)]
#[derive(Clone, Copy, PartialEq, Eq)]
enum FolderSelection {
    None,
    Partial,
    Full,
}

/// A folder header: `px-1.5 py-1 rounded border flex items-center gap-1.5
/// cursor-pointer transition-colors`, the chevron button, the glyph, the
/// uppercase name and the model count.
///
/// Unreachable: the app's single mesh has no import group, so the Models body
/// renders the ungrouped row path. The `space-y-1 rounded-md border p-1`
/// wrapper that groups a folder's rows is part of the same unreachable path.
#[allow(dead_code)]
fn folder_row(
    name: &str,
    count: usize,
    collapsed: bool,
    kind: FolderKind,
    selection: FolderSelection,
) -> impl IntoElement {
    let (background, border) = match selection {
        // `color-mix(in srgb, var(--accent), var(--surface-2) 90%)` /
        // `color-mix(in srgb, var(--accent), var(--border-subtle) 45%)`.
        FolderSelection::Full => (
            mix(ACCENT, SURFACE_2, 0.10),
            mix(ACCENT, BORDER_SUBTLE, 0.55),
        ),
        // `color-mix(in srgb, var(--accent), var(--surface-2) 94%)` /
        // `color-mix(in srgb, var(--accent), var(--border-subtle) 30%)`.
        FolderSelection::Partial => (
            mix(ACCENT, SURFACE_2, 0.06),
            mix(ACCENT, BORDER_SUBTLE, 0.70),
        ),
        FolderSelection::None => (SURFACE_2, BORDER_SUBTLE),
    };

    let glyph = match kind {
        FolderKind::Folder => {
            if collapsed {
                icon("folder", 14., ACCENT)
            } else {
                icon("folder-open", 14., ACCENT)
            }
        }
        FolderKind::Plate => icon("layout-grid", 14., ACCENT),
        FolderKind::System => icon("triangle-alert", 14., 0xff7c88),
    };

    div()
        .px(px(6.))
        .py(px(4.))
        .flex()
        .items_center()
        .gap(px(6.))
        .rounded(px(RADIUS_BARE))
        .border_1()
        .border_color(rgb(border))
        .bg(rgb(background))
        .cursor_pointer()
        // `inline-flex items-center justify-center rounded p-0.5
        // hover:bg-black/20`, chevron at `w-3 h-3`.
        .child(
            div()
                .flex()
                .items_center()
                .justify_center()
                .rounded(px(RADIUS_BARE))
                .p(px(2.))
                .hover(|style| style.bg(rgba(0x00000033)))
                .child(icon(
                    if collapsed {
                        "chevron-right"
                    } else {
                        "chevron-down"
                    },
                    12.,
                    TEXT_MUTED,
                )),
        )
        .child(glyph)
        .child(
            // `text-[10px] font-semibold uppercase tracking-wide truncate`.
            div()
                .flex_1()
                .min_w(px(0.))
                .text_size(px(10.))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(rgb(TEXT_MUTED))
                .truncate()
                .child(name.to_uppercase()),
        )
        .child(
            // `ml-auto text-[10px] tabular-nums`.
            div()
                .text_size(px(10.))
                .text_color(rgb(TEXT_MUTED))
                .child(format!("{count}")),
        )
}

/// The model row's context menu, with the entries `ModelManagerPanel` builds for
/// a model target and no multi-selection.
///
/// Unreachable: the app has no context-menu state to open it from. The group
/// section (`showGroupSection`) and the folder/plate sections only appear for a
/// folder or plate target or a multi-selection, none of which a one-mesh scene
/// can produce; `Repair mesh…` and `Split to bodies` need handlers the app does
/// not have.
#[allow(dead_code)]
fn models_context_menu() -> Div {
    let mut rename = ContextMenuEntry::new("rename-model", "Rename model");
    rename.icon = Some("pencil".into());
    rename.starts_group = true;

    let mut actions = ContextMenuEntry::new("model-actions", "Model actions…");
    actions.icon = Some("box".into());

    context_menu(
        &[rename, actions],
        Some("Models".into()),
        |_, _, _| {},
    )
}

// ---------------------------------------------------------------------------
// Auto Supports (Beta) — `AutoSupportPanel.tsx`
// ---------------------------------------------------------------------------

/// The Auto Supports body: the run button, the island counts, the preset row
/// and the load-a-model hint.
///
/// PORT GAP: the header's settings gear has no destination — the full-screen
/// `AutoSupportSettingsBody` dialog is not ported — so it is not rendered.
pub fn supports_body(count: usize, entity: WeakEntity<DragonFruit>) -> impl IntoElement {
    let generate_entity = entity.clone();

    // `Button variant="secondary" size="auto"` at `w-full !h-8 text-[11px]`
    // with the accent inline style: border accent, accent at 14% over
    // surface-0, accent text. The inline background wins over the variant's
    // hover fill, so there is no hover change.
    //
    // PORT GAP: no busy flag, so the label never becomes "Running…", and no
    // `hasGeometry` / `activeModelId`, so `canRun` cannot disable it.
    let mut run_spec = ButtonSpec::new(ButtonVariant::Secondary, ButtonSize::Sm).full();
    run_spec.hover = Some(mix(ACCENT, SURFACE_0, 0.14));
    let run_button = button_with("supports-generate", "Generate Supports", &run_spec)
        .border_color(rgb(ACCENT))
        .bg(rgb(mix(ACCENT, SURFACE_0, 0.14)))
        .text_color(rgb(ACCENT))
        .on_click(move |_, _, cx| {
            generate_entity
                .update(cx, |this, cx| {
                    if let Some(mesh) = this.mesh.as_ref() {
                        let model = glam::Mat4::from_scale_rotation_translation(
                            glam::Vec3::splat(this.scale),
                            glam::Quat::from_rotation_z(this.rotate_z),
                            glam::Vec3::from_array(this.translate),
                        );
                        let points = stl::overhang_points(mesh, this.support_angle);
                        let target = (300.0 * this.support_density).max(1.0) as usize;
                        let stride = (points.len() / target).max(1);
                        *this.supports.borrow_mut() = points
                            .into_iter()
                            .step_by(stride)
                            .map(|p| model.transform_point3(glam::Vec3::from_array(p)).to_array())
                            .collect();
                    }
                    cx.notify();
                })
                .ok();
        });

    // `rounded-md border p-2` with AUTO_SUPPORT_SECTION_CARD, then
    // `grid grid-cols-3 gap-2 text-center`.
    //
    // PORT GAP: no island scanner — Voxel and Minima are 0, Total is the
    // generated support count, and the `…` scanning state never renders.
    let counts = div()
        .rounded(px(ROUNDED_MD))
        .border_1()
        .border_color(rgb(BORDER_SUBTLE))
        .bg(rgb(SURFACE_1))
        .p(px(8.))
        .flex()
        .gap(px(8.))
        .child(island_stat("Voxel", "0", false))
        .child(island_stat("Minima", "0", false))
        .child(island_stat("Total", &count.to_string(), true));

    // `AutoSupportPresetRow`: `flex gap-1.5`, the preset select at `flex-1
    // min-w-0`, the Auto-Lift button beside it at `flex-1 min-w-0`.
    //
    // PORT GAP: the app has neither a preset store nor an auto-lift flag, so the
    // trigger shows the no-preset label and the button shows OFF; both are inert.
    let preset_row = div()
        .flex()
        .gap(px(6.))
        .child(div().flex_1().min_w(px(0.)).child(hint(
            "supports-preset-tip",
            preset_select_trigger(),
            "Apply this preset to the auto-support settings",
        )))
        .child(div().flex_1().min_w(px(0.)).child(hint(
            "supports-lift-tip",
            // `Button variant="secondary" size="auto"` at
            // `min-w-0 flex-1 !h-8 px-2 text-[11px]` with OFF_STYLE
            // (`background: var(--surface-1)`).
            {
                let mut lift_spec = ButtonSpec::new(ButtonVariant::Secondary, ButtonSize::Sm);
                lift_spec.hover = Some(SURFACE_1);
                button_with("supports-auto-lift", "Auto-Lift OFF", &lift_spec)
                    .flex_1()
                    .min_w(px(0.))
                    .truncate()
            },
            "Lift the selected models to this distance so a support can stand under \
             them; off returns them to the height they were at.",
        )));

    // `text-[10px] italic text-center`, muted.
    let hint_text = div()
        .text_size(px(10.))
        .italic()
        .text_center()
        .text_color(rgb(TEXT_MUTED))
        .child("Load a model and scan for islands.");

    // `px-2.5 pb-3 space-y-2.5`.
    div()
        .px(px(10.))
        .pb(px(12.))
        .flex()
        .flex_col()
        .gap(px(10.))
        .child(run_button)
        .child(counts)
        .child(preset_row)
        // PORT GAP: no debug mode, so `AutoSupportRunDiagnostics` never renders.
        .when(count == 0, |this| this.child(hint_text))
}

/// One island count: uppercase 10px label over a 14px bold value; the total
/// carries the accent.
fn island_stat(label: &'static str, value: &str, accent: bool) -> impl IntoElement {
    div()
        .flex_1()
        .flex()
        .flex_col()
        .text_center()
        .child(
            div()
                .text_size(px(10.))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(rgb(TEXT_MUTED))
                .child(label.to_uppercase()),
        )
        .child(
            div()
                .text_size(px(FONT_MD))
                .font_weight(FontWeight::BOLD)
                .text_color(rgb(if accent { ACCENT } else { TEXT_STRONG }))
                .child(value.to_owned()),
        )
}

/// `AutoSupportPresetSelect`'s trigger: `.ui-input` at `rounded-[4px] w-full
/// !h-8 text-[12px] leading-tight`, `background: surface-1`, centred label with
/// 26px of right padding for the 14px chevron at `right-3`.
fn preset_select_trigger() -> Div {
    div()
        .relative()
        .w_full()
        .h(px(32.))
        .rounded(px(RADIUS_BARE))
        .border_1()
        .border_color(rgb(BORDER_SUBTLE))
        .bg(rgb(SURFACE_1))
        .pr(px(26.))
        .flex()
        .items_center()
        .justify_center()
        .text_size(px(FONT_XS))
        .line_height(px(FONT_XS * 1.25))
        .text_color(rgb(TEXT_STRONG))
        .child(
            div()
                .min_w(px(0.))
                .truncate()
                .child("Custom — not a preset"),
        )
        .child(
            div()
                .absolute()
                .right(px(12.))
                .top(px(0.))
                .bottom(px(0.))
                .flex()
                .items_center()
                .child(icon("chevron-down", 14., TEXT_MUTED)),
        )
}

// ---------------------------------------------------------------------------
// Export — `ExportPanel.tsx`
// ---------------------------------------------------------------------------

/// Which theme colour an active option button carries.
#[derive(Clone, Copy, PartialEq, Eq)]
enum OptionTone {
    /// `activeOptionStyle`: the accent, for scope, STL encoding and plates.
    Accent,
    /// `activeSecondaryOptionStyle`: the secondary accent, for format and
    /// layout.
    AccentSecondary,
}

/// The Export body: the scope picker, the format and layout pickers, the
/// include rows and the primary action.
///
/// PORT GAP: `export_body` receives no model list, so the source's
/// "No meshes loaded yet…" empty state cannot be selected; the full panel always
/// renders. The header's "Meshes" badge has no body equivalent.
pub fn export_body(entity: WeakEntity<DragonFruit>) -> impl IntoElement {
    let export_entity = entity.clone();

    // `grid grid-cols-2 gap-1.5` of `EXPORT_SCOPE_OPTIONS`, in the
    // `rounded-md border p-2 space-y-1.5` card.
    //
    // PORT GAP: no per-model selection state, so the "Active Model" select
    // (`Select` over `modelOptions`) is not rendered.
    let scope = section_card().child(grid2(
        option_button(
            "export-scope-plate",
            "Entire Plate",
            true,
            false,
            OptionTone::Accent,
        ),
        option_button(
            "export-scope-model",
            "Active Model",
            false,
            false,
            OptionTone::Accent,
        ),
    ));

    // PORT GAP: the plate-scope card only exists with more than one plate.

    // `grid grid-cols-3 gap-1.5` of `EXPORT_FORMAT_OPTIONS`, then the STL
    // encoding row when STL is the format.
    //
    // PORT GAP: `stl::export_stl` writes binary STL only, so VOXL and 3MF use
    // the source's disabled-option style and ASCII encoding does too. The source
    // leaves all of them selectable.
    let formats = section_card()
        .child(grid3(
            option(
                "export-format-voxl",
                "VOXL",
                false,
                true,
                OptionTone::AccentSecondary,
                "VOXL Scene (.voxl)",
            ),
            option(
                "export-format-3mf",
                "3MF",
                false,
                true,
                OptionTone::AccentSecondary,
                "3MF Mesh (.3mf)",
            ),
            option(
                "export-format-stl",
                "STL",
                true,
                false,
                OptionTone::AccentSecondary,
                "STL Mesh (.stl)",
            ),
        ))
        .child(grid2(
            option(
                "export-encoding-binary",
                "Binary",
                true,
                false,
                OptionTone::Accent,
                "Binary STL (recommended)",
            ),
            option(
                "export-encoding-ascii",
                "ASCII",
                false,
                true,
                OptionTone::Accent,
                "ASCII STL",
            ),
        ));

    // `layoutOptions` for STL on one plate: the `plates` value relabelled
    // "Bundle" (one file holding everything on the plate) and "Separate".
    //
    // PORT GAP: a per-model directory export does not exist, so "Separate" is
    // inert; it keeps the source's inactive fill.
    let layout = section_card().child(grid2(
        option(
            "export-layout-bundle",
            "Bundle",
            true,
            false,
            OptionTone::AccentSecondary,
            "One file holding everything on the plate",
        ),
        option(
            "export-layout-separate",
            "Separate",
            false,
            false,
            OptionTone::AccentSecondary,
            "One file per model",
        ),
    ));

    // `SettingRow as="label" bordered surface="raised"` at the React defaults.
    //
    // PORT GAP: the STL writer emits the mesh only, so these are display-only.
    let include_rows = div()
        .flex()
        .flex_col()
        .gap(px(6.))
        .child(setting_row(
            "Include Model Mesh",
            None,
            SettingDensity::Compact,
            true,
            SettingSurface::Raised,
            false,
            toggle(true),
        ))
        .child(setting_row(
            "Include Supports",
            None,
            SettingDensity::Compact,
            true,
            SettingSurface::Raised,
            false,
            toggle(true),
        ))
        .child(setting_row(
            "Include Raft",
            None,
            SettingDensity::Compact,
            true,
            SettingSurface::Raised,
            false,
            toggle(true),
        ));

    // `space-y-1.5 border-t pt-2` around the primary action.
    let action = div()
        .flex()
        .flex_col()
        .gap(px(8.))
        .child(div().h(px(1.)).w_full().bg(rgb(BORDER_SUBTLE)))
        .child(
            // `Button variant="accent"` at `w-full gap-1.5`: `.ui-button-accent`
            // is the secondary-button surface, the download glyph and the
            // format's label.
            //
            // PORT GAP: no exporting state — no spinner and no "Exporting…".
            div()
                .id("export-run")
                .w_full()
                .h(px(36.))
                .flex()
                .items_center()
                .justify_center()
                .gap(px(6.))
                .rounded(px(RADIUS_SM))
                .border_1()
                .border_color(rgb(mix(SECONDARY_BUTTON_SURFACE, WHITE, 0.86)))
                .bg(rgb(SECONDARY_BUTTON_SURFACE))
                .px(px(11.))
                .text_size(px(FONT_XS))
                .font_weight(FontWeight::SEMIBOLD)
                .line_height(px(FONT_XS))
                .text_color(rgb(ACCENT_SECONDARY_CONTRAST))
                .cursor_pointer()
                .hover(|style| style.bg(rgb(mix(SECONDARY_BUTTON_SURFACE, BLACK, 0.91))))
                .child(icon("download", 16., ACCENT_SECONDARY_CONTRAST))
                .child("Export as STL")
                .on_click(move |_, _, cx| {
                    export_entity
                        .update(cx, |this, cx| {
                            let Some(mesh) = this.mesh.as_ref() else {
                                return;
                            };
                            let Some(path) = rfd::FileDialog::new()
                                .add_filter("STL", &["stl"])
                                .set_file_name("export.stl")
                                .save_file()
                            else {
                                return;
                            };
                            let transform = glam::Mat4::from_scale_rotation_translation(
                                glam::Vec3::splat(this.scale),
                                glam::Quat::from_rotation_z(this.rotate_z),
                                glam::Vec3::from_array(this.translate),
                            );
                            // The source reports through `exportSuccessToast` /
                            // `exportErrorToast`; the shell owns the toast layer.
                            // A failure logs the error and toasts a fixed message.
                            let (text, tone) = match stl::export_stl(mesh, transform, &path) {
                                Ok(()) => (
                                    format!("Saved to: {}", path.display()),
                                    gpui_ui::ui::ToastTone::Success,
                                ),
                                Err(error) => {
                                    eprintln!("Export failed: {error:#}");
                                    (
                                        "Export failed. Check console for details.".to_owned(),
                                        gpui_ui::ui::ToastTone::Error,
                                    )
                                }
                            };
                            this.show_toast(text, tone, cx);
                        })
                        .ok();
                }),
        );

    // `px-2.5 pt-1 pb-2.5 space-y-2`.
    div()
        .px(px(10.))
        .pt(px(4.))
        .pb(px(10.))
        .flex()
        .flex_col()
        .gap(px(8.))
        .child(scope)
        .child(formats)
        .child(layout)
        .child(include_rows)
        .child(action)
}

/// One picker button in its grid cell, with the source's `Tooltip` title.
///
/// The wrapper carries the cell's `flex-1 min-w-0` so the button keeps the grid
/// geometry while the popover hangs from the cell.
fn option(
    id: &'static str,
    label: &'static str,
    active: bool,
    disabled: bool,
    tone: OptionTone,
    title: &'static str,
) -> impl IntoElement {
    div()
        .flex_1()
        .min_w(px(0.))
        .child(hint(
            format!("{id}-tip"),
            option_button(id, label, active, disabled, tone),
            title,
        ))
}

/// A picker button: `.ui-button-secondary` at `!h-8 px-1.5 text-[11px]
/// whitespace-nowrap`, with the panel's active tint or its inert option style.
///
/// The inline fill the source puts on every one of these wins over the variant's
/// hover fill, so none of them changes on hover.
fn option_button(
    id: &'static str,
    label: &'static str,
    active: bool,
    disabled: bool,
    tone: OptionTone,
) -> Stateful<Div> {
    let (background, border, text, opacity) = if disabled {
        // `disabledOptionStyle`.
        (SURFACE_0, BORDER_SUBTLE, TEXT_MUTED, 0.6)
    } else if active {
        let (accent, weight) = match tone {
            OptionTone::Accent => (ACCENT, 0.15),
            OptionTone::AccentSecondary => (ACCENT_SECONDARY, 0.15),
        };
        // `color-mix(in srgb, <accent>, var(--surface-1) 85%)` and
        // `color-mix(in srgb, <accent>, var(--border-subtle) 30%)`.
        (
            mix(accent, SURFACE_1, weight),
            mix(accent, BORDER_SUBTLE, 0.70),
            TEXT_STRONG,
            1.0,
        )
    } else {
        (SURFACE_0, BORDER_SUBTLE, TEXT_STRONG, 1.0)
    };

    // The inline fill the source puts on every one of these wins over the
    // variant's hover fill, so none of them changes on hover: the spec's hover
    // is pinned to the resting fill.
    //
    // Inside a panel `.ui-panel .ui-button { font-size: 14px }` is unlayered
    // CSS, so the source's `text-[11px]` never applies — only `!h-8` does.
    let mut spec = ButtonSpec::new(ButtonVariant::Secondary, ButtonSize::Sm);
    spec.hover = Some(background);
    if disabled {
        spec = spec.disabled();
    }

    button_with(id, label, &spec)
        .flex_1()
        .min_w(px(0.))
        .bg(rgb(background))
        .border_color(rgb(border))
        .text_color(rgb(text))
        .opacity(opacity)
        .whitespace_nowrap()
        .when(disabled, |this| this.cursor_not_allowed())
}

// ---------------------------------------------------------------------------
// Printing — `PrintingPanel.tsx`
// ---------------------------------------------------------------------------

/// The Printing body: the printer/resin/estimate card, the generated-file card
/// and the action column.
///
/// PORT GAP: the app has no slicer, printer or profile state, so every value is
/// the source's empty-state label and both actions are disabled.
pub fn printing_body(_entity: WeakEntity<DragonFruit>) -> impl IntoElement {
    // `rounded-md border p-2.5 space-y-1`: flat label/value pairs, with `mt-1`
    // on every label after the first.
    let settings = sub_card_with(None, 10.0)
        .border_color(rgb(BORDER_SUBTLE))
        .gap(px(4.0))
        .child(print_label("Printer", false))
        .child(print_value("No printer selected"))
        .child(print_label("Resin", true))
        .child(print_value("No resin selected"))
        .child(print_label("Estimated print time", true))
        .child(print_value("—"))
        .child(print_label("Estimated volume", true))
        .child(print_value("—"));

    // `rounded-md border p-2.5 space-y-1`.
    let generated = sub_card_with(None, 10.0)
        .border_color(rgb(BORDER_SUBTLE))
        .gap(px(4.0))
        .child(print_label("Generated file", false))
        .child(
            div()
                .text_size(px(FONT_MD))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(rgb(TEXT_STRONG))
                .truncate()
                .child("No generated file yet"),
        )
        .child(
            // `{outputFormat ?? '—'} • {outputSizeLabel}`.
            div()
                .text_size(px(FONT_XS))
                .text_color(rgb(TEXT_MUTED))
                .child("— • —"),
        );

    // `grid grid-cols-1 gap-2`.
    let actions = div()
        .flex()
        .flex_col()
        .gap(px(8.))
        .child(disabled_accent_action(
            "printing-download",
            "download",
            "Export as file",
        ))
        .child(
            // `flex items-center gap-1.5`; the send button is `flex-1` at
            // `!h-9 ... text-[12px]`.
            //
            // PORT GAP: no printer — `canSendToPrinter` is false, and the
            // send-target picker and the busy cancel/retry action have no
            // state. The UVTools row needs a saved path.
            div().flex().items_center().gap(px(6.)).child(disabled_secondary_action(
                "printing-send",
                "printer",
                "Send to Printer",
            )),
        );

    // `px-3 pb-3 space-y-2.5`.
    div()
        .px(px(12.))
        .pb(px(12.))
        .flex()
        .flex_col()
        .gap(px(10.))
        .child(settings)
        .child(generated)
        .child(actions)
}

/// One printing field label: `text-xs` muted, optionally `mt-1`.
fn print_label(label: &'static str, spaced: bool) -> Div {
    div()
        .text_size(px(FONT_XS))
        .text_color(rgb(TEXT_MUTED))
        .when(spaced, |this| this.mt(px(4.)))
        .child(label)
}

/// One printing field value: `text-sm font-semibold` strong.
fn print_value(value: &'static str) -> Div {
    div()
        .text_size(px(FONT_MD))
        .font_weight(FontWeight::SEMIBOLD)
        .text_color(rgb(TEXT_STRONG))
        .child(value)
}

/// `.ui-button:disabled` with the accent variant's geometry: `!h-9`, the glyph
/// and the label at `gap-1.5`.
fn disabled_accent_action(
    id: &'static str,
    icon_name: &'static str,
    label: &'static str,
) -> Stateful<Div> {
    let spec = ButtonSpec::new(ButtonVariant::Accent, ButtonSize::Md)
        .full()
        .gap(6.0)
        .disabled()
        .icon(icon_name, 16.0, mix(TEXT_MUTED, SURFACE_2, 0.82));
    button_with(id, label, &spec)
}

/// `.ui-button:disabled` with the secondary variant's geometry: `!h-9 flex-1`,
/// the glyph and the label at `gap-1.5`.
fn disabled_secondary_action(
    id: &'static str,
    icon_name: &'static str,
    label: &'static str,
) -> Stateful<Div> {
    let spec = ButtonSpec::new(ButtonVariant::Secondary, ButtonSize::Md)
        .gap(6.0)
        .disabled()
        .icon(icon_name, 16.0, mix(TEXT_MUTED, SURFACE_2, 0.82));
    button_with_content(
        id,
        &spec,
        div()
            .min_w(px(0.))
            .truncate()
            .whitespace_nowrap()
            .child(label),
    )
    .flex_1()
    .min_w(px(0.))
}
