use super::*;

// ── Hotkeys (`HotkeysSettingsTab.tsx`) ───────────────────────────────────

/// `toKeyLabel` — `' '` reads as Space, a single character is upper-cased.
pub(crate) fn key_label(key: &str) -> String {
    if key == " " {
        return "Space".to_owned();
    }
    if key.chars().count() == 1 {
        return key.to_uppercase();
    }
    key.to_owned()
}

/// `toModifierLabel` — `ctrl` is the platform's primary modifier.
pub(crate) fn modifier_label(modifier: &str, primary: &str) -> String {
    match modifier.trim().to_lowercase().as_str() {
        "ctrl" => primary.to_owned(),
        "shift" => "Shift".to_owned(),
        "alt" => "Alt".to_owned(),
        "meta" => {
            if primary == "Cmd" {
                "Cmd".to_owned()
            } else {
                "Meta".to_owned()
            }
        }
        _ => modifier.to_owned(),
    }
}

pub(crate) fn binding_tokens(binding: &HotkeyBinding, primary: &str) -> Vec<String> {
    let mut tokens: Vec<String> = binding
        .modifier
        .as_deref()
        .map(|modifier| {
            modifier
                .split('+')
                .map(|part| modifier_label(part, primary))
                .collect()
        })
        .unwrap_or_default();
    tokens.push(key_label(&binding.key));
    tokens
}

/// `KbdToken` — `rounded border px-1 py-1 font-mono text-[11px]`.
pub(crate) fn kbd_token(label: &str, muted: bool) -> Div {
    let (border, background, color) = if muted {
        (BORDER_SUBTLE, mix(SURFACE_2, SURFACE_1, 0.6), TEXT_MUTED)
    } else {
        (mix(ACCENT, BORDER_SUBTLE, 0.5), mix(SURFACE_2, SURFACE_1, 0.96), TEXT_STRONG)
    };
    div()
        .min_w(px(20.))
        .flex()
        .items_center()
        .justify_center()
        .rounded(px(4.))
        .border_1()
        .border_color(rgb(border))
        .bg(rgb(background))
        .px(px(4.))
        .py(px(4.))
        .text_size(px(11.))
        .text_color(rgb(color))
        .child(label.to_owned())
}

/// One `HotkeyRow`.
pub(crate) fn hotkey_row(
    category: &'static str,
    action: &'static str,
    label: &'static str,
    binding: &HotkeyBinding,
    recording: bool,
    secondary: Option<&'static str>,
    entity: WeakEntity<DragonFruit>,
) -> Div {
    let primary = if cfg!(target_os = "macos") { "Cmd" } else { "Ctrl" };
    let row_entity = entity;

    let mut right = div()
        .id(format!("hotkey-{category}-{action}"))
        .min_w(px(92.))
        .flex()
        .items_center()
        .justify_center()
        .gap(px(4.))
        .rounded(px(6.))
        .px(px(4.))
        .py(px(2.))
        .cursor_pointer()
        .on_click(move |_, _, cx| {
            edit_and_notify(&row_entity, cx, |draft| {
                draft.hotkeys.recording = if draft.hotkeys.recording == Some((category, action)) {
                    None
                } else {
                    Some((category, action))
                };
            });
        });

    if recording {
        right = right.child(
            div()
                .text_size(px(11.))
                .font_weight(FontWeight::MEDIUM)
                .text_color(rgb(TEXT_MUTED))
                .child("Press keys…"),
        );
    } else {
        for token in binding_tokens(binding, primary) {
            right = right.child(kbd_token(&token, false));
        }
    }

    let mut trailing = div().flex().items_center().justify_end().gap(px(4.)).child(right);
    if !recording {
        if let Some(secondary) = secondary {
            trailing = trailing.child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(4.))
                    .child(
                        div()
                            .text_size(px(11.))
                            .text_color(rgb(TEXT_MUTED))
                            .child("/"),
                    )
                    .child(kbd_token(secondary, true)),
            );
        }
    }

    let (border, background) = if recording {
        (mix(ACCENT, BORDER_SUBTLE, 0.65), mix(ACCENT, SURFACE_1, 0.16))
    } else {
        (BORDER_SUBTLE, mix(SURFACE_2, SURFACE_1, 0.9))
    };

    div()
        .w_full()
        .flex()
        .items_center()
        .justify_between()
        .gap(px(8.))
        .rounded(px(6.))
        .border_1()
        .border_color(rgb(border))
        .bg(rgb(background))
        .px(px(8.))
        .py(px(6.))
        .text_color(rgb(TEXT_STRONG))
        .child(
            div()
                .min_w(px(0.))
                .truncate()
                .text_size(px(11.))
                .text_color(rgb(TEXT_STRONG))
                .child(label),
        )
        .child(trailing)
}

pub(crate) fn hotkey_section(
    title: &'static str,
    description: &'static str,
    categories: &[&'static str],
    state: &SettingsState,
    entity: WeakEntity<DragonFruit>,
) -> Div {
    let reset_entity = entity.clone();
    let reset_categories: Vec<&'static str> = categories.to_vec();

    let mut entries = div().mt(px(8.)).flex().flex_col().gap(px(8.));
    for category in categories {
        let mut group = div().flex().flex_col().gap(px(4.));
        if categories.len() > 1 {
            group = group.child(
                div()
                    .text_size(px(11.))
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(rgb(TEXT_MUTED))
                    .child(category_label(category)),
            );
        }
        for &(entry_category, action, key, modifier, description) in HOTKEY_DEFAULTS.iter() {
            if entry_category != *category {
                continue;
            }
            let _ = (key, modifier);
            let binding = state.hotkeys.binding(entry_category, action);
            let label = if *category == "PRESETS" {
                pinned_slot_label(action)
            } else {
                description
            };
            let recording = state.hotkeys.recording == Some((entry_category, action));
            let secondary = if *category == "GLOBAL" && action == "DELETE" {
                Some("Delete")
            } else {
                None
            };
            group = group.child(hotkey_row(
                entry_category,
                action,
                label,
                &binding,
                recording,
                secondary,
                entity.clone(),
            ));
        }
        entries = entries.child(group);
    }

    div()
        .flex()
        .flex_col()
        .h_full()
        .rounded(px(8.)) // rounded-lg
        .border_1()
        .border_color(rgb(BORDER_SUBTLE))
        .bg(rgb(SURFACE_1))
        .p(px(10.))
        .child(
            div()
                .flex()
                .items_start()
                .gap(px(8.))
                .child(icon_chip("keyboard"))
                .child(
                    div()
                        .min_w(px(0.))
                        .flex_1()
                        .child(
                            div()
                                .text_size(px(14.))
                                .font_weight(FontWeight::SEMIBOLD)
                                .text_color(rgb(TEXT_STRONG))
                                .child(title),
                        )
                        .child(
                            div()
                                .text_size(px(11.))
                                .text_color(rgb(TEXT_MUTED))
                                .child(description),
                        ),
                )
                .child(
                    div()
                        .id(format!("hotkeys-reset-{title}"))
                        .w(px(28.))
                        .h(px(28.))
                        .flex_shrink_0()
                        .flex()
                        .items_center()
                        .justify_center()
                        .rounded(px(6.))
                        .cursor_pointer()
                        .hover(|style| style.bg(rgb(SURFACE_2)))
                        .on_click(move |_, _, cx| {
                            edit_and_notify(&reset_entity, cx, |draft| {
                                for category in reset_categories.iter() {
                                    draft.hotkeys.reset_category(category);
                                }
                            });
                        })
                        .child(icon("rotate-ccw", 14., TEXT_MUTED)),
                ),
        )
        .child(entries)
}

pub(crate) fn category_label(category: &str) -> &'static str {
    match category {
        "GLOBAL" => "General",
        "CAMERA" => "Camera",
        "CANVAS" => "Canvas Tools",
        "SUPPORTS" => "Supports",
        "PRESETS" => "Presets",
        _ => "General",
    }
}

pub(crate) fn pinned_slot_label(action: &str) -> &'static str {
    match action {
        "SLOT_1" => "Slot 1",
        "SLOT_2" => "Slot 2",
        "SLOT_3" => "Slot 3",
        "SLOT_4" => "Slot 4",
        "SLOT_5" => "Slot 5",
        "SLOT_6" => "Slot 6",
        _ => "Slot",
    }
}

pub(crate) fn hotkeys_tab(state: &SettingsState, entity: WeakEntity<DragonFruit>) -> Div {
    const SECTIONS: [(&str, &str, [&str; 2]); 4] = [
        (
            "Global",
            "General and camera shortcuts available across all workspaces.",
            ["GLOBAL", "CAMERA"],
        ),
        (
            "Prepare",
            "Canvas tool switching and scene arrangement shortcuts.",
            ["CANVAS", ""],
        ),
        (
            "Supports",
            "Support authoring workflow shortcuts.",
            ["SUPPORTS", ""],
        ),
        (
            "Support Presets",
            "Quick-apply support preset shortcuts.",
            ["PRESETS", ""],
        ),
    ];

    let mut rows = div().flex().flex_col().gap(px(10.));
    for pair in SECTIONS.chunks(2) {
        let mut row = div().grid().grid_cols(2).gap(px(10.));
        for (title, description, categories) in pair {
            let categories: Vec<&'static str> = categories
                .iter()
                .copied()
                .filter(|category| !category.is_empty())
                .collect();
            row = row.child(hotkey_section(
                title,
                description,
                &categories,
                state,
                entity.clone(),
            ));
        }
        rows = rows.child(row);
    }

    div()
        .flex_1()
        .min_h(px(0.))
        .flex()
        .flex_col()
        .gap(px(8.))
        .child(
            div()
                .id("hotkeys-scroll")
                .flex_1()
                .min_h(px(0.))
                .overflow_y_scroll()
                .pr(px(4.))
                .child(rows),
        )
}

/// The `Recording Shortcut` overlay.
pub(crate) fn hotkeys_overlay(state: &SettingsState, entity: WeakEntity<DragonFruit>) -> Option<Div> {
    state.hotkeys.recording?;
    let cancel_entity = entity;
    Some(
        div()
            .absolute()
            .top(px(0.))
            .left(px(0.))
            .size_full()
            .flex()
            .items_center()
            .justify_center()
            .px(px(16.))
            .bg(rgba(0x0000008c)) // bg-black/55
            .child(
                div()
                    .w(px(448.)) // max-w-md
                    .rounded(px(12.))
                    .border_1()
                    .border_color(rgb(BORDER_STRONG))
                    .bg(rgb(SURFACE_0))
                    .p(px(20.))
                    .child(
                        div()
                            .text_size(px(16.))
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(rgb(TEXT_STRONG))
                            .child("Recording Shortcut"),
                    )
                    .child(
                        div()
                            .mt(px(8.))
                            .text_size(px(14.))
                            .text_color(rgb(TEXT_MUTED))
                            .child("Press the new key combination now."),
                    )
                    .child(
                        div()
                            .mt(px(4.))
                            .flex()
                            .flex_wrap()
                            .text_size(px(12.))
                            .text_color(rgb(TEXT_MUTED))
                            .child("Press ")
                            .child(
                                div()
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .text_color(rgb(TEXT_STRONG))
                                    .child("Esc"),
                            )
                            .child(" or click outside to cancel."),
                    )
                    .child(
                        div().mt(px(16.)).flex().justify_end().child(
                            div()
                                .id("hotkeys-cancel-recording")
                                .h(px(32.))
                                .px(px(12.))
                                .flex()
                                .items_center()
                                .justify_center()
                                .rounded(px(6.))
                                .border_1()
                                .border_color(rgb(BORDER_SUBTLE))
                                .bg(rgb(SURFACE_1))
                                .text_size(px(12.))
                                .font_weight(FontWeight::SEMIBOLD)
                                .text_color(rgb(TEXT_STRONG))
                                .cursor_pointer()
                                .hover(|style| style.bg(rgb(SURFACE_2)))
                                .on_click(move |_, _, cx| {
                                    edit_and_notify(&cancel_entity, cx, |draft| {
                                        draft.hotkeys.recording = None;
                                    });
                                })
                                .child("Cancel"),
                        ),
                    ),
            ),
    )
}
