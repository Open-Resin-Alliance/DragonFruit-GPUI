use super::*;

// ── Plugins (`PluginsSettingsTab.tsx`) ───────────────────────────────────

pub(crate) fn plugins_tab(state: &SettingsState, entity: WeakEntity<DragonFruit>) -> Div {
    let plugins = &state.plugins;
    let allowance_entity = entity.clone();
    let install_entity = entity.clone();

    // `installedRows` is empty: the profile store and the generated built-in
    // plugin manifests are not part of this slice.
    let installed_count = 0usize;

    let mut root = div().flex().flex_col().gap(px(12.));

    let mut header = section(
        "plug",
        "Plugins",
        "Profile packs add presets and material profiles. A sideloaded package adds code and needs a publisher you trust.",
    )
    .relative();
    // `IconButton` absolute right-3 top-3, `PackageOpen`, warning tone.
    header = header.child(
        div()
            .absolute()
            .top(px(12.))
            .right(px(12.))
            .child(
                div()
                    .id("plugins-allowance")
                    .w(px(28.))
                    .h(px(28.))
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded(px(6.))
                    .border_1()
                    .border_color(rgb(mix(0xf59e0b, BORDER_SUBTLE, 0.45)))
                    .bg(rgb(mix(0xf59e0b, SURFACE_1, 0.9)))
                    .cursor_pointer()
                    .hover(|style| style.bg(rgb(mix(0xf59e0b, SURFACE_1, 0.82))))
                    .on_click(move |_, _, cx| {
                        edit_and_notify(&allowance_entity, cx, |draft| {
                            draft.plugins.allowance_dialog_open = true;
                        });
                    })
                    .child(icon("package-open", 16., 0xf59e0b)),
            ),
    );

    let install_row = setting_row_surface(
        "Install profile pack",
        "Adds presets and materials from a repository. Contains no code.",
        false,
        false,
        div()
            .flex()
            .items_center()
            .gap(px(8.))
            .w(px(420.))
            .child(
                div().flex_1().min_w(px(0.)).child(text_field(
                    &plugins.repo_url,
                    "https://github.com/owner/repo",
                    false,
                    34.0,
                    14.0,
                )),
            )
            .child(
                div()
                    .id("plugins-install")
                    .h(px(36.))
                    .px(px(12.))
                    .flex_shrink_0()
                    .flex()
                    .items_center()
                    .justify_center()
                    .gap(px(6.))
                    .rounded(px(6.))
                    .border_1()
                    .border_color(rgb(mix(ACCENT, BORDER_SUBTLE, 0.45)))
                    .bg(rgb(mix(ACCENT, SURFACE_1, 0.86)))
                    .text_size(px(14.))
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(rgb(ACCENT))
                    .child(icon("download", 14., ACCENT))
                    .child("Install"),
            ),
    );

    let mut rows = div().mt(px(8.)).flex().flex_col().gap(px(8.)).child(install_row);
    if plugins.allow_sideload {
        rows = rows.child(setting_row_surface(
            "Import package",
            "Checks the publisher signature, then unseals and installs the .dfplugin file.",
            false,
            false,
            div()
                .h(px(36.))
                .px(px(12.))
                .flex_shrink_0()
                .flex()
                .items_center()
                .justify_center()
                .gap(px(6.))
                .rounded(px(6.))
                .border_1()
                .border_color(rgb(BORDER_SUBTLE))
                .bg(rgb(SURFACE_1))
                .text_size(px(14.))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(rgb(TEXT_STRONG))
                .child(icon("package-plus", 14., TEXT_STRONG))
                .child("Choose file…"),
        ));
    }
    header = header.child(rows);
    if let Some((error, message)) = &plugins.status {
        header = header.child(
            div()
                .mt(px(8.))
                .text_size(px(12.))
                .text_color(rgb(if *error { 0xfca5a5 } else { 0x86efac }))
                .child(message.clone()),
        );
    }
    root = root.child(header);

    // Installed plugins
    let mut installed = section(
        "package-check",
        "Installed plugins",
        if installed_count == 1 {
            "1 plugin in this profile."
        } else {
            "0 plugins in this profile."
        },
    );
    installed = installed.child(
        div()
            .mt(px(8.))
            .flex()
            .flex_col()
            .gap(px(6.))
            .child(
                div()
                    .px(px(12.))
                    .py(px(8.))
                    .text_size(px(12.))
                    .text_color(rgb(TEXT_MUTED))
                    .child("No plugins installed in this profile."),
            ),
    );
    root = root.child(installed);

    // `isDevRuntime` is false for a release build, so the Plugin Creation Studio
    // link is absent, exactly as at the source.
    let _ = install_entity;
    root
}

/// The `Sideloaded packages` allowance dialog.
pub(crate) fn plugins_overlay(state: &SettingsState, entity: WeakEntity<DragonFruit>) -> Option<Div> {
    if !state.plugins.allowance_dialog_open {
        return None;
    }
    let plugins = &state.plugins;
    let cancel_entity = entity.clone();
    let toggle_entity = entity;

    let body = if plugins.allow_sideload {
        div().child(
            div()
                .text_size(px(14.))
                .line_height(relative(1.45))
                .text_color(rgb(TEXT_MUTED))
                .child("Packages you import run with your permissions and can read any file you can read, open network connections, and crash the app if they misbehave. Disabling this stops new imports; packages already installed stay on disk."),
        )
    } else {
        div()
            .flex()
            .flex_col()
            .gap(px(6.))
            .child(
                div()
                    .text_size(px(14.))
                    .line_height(relative(1.45))
                    .text_color(rgb(TEXT_STRONG))
                    .child("A sideloaded package is a native program, not a profile pack."),
            )
            .child(
                div()
                    .text_size(px(14.))
                    .line_height(relative(1.45))
                    .text_color(rgb(TEXT_MUTED))
                    .child("Once imported it runs with your permissions: it can read any file you can read, open network connections, and crash the app if it misbehaves."),
            )
    };

    Some(dialog_shell(
        512.0, // max-w-lg
        "shield-alert",
        (0xf59e0b, mix(0xf59e0b, BORDER_SUBTLE, 0.45), mix(0xf59e0b, SURFACE_1, 0.9)),
        if plugins.allow_sideload {
            "Sideloading is on"
        } else {
            "Allow untrusted plugins?"
        },
        Some("Sideloaded packages".into()),
        body.child(
            div()
                .mt(px(12.))
                .rounded(px(6.))
                .border_1()
                .border_color(rgb(mix(0xd97706, BORDER_SUBTLE, 0.4)))
                .bg(rgb(mix(0xd97706, SURFACE_1, 0.92)))
                .px(px(12.))
                .py(px(8.))
                .flex()
                .items_start()
                .gap(px(8.))
                .child(icon("triangle-alert", 14., 0xd97706))
                .child(
                    div()
                        .flex_1()
                        .text_size(px(12.))
                        .line_height(relative(1.45))
                        .text_color(rgb(TEXT_MUTED))
                        .child("Signatures prove who built the package, not that it is safe. Only enable this for a publisher you trust."),
                ),
        ),
        div()
            .flex()
            .items_center()
            .gap(px(8.))
            .child(action_button(
                "plugins-allowance-cancel",
                "Cancel",
                None,
                16.0,
                SURFACE_1,
                BORDER_SUBTLE,
                TEXT_STRONG,
                SURFACE_2,
                move |cx| {
                    edit_and_notify(&cancel_entity, cx, |draft| {
                        draft.plugins.allowance_dialog_open = false;
                    });
                },
            ))
            .child(action_button(
                "plugins-allowance-toggle",
                if plugins.allow_sideload {
                    "Disable Sideloading"
                } else {
                    "Enable Sideloading"
                },
                Some("shield-alert"),
                16.0,
                if plugins.allow_sideload {
                    mix(0xef4444, SURFACE_1, 0.86)
                } else {
                    mix(0xf59e0b, SURFACE_1, 0.86)
                },
                mix(0xf59e0b, BORDER_SUBTLE, 0.45),
                if plugins.allow_sideload { 0xef4444 } else { 0xf59e0b },
                mix(0xf59e0b, SURFACE_1, 0.76),
                move |cx| {
                    edit_and_notify(&toggle_entity, cx, |draft| {
                        draft.plugins.allow_sideload = !draft.plugins.allow_sideload;
                        draft.plugins.allowance_dialog_open = false;
                    });
                },
            )),
    ))
}
