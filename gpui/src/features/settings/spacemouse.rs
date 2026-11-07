use super::*;

// ── 3D Mouse (`SpaceMouseSettingsTab.tsx`) ───────────────────────────────

pub(crate) fn spacemouse_tab(state: &SettingsState, entity: WeakEntity<DragonFruit>) -> Div {
    let mouse = &state.spacemouse;
    // The native navlib bridge is absent, so `navlibActive` is always false and
    // the tuning sections render.
    let mut root = div().flex().flex_col().gap(px(12.));

    let enable_entity = entity.clone();
    let pivot_entity = entity.clone();

    // Detected devices: no gamepad API in this slice, so `candidatePads` is
    // empty and the source's empty state renders.
    let devices = div()
        .mt(px(8.))
        .rounded(px(6.))
        .border_1()
        .border_color(rgb(BORDER_SUBTLE))
        .bg(rgb(SURFACE_0))
        .p(px(10.))
        .child(
            div()
                .mb(px(8.))
                .flex()
                .items_center()
                .gap(px(8.))
                .child(icon("gamepad-2", 16., TEXT_MUTED))
                .child(
                    div()
                        .text_size(px(12.))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(rgb(TEXT_STRONG))
                        .child("Detected devices"),
                ),
        )
        .child(
            div()
                .text_size(px(12.))
                .text_color(rgb(TEXT_MUTED))
                .child("No compatible input devices detected. Ensure your 3D mouse driver is installed."),
        );

    let pivot = div()
        .mt(px(8.))
        .rounded(px(6.))
        .border_1()
        .border_color(rgb(BORDER_SUBTLE))
        .bg(rgb(SURFACE_0))
        .p(px(10.))
        .flex()
        .flex_col()
        .gap(px(4.))
        .child(
            div()
                .text_size(px(12.))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(rgb(TEXT_STRONG))
                .child("Pivot mode"),
        )
        .child(
            div()
                .text_size(px(12.))
                .text_color(rgb(TEXT_MUTED))
                .child("Auto picks selected/single/nearest model center. Camera Raycast picks the model center nearest your current view beam."),
        )
        .child(select_field(
            "spacemouse-pivot",
            &PIVOT_OPTIONS,
            match mouse.pivot_mode {
                PivotMode::Auto => "auto",
                PivotMode::CameraRay => "camera-ray",
            },
            None,
            None,
            true,
            pivot_entity.clone(),
            move |value, cx| {
                edit_and_notify(&pivot_entity, cx, |draft| {
                    draft.spacemouse.pivot_mode = if value == "camera-ray" {
                        PivotMode::CameraRay
                    } else {
                        PivotMode::Auto
                    };
                });
            },
        ));

    root = root.child(
        section(
            "mouse-pointer-2",
            "3D Mouse Input",
            "6-DoF navigation for pan, orbit, zoom, and roll using compatible 3D mouse devices.",
        )
        .child(
            setting_row(
                "Enable 3D Mouse",
                Some("Uses browser gamepad input from supported 3D mouse devices.".into()),
                SettingDensity::Compact,
                true,
                SettingSurface::Sunken,
                false,
                segmented(
                    "spacemouse-enabled",
                    &["OFF", "ON"],
                    usize::from(mouse.enabled),
                    false,
                    move |index, cx| {
                        edit_and_notify(&enable_entity, cx, |draft| {
                            draft.spacemouse.enabled = index == 1;
                        });
                    },
                ),
            )
            .mt(px(12.)),
        )
        .child(devices)
        .child(pivot),
    );

    // Motion Tuning
    let dominant_entity = entity.clone();
    root = root.child(
        section(
            "sliders-horizontal",
            "Motion Tuning",
            "Adjust sensitivity and movement behavior for translation, rotation, zoom, and deadzone.",
        )
        .child(
            div()
                .mt(px(8.))
                .grid()
                .grid_cols(2)
                .gap(px(8.))
                .child(slider_field(
                    "Translation",
                    format!("{:.2}", mouse.translation_sensitivity),
                    mouse.translation_sensitivity,
                    0.1,
                    4.0,
                ))
                .child(slider_field(
                    "Rotation",
                    format!("{:.2}", mouse.rotation_sensitivity),
                    mouse.rotation_sensitivity,
                    0.1,
                    4.0,
                ))
                .child(slider_field(
                    "Zoom",
                    format!("{:.2}", mouse.zoom_sensitivity),
                    mouse.zoom_sensitivity,
                    0.1,
                    4.0,
                ))
                .child(slider_field(
                    "Deadzone",
                    format!("{:.2}", mouse.deadzone),
                    mouse.deadzone,
                    0.0,
                    0.3,
                )),
        )
        .child(
            setting_row(
                "Dominant axis mode",
                Some("Prioritize strongest axis per translation/rotation group for steadier input.".into()),
                SettingDensity::Compact,
                true,
                SettingSurface::Sunken,
                false,
                segmented(
                    "spacemouse-dominant",
                    &["OFF", "ON"],
                    usize::from(mouse.dominant_axis),
                    false,
                    move |index, cx| {
                        edit_and_notify(&dominant_entity, cx, |draft| {
                            draft.spacemouse.dominant_axis = index == 1;
                        });
                    },
                ),
            )
            .mt(px(8.)),
        ),
    );

    // Axis Inversion
    let axes: [(&'static str, &'static str, bool); 6] = [
        ("invertTx", "Invert X pan", mouse.invert_tx),
        ("invertTy", "Invert Y pan", mouse.invert_ty),
        ("invertTz", "Invert zoom", mouse.invert_tz),
        ("invertRx", "Invert pitch", mouse.invert_rx),
        ("invertRy", "Invert yaw", mouse.invert_ry),
        ("invertRz", "Invert roll", mouse.invert_rz),
    ];
    let mut axis_grid = div().mt(px(8.)).grid().grid_cols(3).gap(px(8.));
    for (key, label, checked) in axes {
        let axis_entity = entity.clone();
        axis_grid = axis_grid.child(
            div()
                .id(format!("spacemouse-{key}"))
                .flex()
                .items_center()
                .gap(px(8.))
                .rounded(px(4.))
                .border_1()
                .border_color(rgb(BORDER_SUBTLE))
                .bg(rgb(SURFACE_0))
                .px(px(8.))
                .py(px(6.))
                .text_size(px(12.))
                .cursor_pointer()
                .on_click(move |_, _, cx| {
                    edit_and_notify(&axis_entity, cx, |draft| match key {
                        "invertTx" => draft.spacemouse.invert_tx = !draft.spacemouse.invert_tx,
                        "invertTy" => draft.spacemouse.invert_ty = !draft.spacemouse.invert_ty,
                        "invertTz" => draft.spacemouse.invert_tz = !draft.spacemouse.invert_tz,
                        "invertRx" => draft.spacemouse.invert_rx = !draft.spacemouse.invert_rx,
                        "invertRy" => draft.spacemouse.invert_ry = !draft.spacemouse.invert_ry,
                        _ => draft.spacemouse.invert_rz = !draft.spacemouse.invert_rz,
                    });
                })
                .child(checkbox(checked, true))
                .child(div().text_color(rgb(TEXT_MUTED)).child(label)),
        );
    }
    root = root.child(
        section(
            "arrow-left-right",
            "Axis Inversion",
            "Flip the direction of individual pan, zoom, pitch, yaw, and roll axes.",
        )
        .child(axis_grid),
    );

    root.child(
        div()
            .rounded(px(6.))
            .border_1()
            .border_color(rgb(mix(BORDER_SUBTLE, ACCENT, 0.8)))
            .bg(rgb(mix(SURFACE_1, ACCENT, 0.96)))
            .px(px(12.))
            .py(px(8.))
            .child(
                div()
                    .text_size(px(11.))
                    .line_height(relative(1.2))
                    .text_color(rgb(TEXT_MUTED))
                    .child("Supports 3Dconnexion SpaceMouse devices. “SpaceMouse” is a trademark of 3Dconnexion."),
            ),
    )
}
