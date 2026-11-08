//! Support settings — the Support Studio's panel, `SupportSidebar.tsx`'s preset
//! strip over the auto-support knobs `AutoSupportSettingsBody.tsx` renders
//! (Density & Sizing, Stability) and its Delete / Reset / Save footer.
//!
//! The strip's collection and the selected record come from
//! `support-presets.json` ([`super::studio_presets`]); the knobs edit the *live*
//! auto-support block that module holds, seeded once from the selected record.
//! A preset selection changes neither block (`setActivePreset` preserves
//! `autoSupport`) nor `DragonFruit::support_angle`.
//!
//! # PORT DEVIATIONS
//!
//! 1. Only the sections this panel is named for are ported: the preset strip,
//!    Density & Sizing, Stability and the footer. Detection, Distribution,
//!    Post-processing, Diagnostics and Advanced (calibration) are not rendered.
//! 2. The strip lists the studio collection (`detail` / `structure` / `anchor`
//!    plus the user's records, from `presets.ts`) rather than the auto-support
//!    quick-select (`light` / `medium` / `heavy`) the first port put here: the
//!    store crate models the studio collection, and that is what the Support
//!    Studio's strip shows.
//! 3. The lock is the active record's own `isBuiltIn` flag, never its id
//!    (`presets.ts:146-153,210-217,239-246` ship all three factory records with
//!    `isBuiltIn: false`, and `:779` states there is no built-in lock), which is
//!    the same source `AutoSupportPanel.tsx:125` derives `presetLocked` from.
//!    A record that *does* set the flag locks every knob and gates `Save` on
//!    `Duplicate`, as at the source.
//! 4. `Rename` / `Export` / `Import` are inert: the source's rename needs a text
//!    field and export/import need the Tauri file bridge, neither of which this
//!    slice has. Their buttons render the source's enabled recipe; `Rename` is
//!    gated on the record's own `isBuiltIn`, as `PresetSelector.tsx:543-544`
//!    gates it.
//! 5. `overhangSelfSupportAngleDeg` is wired to `DragonFruit::support_angle`, the
//!    value `stl::overhang_points` classifies with; the other knobs have no
//!    consumer until W6 ports `autoSupport`. The angle is applied when the field
//!    is edited, on `Reset` and on `Save` — never when a preset is selected
//!    (`setActivePreset` preserves the live block, `presets.ts:683-693`).
//! 6. The knobs edit the *live* auto-support block, not the selected record's:
//!    the source keeps `autoSupport: { ...current.autoSupport }` on selection and
//!    restores the record's own block on `Save` (`presets.ts:723-731`). Selecting
//!    a preset therefore changes none of these values.
//! 7. No text entry: `labeled_number_field`'s chevrons are the way to change a
//!    value. No range inputs, no CSS transitions and no `tabular-nums`.

use gpui::{
    App, Div, FontWeight, IntoElement, SharedString, WeakEntity, div, prelude::*, px, rgb, rgba,
};

use gpui_core::support_state::{NO_ACTIVE_PRESET_LABEL, field_help_text};
use gpui_core::theme::*;
use gpui_ui::panel_fields::with_alpha;
use gpui_ui::ui::{
    self, ButtonSize, ButtonSpec, ButtonVariant, SelectOption, select_dropdown,
};

use super::shared::{
    accent_secondary_action_bg, accent_secondary_action_border, field_help,
    labeled_number_field, normalize_value, notify,
};
use super::studio_presets;
use crate::DragonFruit;

// ---------------------------------------------------------------------------
// Support settings — `AutoSupportSettingsBody.tsx`
// ---------------------------------------------------------------------------
pub fn support_settings_body(entity: WeakEntity<DragonFruit>) -> impl IntoElement {
    let rows = studio_presets::preset_rows();
    let active_id = studio_presets::active_id();
    let draft = studio_presets::values();
    let modified = studio_presets::is_modified();
    let preset_menu_open = studio_presets::menu_open();
    let active_name =
        studio_presets::active_name().unwrap_or_else(|| NO_ACTIVE_PRESET_LABEL.to_string());
    // The lock is the record's own `isBuiltIn`, never the id: all three factory
    // records ship `isBuiltIn: false` (`presets.ts:146-153,210-217,239-246`),
    // and `presets.ts:779` states there is no built-in lock.
    let locked = studio_presets::active_locked();

    // ── Preset strip ────────────────────────────────────────────────────────
    let trigger_label: SharedString = if modified {
        format!("{active_name} *").into()
    } else {
        active_name.clone().into()
    };
    let options: Vec<SelectOption> = rows
        .iter()
        .map(|(id, name, _)| SelectOption::new(id.clone(), name.clone()))
        .collect();

    let toggle_entity = entity.clone();
    let select_entity = entity.clone();
    let selector = select_dropdown(
        "support-preset-select",
        &options,
        active_id.as_str(),
        preset_menu_open,
        false,
        move |_, _, cx| {
            studio_presets::toggle_menu();
            notify(&toggle_entity, cx);
        },
        move |index, _, cx| {
            if let Some((id, _, _)) = rows.get(index) {
                // `setActivePreset` preserves the live auto-support block
                // (`presets.ts:683-693`): the selection must not move a knob or
                // the angle the app classifies with.
                studio_presets::select(id);
            }
            notify(&select_entity, cx);
        },
    );
    // The trigger's label is the preset name; `select_dropdown` derives it from
    // the option list, so the modified marker needs its own overlay.
    let selector = if modified {
        div()
            .relative()
            .w_full()
            .child(selector)
            .child(
                div()
                    .absolute()
                    .left(px(10.0))
                    .right(px(40.0))
                    .top(px(0.0))
                    .bottom(px(0.0))
                    .flex()
                    .items_center()
                    .text_size(px(12.0))
                    .text_color(rgb(TEXT_STRONG))
                    .child(trigger_label.clone()),
            )
    } else {
        div().w_full().child(selector)
    };

    // `STRIP_BUTTON`: `!h-8 !px-2.5 !py-0`, gap 1 (4px), the glyph in the
    // button's own colour.
    let strip_button = |id: &'static str, icon_name: &str, label: &'static str, enabled: bool| {
        let mut spec = ButtonSpec::new(ButtonVariant::Secondary, ButtonSize::Sm)
            .padding_x(10.0)
            .padding_y(0.0, 0.0)
            .gap(4.0)
            .icon(icon_name, 14.0, if enabled { TEXT_STRONG } else { TEXT_MUTED });
        if !enabled {
            spec = spec.disabled();
        }
        ui::button_with(id, label, &spec)
    };
    // `Duplicate` is the way out of a locked built-in: the copy is a custom
    // profile, so the policy fields unlock and Save can write it.
    let duplicate_entity = entity.clone();
    let duplicate = ui::button_with(
        "support-preset-duplicate",
        "Duplicate",
        &ButtonSpec::new(ButtonVariant::Secondary, ButtonSize::Sm)
            .padding_x(10.0)
            .padding_y(0.0, 0.0)
            .gap(4.0)
            .icon("copy", 14.0, TEXT_STRONG),
    )
    .on_click(move |_, _, cx| {
        studio_presets::duplicate_active();
        notify(&duplicate_entity, cx);
    });
    let strip = div()
        .rounded(px(ROUNDED_XL))
        .border_1()
        .border_color(rgb(BORDER_SUBTLE))
        .bg(rgb(SURFACE_2))
        .p(px(12.0))
        .flex()
        .flex_col()
        .gap(px(8.0))
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(8.0))
                .child(div().flex_1().min_w(px(0.0)).child(selector))
                // `PresetSelector.tsx:543-544` refuses rename only when the
                // selected record is `isBuiltIn`; the click is inert until the
                // slice gains text entry (deviation 5 in `studio_presets`).
                .child(strip_button("support-preset-rename", "pen-line", "Rename", !locked))
                .child(duplicate)
                .child(strip_button("support-preset-export", "download", "Export", true))
                .child(strip_button("support-preset-import", "upload", "Import", true)),
        );

    // ── Density & Sizing ────────────────────────────────────────────────────
    let density_card = field_card(
        "Density & Sizing",
        Some("How many supports, and how thick"),
        vec![
            labeled_number_field(
                "support-area-per-support",
                "Support Density",
                field_help_text(locked, "Projected area each support carries — smaller = more, tighter supports (grid spacing ≈ √value)"),
                draft.area_per_support_mm2,
                Some("mm²"),
                0.5,
                1.0,
                30.0,
                locked,
                {
                    let entity = entity.clone();
                    move |next, _, cx| {
                        studio_presets::set_value(|values| {
                            values.area_per_support_mm2 = normalize_value(next, 1.0, 30.0, 0.5);
                        });
                        notify(&entity, cx);
                    }
                },
            ),
            labeled_number_field(
                "support-size-scale",
                "Support Size",
                field_help_text(locked, "Master multiplier over the preset sizing bands — thicker or thinner everywhere"),
                draft.size_scale,
                Some("×"),
                0.05,
                0.5,
                2.0,
                locked,
                {
                    let entity = entity.clone();
                    move |next, _, cx| {
                        studio_presets::set_value(|values| {
                            values.size_scale = normalize_value(next, 0.5, 2.0, 0.05);
                        });
                        notify(&entity, cx);
                    }
                },
            ),
            labeled_number_field(
                "support-grid-threshold",
                "Grid Threshold",
                field_help_text(locked, "Flat regions at/above this area get a full grid; smaller regions get a single support"),
                draft.grid_area_threshold_mm2,
                Some("mm²"),
                5.0,
                5.0,
                200.0,
                locked,
                {
                    let entity = entity.clone();
                    move |next, _, cx| {
                        studio_presets::set_value(|values| {
                            values.grid_area_threshold_mm2 =
                                normalize_value(next, 5.0, 200.0, 5.0);
                        });
                        notify(&entity, cx);
                    }
                },
            ),
            labeled_number_field(
                "support-flat-boost",
                "Flat Boost",
                field_help_text(locked, "Grid spacing on flat ceilings — lower = denser supports on anchor surfaces (0.7 = ~2× the supports)"),
                draft.flat_density_boost,
                Some("×"),
                0.05,
                0.5,
                1.0,
                locked,
                {
                    let entity = entity.clone();
                    move |next, _, cx| {
                        studio_presets::set_value(|values| {
                            values.flat_density_boost = normalize_value(next, 0.5, 1.0, 0.05);
                        });
                        notify(&entity, cx);
                    }
                },
            ),
            labeled_number_field(
                "support-slope-relax",
                "Slope Relax",
                field_help_text(locked, "Grid spacing on slopes at the self-support angle — higher = sparser"),
                draft.slope_relax_factor,
                Some("×"),
                0.1,
                1.0,
                2.0,
                locked,
                {
                    let entity = entity.clone();
                    move |next, _, cx| {
                        studio_presets::set_value(|values| {
                            values.slope_relax_factor = normalize_value(next, 1.0, 2.0, 0.1);
                        });
                        notify(&entity, cx);
                    }
                },
            ),
            labeled_number_field(
                "support-suction-scale",
                "Suction Scale",
                field_help_text(locked, "How strongly flat density grows with region area — large shallow ceilings carry more peel. 0 = off"),
                draft.suction_area_exponent,
                None,
                0.05,
                0.0,
                0.4,
                locked,
                {
                    let entity = entity.clone();
                    move |next, _, cx| {
                        studio_presets::set_value(|values| {
                            values.suction_area_exponent =
                                normalize_value(next, 0.0, 0.4, 0.05);
                        });
                        notify(&entity, cx);
                    }
                },
            ),
        ],
    );

    // ── Stability ───────────────────────────────────────────────────────────
    let stability_card = field_card(
        "Stability",
        Some("Held against toppling and peel"),
        vec![
            labeled_number_field(
                "support-self-support-angle",
                "Self-Support Angle",
                field_help_text(locked, "Surfaces flatter than this angle get supports (resin standard: 45°). Higher = fewer, mostly on the steepest parts."),
                draft.self_support_angle_deg,
                Some("°"),
                5.0,
                20.0,
                75.0,
                locked,
                {
                    let entity = entity.clone();
                    move |next, _, cx| {
                        studio_presets::set_value(|values| {
                            values.self_support_angle_deg =
                                normalize_value(next, 20.0, 75.0, 5.0);
                        });
                        // Wired: the app classifies overhangs with this value.
                        apply_angle(&entity, cx);
                        notify(&entity, cx);
                    }
                },
            ),
            ui::toggle_row(
                "support-stabilization",
                "Stabilization Anchors",
                Some(field_help(
                    "support-stabilization-help",
                    field_help_text(locked, "Add contacts along the edge or corner a pose bears on, so a leaning part is held against toppling and peel (default on)"),
                )
                .into_any_element()),
                draft.stabilization_enabled,
                locked,
                {
                    let entity = entity.clone();
                    move |next, _, cx| {
                        studio_presets::set_value(|values| {
                            values.stabilization_enabled = next;
                        });
                        notify(&entity, cx);
                    }
                },
            ),
            ui::toggle_row(
                "support-minima-reinforcement",
                "Minima Reinforcement",
                Some(field_help(
                    "support-minima-reinforcement-help",
                    field_help_text(locked, "Ring each mesh minima with contacts on its own flank, so a section starts on a base instead of a needle (default on)"),
                )
                .into_any_element()),
                draft.minima_reinforcement_enabled,
                locked,
                {
                    let entity = entity.clone();
                    move |next, _, cx| {
                        studio_presets::set_value(|values| {
                            values.minima_reinforcement_enabled = next;
                        });
                        notify(&entity, cx);
                    }
                },
            ),
        ],
    );

    // ── Footer ──────────────────────────────────────────────────────────────
    // `Button variant="tinted-danger" size="md"`: the `.ui-button-tint-danger`
    // fill, with the trash glyph before the label.
    let delete_enabled = !locked;
    let mut delete_spec = ButtonSpec::new(ButtonVariant::TintedDanger, ButtonSize::Md)
        .gap(6.0)
        .icon("trash-2", 14.0, DANGER);
    if !delete_enabled {
        delete_spec = delete_spec.disabled();
    }
    let delete = ui::button_with("support-preset-delete", "Delete", &delete_spec);
    let delete = if delete_enabled {
        let delete_entity = entity.clone();
        delete.on_click(move |_, _, cx| {
            studio_presets::delete_active();
            notify(&delete_entity, cx);
        })
    } else {
        delete
    };

    let reset_state_enabled = modified;
    let reset_entity = entity.clone();
    let mut reset_spec = ButtonSpec::new(ButtonVariant::Secondary, ButtonSize::Md)
        .gap(6.0)
        .icon("rotate-ccw", 14.0, TEXT_STRONG);
    if !reset_state_enabled {
        reset_spec = reset_spec.disabled();
    }
    let reset = ui::button_with("support-preset-reset", "Reset", &reset_spec);
    let reset = if reset_state_enabled {
        reset.on_click(move |_, _, cx| {
            studio_presets::reset();
            apply_angle(&reset_entity, cx);
            notify(&reset_entity, cx);
        })
    } else {
        reset
    };

    let save_entity = entity.clone();
    // `SAVE_STYLE`: the accent-secondary action token, filled; its inline
    // `style` also pins the hover fill.
    let mut save_spec = ButtonSpec::new(ButtonVariant::Secondary, ButtonSize::Md).gap(6.0);
    let save_enabled = modified && !locked;
    if save_enabled {
        save_spec = save_spec
            .fill(
                accent_secondary_action_bg(),
                accent_secondary_action_border(),
                ACCENT_SECONDARY,
            )
            .icon("save", 14.0, ACCENT_SECONDARY);
    } else {
        save_spec = save_spec.disabled();
    }
    let save = ui::button_with("support-preset-save", "Save", &save_spec);
    let save = if save_enabled {
        save.on_click(move |_, _, cx| {
            studio_presets::save_active();
            apply_angle(&save_entity, cx);
            notify(&save_entity, cx);
        })
    } else {
        save
    };

    let footer = div()
        .flex()
        .items_center()
        .justify_between()
        .gap(px(12.0))
        .px(px(16.0))
        .py(px(12.0))
        .flex_shrink_0()
        .border_t(px(1.0))
        .border_color(rgb(BORDER_SUBTLE))
        .bg(rgba(with_alpha(SURFACE_1, 0xe6)))
        .child(delete)
        .child(div().flex().items_center().gap(px(8.0)).child(reset).child(save));

    // `flex-1 min-h-0 flex flex-col`: the scrolling surface then the footer.
    div()
        .flex()
        .flex_col()
        .child(
            div()
                .p(px(16.0))
                .flex()
                .flex_col()
                .gap(px(12.0))
                .child(strip)
                .child(density_card)
                .child(stability_card),
        )
        .child(footer)
}

/// The one knob the app consumes: `DragonFruit::support_angle` is what
/// `stl::overhang_points` classifies with.
fn apply_angle(entity: &WeakEntity<DragonFruit>, cx: &mut App) {
    let angle = studio_presets::values().self_support_angle_deg;
    entity
        .update(cx, |this, cx| {
            this.support_angle = angle;
            cx.notify();
        })
        .ok();
}

/// A `FieldCard`: `rounded-xl border p-3` on `--surface-2`, the uppercase header,
/// its one-liner and a two-column field grid.
fn field_card(
    title: &'static str,
    subtitle: Option<&'static str>,
    fields: Vec<Div>,
) -> Div {
    let mut grid = div().flex().flex_col().gap(px(8.0));
    let mut row = div().flex().gap(px(8.0));
    let mut filled = 0;
    for field in fields {
        row = row.child(div().flex_1().min_w(px(0.0)).child(field));
        filled += 1;
        if filled == 2 {
            grid = grid.child(row);
            row = div().flex().gap(px(8.0));
            filled = 0;
        }
    }
    if filled == 1 {
        grid = grid.child(row.child(div().flex_1().min_w(px(0.0))));
    }

    let mut card = div()
        .rounded(px(ROUNDED_XL))
        .border_1()
        .border_color(rgb(BORDER_SUBTLE))
        .bg(rgb(SURFACE_2))
        .p(px(12.0))
        .flex()
        .flex_col()
        .child(
            div()
                .text_size(px(FONT_SM))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(rgb(INDICATOR))
                .child(title.to_uppercase()),
        );
    if let Some(subtitle) = subtitle {
        card = card.child(
            div()
                .mt(px(2.0))
                .text_size(px(FONT_XS))
                .line_height(px(FONT_XS * 1.375))
                .text_color(rgb(TEXT_MUTED))
                .child(subtitle),
        );
    }
    card.child(div().mt(px(8.0)).child(grid))
}
