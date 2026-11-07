use gpui::{
    App, ClickEvent, Div, FontWeight, IntoElement, Stateful, WeakEntity, Window, div, prelude::*,
    px, rgb, rgba,
};

use gpui_core::icons::icon;
use gpui_core::support_state::*;
use gpui_core::theme::*;
use gpui_ui::panel_fields::{WHITE, with_alpha};
use gpui_ui::ui::{button, spinner_colored};

use super::shared::{
    ISLAND_INTERSECTION, ISLAND_MINIMA, ISLAND_OVERHANG, ISLAND_VOXEL, ROUNDED_BARE, flat_button,
    hint, hint_full, notify, panel_icon_button, section_card,
};
use crate::DragonFruit;

// ---------------------------------------------------------------------------
// Islands — `IslandsPanel.tsx`
// ---------------------------------------------------------------------------
pub fn islands_body(
    state: &IslandsPanelState,
    entity: WeakEntity<DragonFruit>,
) -> impl IntoElement {
    let model = state.model();
    let has_geometry = model.has_geometry;
    let scanning = model.scanning;
    let applying_settings = model.applying_settings;
    let has_data = model.has_data();
    let voxel_total = model.voxel_total;
    let minima_total = model.minima_total;
    let coincident_total = model.coincident_total;
    let island_count = model.islands.len();
    let selected_index = model.selected_index;
    let current_label = selected_index
        .and_then(|index| model.islands.get(index))
        .cloned();
    let advanced_mode = model.advanced_mode;
    let show_voxel_only = model.show_voxel_only;
    let show_minima_only = model.show_minima_only;
    let show_intersection = model.show_intersection;
    let show_overhangs = model.show_overhangs;
    drop(model);

    // Header right slot: `IconButton className="!p-0.5"` with a 14px gear.
    let gear_state = state.clone();
    let gear_entity = entity.clone();
    let gear = hint(
        "islands-settings-tip",
        panel_icon_button("islands-settings", "settings", 14.0, 2.0, move |_, _, cx| {
            gear_state.model_mut().settings_open = true;
            notify(&gear_entity, cx);
        }),
        "Scan settings",
    );

    // `Button size="auto"` at `w-full !h-8 text-[11px] disabled:opacity-50` with
    // the accent inline style.
    let scan_entity = entity.clone();
    let scan_button = flat_button(
        "islands-scan",
        if scanning { "Scanning…" } else { "Scan Islands" },
        32.0,
        mix(ACCENT, SURFACE_0, 0.14),
        ACCENT,
        ACCENT,
        None,
        has_geometry && !scanning,
        move |_, _, cx| {
            // PORT GAP: no island scanner — the run belongs to the shell.
            notify(&scan_entity, cx);
        },
    )
    .w_full();

    // Breakdown by type: `grid grid-cols-3 gap-2 text-center`.
    let breakdown = section_card().child(
        div()
            .flex()
            .gap(px(8.0))
            .child(island_stat("Voxels", voxel_total, ISLAND_VOXEL))
            .child(island_stat("Minima", minima_total, ISLAND_MINIMA))
            .child(island_stat("Coincident", coincident_total, ISLAND_INTERSECTION)),
    );

    // Navigation: `flex items-center gap-2`, two `flex-1 h-8` arrows around the
    // counter.
    let prev_state = state.clone();
    let prev_entity = entity.clone();
    let next_state = state.clone();
    let next_entity = entity.clone();
    let can_prev = island_count > 0 && selected_index.is_some_and(|index| index > 0);
    // `disabled={orderedIslands.length === 0 || selectedIndex >= orderedIslands.length - 1}`:
    // with nothing selected the source's index is -1, so Next is live.
    let can_next = island_count > 0
        && selected_index.is_none_or(|index| index + 1 < island_count);
    let nav_counter = div()
        .flex_1()
        .min_w(px(0.0))
        .flex()
        .items_center()
        .justify_center()
        .gap(px(4.0))
        .child(
            div()
                .text_size(px(13.0))
                .font_weight(FontWeight::BOLD)
                .text_color(rgb(TEXT_STRONG))
                .child(match &current_label {
                    Some(label) => label.to_string(),
                    None => island_count.to_string(),
                }),
        )
        .when(selected_index.is_some(), |this| {
            this.child(
                div()
                    .text_size(px(10.0))
                    .text_color(rgb(TEXT_MUTED))
                    .child(format!("/ {island_count}")),
            )
        });
    let navigation = section_card().child(
        div()
            .flex()
            .items_center()
            .gap(px(8.0))
            .child(nav_arrow(
                "islands-prev",
                "chevron-left",
                can_prev,
                move |_, _, cx| {
                    let mut model = prev_state.model_mut();
                    let count = model.islands.len();
                    model.selected_index = match model.selected_index {
                        Some(index) if index > 0 => Some(index - 1),
                        Some(_) => Some(0),
                        None if count > 0 => Some(0),
                        None => None,
                    };
                    drop(model);
                    notify(&prev_entity, cx);
                },
            ))
            .child(nav_counter)
            .child(nav_arrow(
                "islands-next",
                "chevron-right",
                can_next,
                move |_, _, cx| {
                    let mut model = next_state.model_mut();
                    let count = model.islands.len();
                    model.selected_index = match model.selected_index {
                        Some(index) if index + 1 < count => Some(index + 1),
                        None if count > 0 => Some(0),
                        other => other,
                    };
                    drop(model);
                    notify(&next_entity, cx);
                },
            )),
    );

    // Display toggles, advanced mode only: `grid grid-cols-2 gap-1.5`.
    let toggle_row = |a: Stateful<Div>, b: Stateful<Div>| {
        div().flex().gap(px(6.0)).child(div().flex_1().min_w(px(0.0)).child(a)).child(
            div().flex_1().min_w(px(0.0)).child(b),
        )
    };
    let display_toggles = section_card()
        .flex()
        .flex_col()
        .gap(px(6.0))
        .child(toggle_row(
            layer_toggle(
                "islands-toggle-voxels",
                "Voxels",
                show_voxel_only,
                ISLAND_VOXEL,
                "Slicing islands and suspended areas detected from layer contours",
                {
                    let state = state.clone();
                    let entity = entity.clone();
                    move |next, _, cx| {
                        state.model_mut().show_voxel_only = next;
                        notify(&entity, cx);
                    }
                },
            ),
            layer_toggle(
                "islands-toggle-minima",
                "Minima",
                show_minima_only,
                ISLAND_MINIMA,
                "Individual lowest-vertex triangles on the mesh surface",
                {
                    let state = state.clone();
                    let entity = entity.clone();
                    move |next, _, cx| {
                        state.model_mut().show_minima_only = next;
                        notify(&entity, cx);
                    }
                },
            ),
        ))
        .child(toggle_row(
            layer_toggle(
                "islands-toggle-coincident",
                "Coincident",
                show_intersection,
                ISLAND_INTERSECTION,
                "Regions where both voxel and geometric islands overlap",
                {
                    let state = state.clone();
                    let entity = entity.clone();
                    move |next, _, cx| {
                        state.model_mut().show_intersection = next;
                        notify(&entity, cx);
                    }
                },
            ),
            layer_toggle(
                "islands-toggle-overhangs",
                "Overhangs",
                show_overhangs,
                ISLAND_OVERHANG,
                "Shallow surfaces detected by the mesh-normal classifier (the surfaces auto-supports grid)",
                {
                    let state = state.clone();
                    let entity = entity.clone();
                    move |next, _, cx| {
                        state.model_mut().show_overhangs = next;
                        notify(&entity, cx);
                    }
                },
            ),
        ));

    // Empty state: `rounded-md border px-3 py-3 text-center text-[11px]
    // leading-snug` on `color-mix(surface-1, transparent 8%)`.
    let empty_state = div()
        .rounded(px(ROUNDED_MD))
        .border_1()
        .border_color(rgb(BORDER_SUBTLE))
        .bg(rgba(with_alpha(SURFACE_1, 0xeb)))
        .px(px(12.0))
        .py(px(12.0))
        .text_center()
        .text_size(px(11.0))
        .line_height(px(11.0 * 1.375))
        .text_color(rgb(TEXT_MUTED))
        .child("Scan the model to detect unsupported islands");

    // `applyingSettings` overlay: `absolute inset-0 … rounded-md` on
    // `color-mix(surface-0 60%, transparent)`.
    let overlay = div()
        .absolute()
        .top(px(0.0))
        .left(px(0.0))
        .right(px(0.0))
        .bottom(px(0.0))
        .flex()
        .flex_col()
        .items_center()
        .justify_center()
        .rounded(px(ROUNDED_MD))
        .bg(rgba(with_alpha(SURFACE_0, 0x99)))
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(8.0))
                .px(px(12.0))
                .py(px(8.0))
                .rounded(px(ROUNDED_MD))
                .border_1()
                .border_color(rgb(BORDER_SUBTLE))
                .bg(rgb(SURFACE_1))
                .shadow_md()
                .child(spinner_colored(16.0, ACCENT))
                .child(
                    div()
                        .text_size(px(11.0))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(rgb(TEXT_STRONG))
                        .child("Recalculating…"),
                ),
        );

    // `px-2.5 pt-1 pb-3 space-y-2.5`.
    div()
        .relative()
        .px(px(10.0))
        .pt(px(4.0))
        .pb(px(12.0))
        .flex()
        .flex_col()
        .gap(px(10.0))
        .child(div().flex().justify_end().child(gear))
        .child(scan_button)
        .when(has_data && !scanning, |this| {
            this.child(breakdown)
                .child(navigation)
                .when(advanced_mode, |this| this.child(display_toggles))
        })
        .when(!has_data && !scanning && has_geometry, |this| {
            this.child(empty_state)
        })
        .when(applying_settings, |this| this.child(overlay))
}

/// One type count: a 6px dot and a 9px uppercase label over a 13px bold value.
fn island_stat(label: &'static str, count: usize, color: u32) -> Div {
    div()
        .flex_1()
        .min_w(px(0.0))
        .flex()
        .flex_col()
        .items_center()
        .child(
            div()
                .flex()
                .items_center()
                .justify_center()
                .gap(px(4.0))
                .mb(px(2.0))
                .child(div().w(px(6.0)).h(px(6.0)).flex_shrink_0().rounded_full().bg(rgb(color)))
                .child(
                    div()
                        .text_size(px(9.0))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(rgb(TEXT_MUTED))
                        .truncate()
                        .child(label.to_uppercase()),
                ),
        )
        .child(
            div()
                .text_size(px(13.0))
                .font_weight(FontWeight::BOLD)
                .text_color(rgb(TEXT_STRONG))
                .child(count.to_string()),
        )
}

/// One nav arrow: `flex-1 h-8 rounded border flex items-center justify-center`,
/// `disabled:opacity-40`.
fn nav_arrow(
    id: &'static str,
    icon_name: &'static str,
    enabled: bool,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> Stateful<Div> {
    let mut el = div()
        .id(id)
        .flex_1()
        .h(px(32.0))
        .flex()
        .items_center()
        .justify_center()
        .rounded(px(ROUNDED_BARE))
        .border_1()
        .border_color(rgb(BORDER_SUBTLE))
        .bg(rgb(SURFACE_0))
        .text_color(rgb(TEXT_STRONG))
        .child(icon(icon_name, 14.0, TEXT_STRONG));
    if enabled {
        el = el.cursor_pointer().hover(|style| style.bg(rgb(SURFACE_2))).on_click(on_click);
    } else {
        el = el.opacity(0.4).cursor_not_allowed();
    }
    el
}

/// `ToggleBtn`: `min-h-[36px] w-full rounded-md border px-2 text-[11px]
/// font-semibold uppercase tracking-wide`, tinted by its layer colour when on.
fn layer_toggle(
    id: &'static str,
    label: &'static str,
    checked: bool,
    color: u32,
    hint_text: &'static str,
    on_change: impl Fn(bool, &mut Window, &mut App) + 'static,
) -> Stateful<Div> {
    let (background, border, text) = if checked {
        (
            mix(color, SURFACE_1, 0.16),
            mix(color, WHITE, 0.90),
            mix(color, TEXT_STRONG, 0.75),
        )
    } else {
        (SURFACE_1, BORDER_SUBTLE, TEXT_MUTED)
    };
    let button = div()
        .id(id)
        .min_h(px(36.0))
        .w_full()
        .rounded(px(ROUNDED_MD))
        .border_1()
        .border_color(rgb(border))
        .bg(rgb(background))
        .px(px(8.0))
        .text_size(px(11.0))
        .font_weight(FontWeight::SEMIBOLD)
        .text_color(rgb(text))
        .flex()
        .items_center()
        .justify_center()
        .gap(px(6.0))
        .cursor_pointer()
        .child(label.to_uppercase())
        .on_click(move |_: &ClickEvent, window: &mut Window, cx: &mut App| {
            on_change(!checked, window, cx)
        });
    hint_full(format!("{id}-tip"), button, hint_text)
}
