//! The canvas right-click menu — `src/components/ui/EditorContextMenu.tsx`
//! over `src/components/ui/ContextMenu.tsx`.
//!
//! `MENU_ITEMS` is the source's top-level list: the clipboard block, then the
//! three flyouts (`Repair`, `Supports`, `Geometry`). The menu's action
//! vocabulary (`EditorMenuAction`) is what `page.tsx`'s
//! `handleEditorMenuAction` switches on; this build can perform only `delete`,
//! so every other row closes the menu and does nothing, documented at its arm
//! in [`select`] rather than faked.
//!
//! # PORT DEVIATIONS
//!
//! * **Heading.** `page.tsx` passes `editorContextMenuTitle`: `Editor` in
//!   Prepare, `Supports` in Support. The menu is built from a pointer position
//!   and the entity alone, and GPUI has no way to read entity state while an
//!   element tree is being built, so the heading is omitted. The rows are the
//!   source's either way.
//! * **Flyout side.** `ContextMenu.tsx` measures each flyout and flips it left
//!   at the window edge. Element builders cannot measure, so every flyout opens
//!   right (`ui::FlyoutSide::Right`), the source's pre-layout state.
//! * **Clamping.** The source measures the menu and calls `clampToViewport`.
//!   `anchored().snap_to_window_with_margin(8)` is the window-side equivalent:
//!   the menu keeps `EDGE_MARGIN_PX` clear of every edge without a measure pass.
//! * **Dismissal.** `useOutsideDismiss` / `useEscapeToClose` live in the
//!   parent; `app.rs` owns `editor_menu` and clears it on canvas press and on
//!   Escape. Every row also clears it here, as `handleEditorMenuAction`'s
//!   trailing `closeEditorContextMenu()` does.
//! * **Disabled rows.** The source derives `disabledActions` from the scene
//!   (selection, paste buffer, repair report). None of that state is readable
//!   from [`editor_menu`]'s arguments, so no row is disabled; the unported rows
//!   are no-ops instead (see [`select`]).
//! * **`aria-*` / `role`** — no accessible-name attribute in this port.

use gpui::{
    App, IntoElement, MouseButton, WeakEntity, anchored, deferred, div, point, prelude::*, px,
};

use gpui_ui::ui::{ContextMenuEntry, FlyoutSide, context_menu_placed};

use crate::DragonFruit;

/// `ContextMenu.tsx`'s default `widthClassName` (`w-48`). The flyouts reuse the
/// menu's width, as the source's `widthClassName` does.
const MENU_WIDTH: f32 = 192.0;

/// `EDGE_MARGIN_PX` in `ContextMenu.tsx`: the gap the clamped menu keeps clear
/// of the window edge.
const EDGE_MARGIN: f32 = 8.0;

/// The editor canvas menu at `menu` (window-space pixels of the gesture), or
/// nothing when no menu is open.
///
/// The surface, rows, grouping rules and flyouts are `ui::context_menu_placed`;
/// this function owns the entry list, the placement and the actions.
pub fn editor_menu(menu: Option<(f32, f32)>, entity: WeakEntity<DragonFruit>) -> impl IntoElement {
    let Some((x, y)) = menu else {
        // `ContextMenu` renders `null` for a `null` position.
        return div().into_any_element();
    };

    let entries = menu_entries();
    let surface = context_menu_placed(
        &entries,
        None,
        MENU_WIDTH,
        // `SubmenuFlyout` flips left only after measuring; see the module docs.
        FlyoutSide::Right,
        move |id, _window, cx| select(&entity, id, cx),
    );

    // `ContextMenu` portals to `document.body`; `deferred` is GPUI's equivalent
    // and the priority lifts the menu above the rail's menus (priority 0), whose
    // tooltips it may overlap.
    deferred(
        anchored()
            .position(point(px(x), px(y)))
            // `clampToViewport(position, size, { margin: EDGE_MARGIN_PX })`.
            .snap_to_window_with_margin(px(EDGE_MARGIN))
            .child(
                div()
                    // Clicks inside the menu must not reach the canvas' orbit
                    // or grab handlers.
                    .occlude()
                    .w(px(MENU_WIDTH))
                    .child(surface)
                    // The parent starts camera gestures on the canvas; a press on
                    // the menu is not one.
                    .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                    .on_mouse_down(MouseButton::Right, |_, _, cx| cx.stop_propagation()),
            ),
    )
    .with_priority(2)
    .into_any_element()
}

/// `MENU_ITEMS` — the editor menu's rows, labels, icons and groups.
///
/// Labels are the source's Lingui descriptors resolved for `en`; the icons are
/// the same lucide slugs.
pub fn menu_entries() -> Vec<ContextMenuEntry> {
    vec![
        // Selection / clipboard.
        ContextMenuEntry::new("delete", "Delete").with_icon("trash-2"),
        ContextMenuEntry::new("cut", "Cut").with_icon("scissors"),
        ContextMenuEntry::new("copy", "Copy").with_icon("copy"),
        ContextMenuEntry::new("paste", "Paste").with_icon("clipboard-paste"),
        // Mesh repair, fanned out. `page.tsx` relabels the first row "Repair
        // Again" and adds the undo/report rows once the active model has a
        // repair; this build has no repair pass, so the source's base shape is
        // the one drawn.
        ContextMenuEntry::new("repair-menu", "Repair")
            .with_icon("wrench")
            .starts_group()
            .with_children([
                ContextMenuEntry::new("repair", "Auto-Repair").with_icon("wrench"),
                ContextMenuEntry::new("remesh", "Remesh").with_icon("pyramid"),
                ContextMenuEntry::new("shrinkwrap", "Shrink-Wrap").with_icon("refresh-cw"),
            ]),
        // Support scaffolding and geometry designation, each fanned out so the
        // top level stays short.
        ContextMenuEntry::new("supports-menu", "Supports")
            .with_icon("blocks")
            .starts_group()
            .with_children([
                ContextMenuEntry::new("split-supports", "Split supports").with_icon("split"),
                ContextMenuEntry::new("merge-supports", "Merge supports").with_icon("link"),
                ContextMenuEntry::new("scan-for-supports", "Scan for Supports").with_icon("search"),
            ]),
        ContextMenuEntry::new("geometry-menu", "Geometry")
            .with_icon("shapes")
            .with_children([
                ContextMenuEntry::new("mark-as-support-geometry", "Mark as Support")
                    .with_icon("life-buoy"),
                ContextMenuEntry::new("mark-as-model-geometry", "Mark as Model").with_icon("box"),
            ]),
    ]
}

/// `handleEditorMenuAction`: run the row's action, then close the menu.
///
/// Only `delete` has an implementation in this build. Every other arm is an
/// explicit no-op: the source action needs a scene graph this port does not
/// have (clipboard, support split/merge/scan, geometry designation) or a pass it
/// does not run (mesh repair, remesh, shrink-wrap). They close the menu, as the
/// source does after any action, but change nothing else.
fn select(entity: &WeakEntity<DragonFruit>, id: &str, cx: &mut App) {
    entity
        .update(cx, |this, cx| {
            // `closeEditorContextMenu()` at the end of the source handler.
            this.editor_menu = None;

            match id {
                // `dispatchDeleteModelAction(..., scene.deleteModels)`, gated on
                // `hasTargetModel`.
                "delete" => {
                    if this.mesh.is_some() {
                        this.delete_model();
                    }
                }

                // No-op: the port has no model clipboard, so there is nothing
                // to cut, copy or paste (`scene.canPasteModel` is always false).
                "cut" | "copy" | "paste" => {}

                // No-op: `MeshRepairModals`' manual repair flow
                // (`setManualRepairModelIds`) needs the repair pipeline, which
                // this build does not run.
                "repair" => {}

                // No-op: `TetrahedralRemeshConfirmModal` /
                // `ShrinkWrapConfirmModal` confirm a mesh rebuild that has no
                // implementation here.
                "remesh" | "shrinkwrap" => {}

                // No-op: `scene.splitSupports` / `scene.mergeSupports` /
                // `scene.scanModelForSupportsInPlace` edit a live support graph;
                // the port only draws the `SUPP` document a scene file carried.
                "split-supports" | "merge-supports" | "scan-for-supports" => {}

                // No-op: `scene.toggleSupportDesignation` has no counterpart in
                // the single-mesh view.
                "mark-as-support-geometry" | "mark-as-model-geometry" => {}

                // A flyout parent's id is never passed to the selection callback
                // (`context_menu_placed` sends the leaf's id only); anything
                // else is a row this build does not know.
                _ => {}
            }

            cx.notify();
        })
        .ok();
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::SharedString;

    /// `&*SharedString` — the rows compare as text, not as their small-string
    /// internals.
    fn text(value: &SharedString) -> &str {
        value
    }

    fn row(entry: &ContextMenuEntry) -> (&str, &str, bool, Vec<&str>) {
        (
            text(&entry.id),
            text(&entry.label),
            entry.starts_group,
            entry.children.iter().map(|child| text(&child.id)).collect(),
        )
    }

    /// The rows, their labels, their grouping rules and each flyout's children
    /// are `MENU_ITEMS`.
    #[test]
    fn entries_follow_menu_items() {
        let entries = menu_entries();
        let rows: Vec<(&str, &str, bool, Vec<&str>)> = entries.iter().map(row).collect();

        assert_eq!(
            rows,
            vec![
                ("delete", "Delete", false, vec![]),
                ("cut", "Cut", false, vec![]),
                ("copy", "Copy", false, vec![]),
                ("paste", "Paste", false, vec![]),
                (
                    "repair-menu",
                    "Repair",
                    true,
                    vec!["repair", "remesh", "shrinkwrap"]
                ),
                (
                    "supports-menu",
                    "Supports",
                    true,
                    vec!["split-supports", "merge-supports", "scan-for-supports"]
                ),
                (
                    "geometry-menu",
                    "Geometry",
                    false,
                    vec!["mark-as-support-geometry", "mark-as-model-geometry"]
                ),
            ]
        );
    }

    /// A flyout parent runs no action, so its id never reaches [`select`]; the
    /// ids the callback can receive are the leaves.
    #[test]
    fn leaf_ids_are_the_selectable_rows() {
        let mut leaves: Vec<String> = Vec::new();
        for entry in menu_entries() {
            if entry.children.is_empty() {
                leaves.push(text(&entry.id).to_owned());
            } else {
                leaves.extend(
                    entry
                        .children
                        .iter()
                        .map(|child| text(&child.id).to_owned()),
                );
            }
        }
        assert_eq!(
            leaves,
            vec![
                "delete",
                "cut",
                "copy",
                "paste",
                "repair",
                "remesh",
                "shrinkwrap",
                "split-supports",
                "merge-supports",
                "scan-for-supports",
                "mark-as-support-geometry",
                "mark-as-model-geometry",
            ]
        );
        assert!(!leaves.iter().any(|id| id.ends_with("-menu")));
    }

    /// Every icon the menu names is a generated lucide slug (`assets/icons`).
    #[test]
    fn icons_are_registered_assets() {
        fn walk(entries: &[ContextMenuEntry], found: &mut Vec<String>) {
            for entry in entries {
                if let Some(icon) = &entry.icon {
                    found.push(text(icon).to_owned());
                }
                walk(&entry.children, found);
            }
        }

        let mut found = Vec::new();
        walk(&menu_entries(), &mut found);

        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/icons");
        for name in found {
            assert!(
                root.join(format!("{name}.svg")).is_file(),
                "missing icon asset {name}.svg"
            );
        }
    }
}
