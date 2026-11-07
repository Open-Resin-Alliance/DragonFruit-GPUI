//! UI probe: a settings-shaped panel on the `gpui-pre` + `gpui-component` line.
//!
//! Run with `cargo run`.
//!
//! This is the other half of the audit. The viewport probes use `bite-gpui`
//! because `gpui_interop` is published against it; the component library is on
//! `gpui-pre` instead. This crate proves the component line builds and renders
//! the controls a slicer settings panel needs.

use gpui_kit::assets::Assets;
use gpui_kit::base::{h_flex, v_flex};
use gpui_kit::component::{
    badge::Badge,
    button::{Button, ButtonVariants},
    checkbox::Checkbox,
    input::{Input, InputState},
    label::Label,
    progress::Progress,
    separator::Separator,
    switch::Switch,
    tab::{Tab, TabBar},
};
use gpui_kit::*;

struct UiProbe {
    name: Entity<InputState>,
    auto_save: bool,
    experimental: bool,
    tab: usize,
    printed: bool,
}

impl UiProbe {
    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let name = cx.new(|cx| InputState::new(window, cx).placeholder("DragonFruit printer"));
        Self {
            name,
            auto_save: true,
            experimental: false,
            tab: 0,
            printed: false,
        }
    }
}

impl Render for UiProbe {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if !self.printed {
            self.printed = true;
            eprintln!("ui-probe: rendered the settings panel");
        }
        v_flex()
            .size_full()
            .p_6()
            .gap_4()
            .child(
                h_flex()
                    .justify_between()
                    .items_center()
                    .child(Label::new("Settings").text_xl())
                    .child(Badge::new().count(3)),
            )
            .child(
                TabBar::new("tabs")
                    .underline()
                    .selected_index(self.tab)
                    .on_click(cx.listener(|this, index: &usize, _, cx| {
                        this.tab = *index;
                        cx.notify();
                    }))
                    .child(Tab::new().label("Printer"))
                    .child(Tab::new().label("Material"))
                    .child(Tab::new().label("Supports")),
            )
            .child(Separator::horizontal())
            .child(
                v_flex()
                    .gap_3()
                    .child(Label::new("Printer name").secondary("Shown in the printer library"))
                    .child(Input::new(&self.name))
                    .child(
                        Checkbox::new("experimental")
                            .label("Enable experimental features")
                            .checked(self.experimental)
                            .on_change(cx.listener(|this, checked: &bool, _, cx| {
                                this.experimental = *checked;
                                cx.notify();
                            })),
                    )
                    .child(
                        Switch::new("auto-save")
                            .label("Autosave scene")
                            .checked(self.auto_save)
                            .on_change(cx.listener(|this, checked: &bool, _, cx| {
                                this.auto_save = *checked;
                                cx.notify();
                            })),
                    ),
            )
            .child(Separator::horizontal())
            .child(Progress::new("progress").value(0.62))
            .child(
                h_flex()
                    .gap_2()
                    .child(Button::new("save").primary().label("Save"))
                    .child(Button::new("cancel").secondary().label("Cancel"))
                    .child(
                        Button::new("delete")
                            .danger()
                            .label("Delete")
                            .tooltip("Remove this printer"),
                    ),
            )
    }
}

fn main() {
    let app = gpui_kit::application().with_assets(Assets);
    app.run(move |cx| {
        gpui_kit::init(cx);
        let options = WindowOptions {
            window_bounds: Some(WindowBounds::centered(size(px(720.), px(560.)), cx)),
            ..Default::default()
        };
        gpui_kit::open_window(options, cx, |window, cx| {
            cx.new(|cx| UiProbe::new(window, cx))
        })
        .expect("a window");
        cx.activate(true);
    });
}
