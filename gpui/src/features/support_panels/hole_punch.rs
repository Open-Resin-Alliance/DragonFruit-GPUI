use gpui::{FontWeight, IntoElement, WeakEntity, div, prelude::*, px, rgb, rgba};

use gpui_core::icons::icon;
use gpui_core::support_state::*;
use gpui_core::theme::*;
use gpui_ui::panel_fields::{WHITE, format_number, with_alpha};
use gpui_ui::ui::{self, ButtonSize, ButtonSpec, ButtonVariant, spinner_colored};

use super::shared::{
    ROUNDED_BARE, accent_secondary_action_border, clamp_float, labeled_section_card, mode_pill,
    notify, panel_body, ui_meta,
};
use crate::DragonFruit;

// ---------------------------------------------------------------------------
// Hole punching — `HolePunchPanel.tsx`
// ---------------------------------------------------------------------------
/// The Hole Punching body: the interior-view hint, the depth mode, the punch
/// depth, the two hole diameters and the Reset / Apply row.
///
/// PORT GAP: no punching engine — Apply and Reset only flip the panel's flags.
pub fn hole_punch_body(
    state: &HolePunchState,
    entity: WeakEntity<DragonFruit>,
) -> impl IntoElement {
    let model = state.model();
    let radius_mm = model.radius_mm;
    let effective_radius_y_mm = model.radius_y_mm.unwrap_or(model.radius_mm);
    let depth_mm = model.depth_mm;
    let depth_mode = model.depth_mode;
    let linked = model.linked;
    let can_use_auto_depth = model.can_use_auto_depth;
    let is_applying = model.is_applying;
    let can_apply = model.can_apply;
    let can_reset = model.can_reset;
    let disabled = model.disabled;
    let interior_view = model.interior_view;
    let interior_view_available = model.interior_view_available;
    drop(model);

    // `disabledStyle`: `opacity: 0.45` (the `grayscale(0.7)` has no GPUI
    // equivalent).
    let body_opacity = if disabled { 0.45 } else { 1.0 };

    // Interior View Mode card: the accent-secondary fill.
    let interior_card = ui::sub_card_with(Some((ACCENT_SECONDARY, 0.08)), 8.0)
        .border_color(rgb(accent_secondary_action_border()))
        .min_h(px(72.0))
        .items_center()
        .justify_center()
        .text_center()
        .child(
            div()
                .text_size(px(FONT_SM))
                .text_color(rgb(ACCENT_SECONDARY))
                .child("Interior View Mode"),
        )
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(6.0))
                .child(
                    div()
                        .text_size(px(10.0))
                        .text_color(rgb(TEXT_MUTED))
                        .child("Press"),
                )
                .child(
                    div()
                        .px(px(4.0))
                        .rounded(px(ROUNDED_BARE))
                        .bg(rgb(SURFACE_2))
                        .text_size(px(FONT_XS))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(rgb(ACCENT_SECONDARY))
                        .child("X"),
                )
                .child(
                    div()
                        .text_size(px(10.0))
                        .text_color(rgb(TEXT_MUTED))
                        .child("for"),
                )
                .child(
                    div()
                        .text_size(px(10.0))
                        .font_weight(FontWeight::BOLD)
                        .text_color(rgb(ACCENT_SECONDARY))
                        .child(if interior_view {
                            "Exterior View"
                        } else {
                            "Interior View"
                        }),
                ),
        );

    // Depth Mode: Auto / Manual.
    let auto_state = state.clone();
    let auto_entity = entity.clone();
    let manual_state = state.clone();
    let manual_entity = entity.clone();
    let depth_card = ui::sub_card_with(Some((ACCENT, 0.05)), 8.0)
        .child(ui_meta("Depth Mode"))
        .child(
            div()
                .flex()
                .gap(px(4.0))
                .child(div().flex_1().child(mode_pill(
                    "hole-punch-auto",
                    "Auto",
                    depth_mode == HolePunchDepthMode::Auto,
                    !(disabled || is_applying || !can_use_auto_depth),
                    move |_, _, cx| {
                        auto_state.model_mut().depth_mode = HolePunchDepthMode::Auto;
                        notify(&auto_entity, cx);
                    },
                )))
                .child(div().flex_1().child(mode_pill(
                    "hole-punch-manual",
                    "Manual",
                    depth_mode == HolePunchDepthMode::Manual,
                    !(disabled || is_applying),
                    move |_, _, cx| {
                        manual_state.model_mut().depth_mode = HolePunchDepthMode::Manual;
                        notify(&manual_entity, cx);
                    },
                ))),
        );

    // Punch Depth: `clampFloat(value, 1, 120, 1)`.
    let depth_disabled = disabled || is_applying;
    let depth_value = depth_mm;
    let depth_state = state.clone();
    let depth_entity = entity.clone();
    let depth_state_down = state.clone();
    let depth_entity_down = entity.clone();
    let depth_field_card = labeled_section_card("Punch Depth").child(ui::scrollable_number_field(
        "hole-punch-depth",
        format_number(depth_mm),
        Some("mm"),
        false,
        depth_disabled,
        depth_value < 120.0,
        depth_value > 1.0,
        move |_, _, cx| {
            depth_state.model_mut().depth_mm =
                clamp_float((depth_value - 0.5).max(1.0), 1.0, 120.0, 1);
            notify(&depth_entity, cx);
        },
        move |_, _, cx| {
            depth_state_down.model_mut().depth_mm =
                clamp_float((depth_value + 0.5).min(120.0), 1.0, 120.0, 1);
            notify(&depth_entity_down, cx);
        },
    ));

    // Hole Diameter: the link toggle, then the X and Y diameters.
    let link_state = state.clone();
    let link_entity = entity.clone();
    let link_toggle = div()
        .id("hole-punch-link")
        .w(px(20.0))
        .h(px(20.0))
        .flex()
        .items_center()
        .justify_center()
        .rounded(px(ROUNDED_BARE))
        .when(!(disabled || is_applying), |this| {
            this.cursor_pointer()
                .hover(|style| style.bg(rgba(with_alpha(WHITE, 0x1a))))
        })
        .child(icon(
            if linked { "link" } else { "unlink" },
            14.0,
            if linked { ACCENT } else { TEXT_MUTED },
        ))
        .when(!(disabled || is_applying), |this| {
            this.on_click(move |_, _, cx| {
                let mut model = link_state.model_mut();
                model.linked = !model.linked;
                if model.linked {
                    model.radius_y_mm = None;
                } else {
                    // Initialize Y to the current X so it stays independent.
                    model.radius_y_mm = Some(model.radius_mm);
                }
                drop(model);
                notify(&link_entity, cx);
            })
        });

    let x_state = state.clone();
    let x_entity = entity.clone();
    let x_state_down = state.clone();
    let x_entity_down = entity.clone();
    let y_state = state.clone();
    let y_entity = entity.clone();
    let y_state_down = state.clone();
    let y_entity_down = entity.clone();
    let diameter_disabled = disabled || is_applying;
    let diameter_x = radius_mm * 2.0;
    let diameter_y = effective_radius_y_mm * 2.0;
    let diameter_card = ui::sub_card()
        .gap(px(8.0))
        .child(
            div()
                .flex()
                .items_center()
                .justify_between()
                .child(
                    div()
                        .text_size(px(FONT_SM))
                        .text_color(rgb(TEXT_MUTED))
                        .child("Hole Diameter"),
                )
                .child(link_toggle),
        )
        .child(ui::scrollable_number_field(
            "hole-punch-diameter-x",
            format_number(diameter_x),
            Some("mm"),
            false,
            diameter_disabled,
            diameter_x < 40.0,
            diameter_x > 0.4,
            move |_, _, cx| {
                let next_radius =
                    clamp_float((diameter_x - 0.1).max(0.4) * 0.5, 0.2, 20.0, 2);
                let mut model = x_state.model_mut();
                model.radius_mm = next_radius;
                if model.linked {
                    model.radius_y_mm = None;
                }
                drop(model);
                notify(&x_entity, cx);
            },
            move |_, _, cx| {
                let next_radius =
                    clamp_float((diameter_x + 0.1).min(40.0) * 0.5, 0.2, 20.0, 2);
                let mut model = x_state_down.model_mut();
                model.radius_mm = next_radius;
                if model.linked {
                    model.radius_y_mm = None;
                }
                drop(model);
                notify(&x_entity_down, cx);
            },
        ))
        .child(ui::scrollable_number_field(
            "hole-punch-diameter-y",
            format_number(diameter_y),
            Some("mm"),
            false,
            diameter_disabled || linked,
            diameter_y < 40.0,
            diameter_y > 0.4,
            move |_, _, cx| {
                y_state.model_mut().radius_y_mm =
                    Some(clamp_float((diameter_y - 0.1).max(0.4) * 0.5, 0.2, 20.0, 2));
                notify(&y_entity, cx);
            },
            move |_, _, cx| {
                y_state_down.model_mut().radius_y_mm =
                    Some(clamp_float((diameter_y + 0.1).min(40.0) * 0.5, 0.2, 20.0, 2));
                notify(&y_entity_down, cx);
            },
        ));

    // Reset / Apply.
    let reset_state = state.clone();
    let reset_entity = entity.clone();
    let apply_state = state.clone();
    let apply_entity = entity.clone();
    let reset_enabled = !(disabled || is_applying || !can_reset);
    let apply_enabled = !(disabled || is_applying || !can_apply);
    let mut reset_spec = ButtonSpec::new(ButtonVariant::Secondary, ButtonSize::Sm).full();
    if !reset_enabled {
        reset_spec = reset_spec.disabled();
    }
    let mut apply_spec = ButtonSpec::new(ButtonVariant::Accent, ButtonSize::Sm).full();
    if !apply_enabled {
        apply_spec = apply_spec.disabled();
    }
    let mut reset = ui::button_with("hole-punch-reset", "Reset", &reset_spec);
    if reset_enabled {
        reset = reset.on_click(move |_, _, cx| {
            *reset_state.model_mut() = HolePunchModel::default();
            notify(&reset_entity, cx);
        });
    }
    let mut apply = ui::button_with_content(
        "hole-punch-apply",
        &apply_spec,
        div()
            .flex()
            .items_center()
            .justify_center()
            .gap(px(6.0))
            .when(is_applying, |this| {
                this.child(spinner_colored(12.0, ACCENT_SECONDARY_CONTRAST))
            })
            .child(if is_applying { "Applying…" } else { "Apply" }),
    );
    if apply_enabled {
        apply = apply.on_click(move |_, _, cx| {
            // PORT GAP: no punching engine — the flag is the port.
            let mut model = apply_state.model_mut();
            model.is_applying = false;
            drop(model);
            notify(&apply_entity, cx);
        });
    }
    let actions = div()
        .flex()
        .gap(px(8.0))
        .child(div().flex_1().child(reset))
        .child(div().flex_1().child(apply));

    // `px-2 pb-2 space-y-2 sm:px-2.5 sm:pb-2.5`.
    panel_body()
        .opacity(body_opacity)
        .when(can_use_auto_depth && interior_view_available, |this| {
            this.child(interior_card)
        })
        .child(depth_card)
        .when(depth_mode == HolePunchDepthMode::Manual, |this| {
            this.child(depth_field_card)
        })
        .child(diameter_card)
        .child(actions)
}
