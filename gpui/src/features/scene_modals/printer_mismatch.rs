//! `ScenePrinterMismatchModal.tsx` — the printer/scene conflict dialog.

use std::rc::Rc;

use gpui::{App, ClickEvent, Div, SharedString, Window, div, prelude::*, px};
use gpui_core::theme::*;
use gpui_ui::ui::{ButtonSize, ButtonVariant, IconTone, button, dialog_modal};

use super::chrome::{LEADING_RELAXED, DIALOG_LG, muted_paragraph, Run, rich_paragraph, action_slot};
use super::format::{num};

// ── ScenePrinterMismatchModal ───────────────────────────────────────────────

/// `BuildVolumeMm`.
#[derive(Clone, Copy)]
pub struct BuildVolumeMm {
    pub width: f64,
    pub depth: f64,
    pub height: f64,
}

impl BuildVolumeMm {
    fn matches(&self, other: &Self) -> bool {
        let same = |a: f64, b: f64| (a - b).abs() < 1e-6;
        same(self.width, other.width)
            && same(self.depth, other.depth)
            && same(self.height, other.height)
    }

    fn label(&self) -> String {
        format!(
            "{} × {} × {} mm",
            num(self.width),
            num(self.depth),
            num(self.height)
        )
    }
}

/// `ScenePrinterMismatchModal`'s props.
pub struct ScenePrinterMismatch {
    pub recorded_printer_name: Option<SharedString>,
    pub recorded_build_volume: BuildVolumeMm,
    pub current_printer_name: SharedString,
    pub current_build_volume: BuildVolumeMm,
    pub will_add_printer: bool,
    pub current_is_smaller: bool,
    pub current_has_required_materials: bool,
    pub scene_uses_multi_tank_materials: bool,
    pub current_material_mode: Option<SharedString>,
    pub missing_plugin_name: Option<SharedString>,
}

/// `ScenePrinterMismatchModal` — the printer/scene conflict dialog.
///
/// Three variants, chosen exactly as the source chooses them: the missing
/// plugin (`Printer plugin not installed`), the lost material assignments
/// (`Built for Multi-Material`), and the plain mismatch (`Built for another
/// printer`). `Manage Plugins` is only offered in the missing-plugin variant.
#[allow(clippy::too_many_arguments)]
pub fn scene_printer_mismatch_modal(
    mismatch: ScenePrinterMismatch,
    on_switch: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    on_keep: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    on_manage_plugins: Option<impl Fn(&ClickEvent, &mut Window, &mut App) + 'static>,
) -> Div {
    let recorded_label: SharedString = mismatch
        .recorded_printer_name
        .as_ref()
        .map(|name| name.trim().to_string())
        .filter(|name| !name.is_empty())
        .unwrap_or_else(|| "the printer this scene was built for".into())
        .into();
    let plugin_missing = mismatch
        .missing_plugin_name
        .as_ref()
        .is_some_and(|name| !name.trim().is_empty());
    let loses_material_assignments = mismatch.scene_uses_multi_tank_materials
        && mismatch
            .current_material_mode
            .as_deref()
            .is_none_or(|mode| mode != "multiTank");
    let lacks_required_materials = !plugin_missing && !mismatch.current_has_required_materials;
    let build_volumes_match = mismatch
        .recorded_build_volume
        .matches(&mismatch.current_build_volume);

    let title: SharedString = if plugin_missing {
        "Printer plugin not installed".into()
    } else if loses_material_assignments {
        "Built for Multi-Material".into()
    } else {
        "Built for another printer".into()
    };
    let subtitle: SharedString = if plugin_missing {
        "Only the geometry can be imported".into()
    } else if lacks_required_materials {
        "The selected printer is missing the scene's materials".into()
    } else if loses_material_assignments {
        "This scene is made for a multi-material printer".into()
    } else if mismatch.current_is_smaller {
        "This scene may not fit the one selected".into()
    } else {
        "Its plates are spaced for that printer".into()
    };

    let keep_close = Rc::new(on_keep);
    let close_keep = keep_close.clone();
    let action_keep = keep_close.clone();

    // ── body ───────────────────────────────────────────────────────────────
    let mut body = div().flex().flex_col().gap(px(16.0));
    if plugin_missing {
        let plugin_name = mismatch.missing_plugin_name.clone().unwrap_or_default();
        body = body
            .child(rich_paragraph(
                14.0,
                LEADING_RELAXED,
                vec![
                    Run::plain("This scene was saved for "),
                    Run::strong(recorded_label.clone()),
                    Run::plain(", whose printer and material profiles come from the "),
                    Run::strong(plugin_name),
                    Run::plain(
                        " plugin. That plugin is not installed, so those profiles cannot be \
                         restored.",
                    ),
                ],
            ))
            .child(muted_paragraph(
                "Only the geometry will be imported. Install the plugin, then reopen the scene, \
                 to restore its printer and materials.",
            ));
    } else {
        if lacks_required_materials {
            body = body.child(rich_paragraph(
                14.0,
                LEADING_RELAXED,
                vec![
                    Run::strong(mismatch.current_printer_name.clone()),
                    Run::plain(" does not have the materials this scene uses. Load the geometry "),
                    Run::plain("without them, or switch to "),
                    Run::strong(recorded_label.clone()),
                    Run::plain(" to restore them."),
                ],
            ));
        }
        if loses_material_assignments {
            body = body.child(muted_paragraph(
                "Its materials are split across several tanks, which the printer selected now \
                 cannot print. The assignments stay with the scene, hidden while this printer is \
                 selected — switch to a multi-material printer to print them.",
            ));
        }
        if !build_volumes_match {
            let recorded_volume = mismatch.recorded_build_volume.label();
            let current_volume = mismatch.current_build_volume.label();
            let mut lines = div().flex().flex_col().gap(px(16.0));
            lines = lines
                .child(rich_paragraph(
                    14.0,
                    LEADING_RELAXED,
                    vec![
                        Run::strong(recorded_label.clone()),
                        Run::plain(" has a build volume of "),
                        Run::plain(recorded_volume),
                        Run::plain("."),
                    ],
                ))
                .child(rich_paragraph(
                    14.0,
                    LEADING_RELAXED,
                    if mismatch.current_is_smaller {
                        vec![
                            Run::plain("The printer selected now, "),
                            Run::strong(mismatch.current_printer_name.clone()),
                            Run::plain(", has a build volume of "),
                            Run::plain(current_volume),
                            Run::plain(". A plate packed for the larger machine will not fit."),
                        ]
                    } else {
                        vec![
                            Run::plain("The printer selected now, "),
                            Run::strong(mismatch.current_printer_name.clone()),
                            Run::plain(", has a build volume of "),
                            Run::plain(current_volume),
                            Run::plain(", and this scene was laid out for the other one."),
                        ]
                    },
                ))
                .child(rich_paragraph(
                    14.0,
                    LEADING_RELAXED,
                    if mismatch.will_add_printer {
                        vec![
                            Run::plain("It is not in your printer library, so switching adds "),
                            Run::strong(recorded_label.clone()),
                            Run::plain(" from this scene."),
                        ]
                    } else {
                        vec![
                            Run::plain("Switch to "),
                            Run::strong(recorded_label.clone()),
                            Run::plain(" before importing?"),
                        ]
                    },
                ));
            body = body.child(lines);
        }
    }

    // ── actions ────────────────────────────────────────────────────────────
    let mut actions = div().flex().gap(px(8.0));
    if plugin_missing {
        if let Some(on_manage_plugins) = on_manage_plugins {
            actions = actions.child(action_slot(
                button(
                    "printer-mismatch-manage-plugins",
                    "Manage Plugins",
                    ButtonVariant::Secondary,
                    ButtonSize::Md,
                )
                .on_click(on_manage_plugins),
            ));
        }
        actions = actions.child(action_slot(
            button(
                "printer-mismatch-import-geometry",
                "Import Geometry",
                ButtonVariant::TintedAccent,
                ButtonSize::Md,
            )
            .on_click(move |event, window, cx| action_keep(event, window, cx)),
        ));
    } else if lacks_required_materials {
        actions = actions
            .child(action_slot(
                button(
                    "printer-mismatch-load-without-materials",
                    "Load without materials",
                    ButtonVariant::Secondary,
                    ButtonSize::Md,
                )
                .on_click(move |event, window, cx| action_keep(event, window, cx)),
            ))
            .child(action_slot(
                button(
                    "printer-mismatch-switch",
                    format!("Switch to {recorded_label}"),
                    ButtonVariant::TintedAccent,
                    ButtonSize::Md,
                )
                .on_click(move |event, window, cx| on_switch(event, window, cx)),
            ));
    } else {
        actions = actions
            .child(action_slot(
                button(
                    "printer-mismatch-keep",
                    format!("Keep {}", mismatch.current_printer_name),
                    ButtonVariant::Secondary,
                    ButtonSize::Md,
                )
                .on_click(move |event, window, cx| action_keep(event, window, cx)),
            ))
            .child(action_slot(
                button(
                    "printer-mismatch-switch",
                    format!("Switch to {recorded_label}"),
                    ButtonVariant::TintedAccent,
                    ButtonSize::Md,
                )
                .on_click(move |event, window, cx| on_switch(event, window, cx)),
            ));
    }

    dialog_modal(
        DIALOG_LG,
        title,
        Some(subtitle),
        Some("alert-triangle"),
        IconTone::Warning,
        body,
        actions,
        move |event, window, cx| close_keep(event, window, cx),
    )
}
