//! A DragonFruit panel rebuilt in GPUI, to compare design fidelity.
//!
//! Run with `cargo run --bin panel`.
//!
//! The target is `src/components/controls/AutoSupportPanel.tsx` and the tokens
//! in `src/app/globals.css`: surface-0 card, 1px border-subtle, 8px radius,
//! 10px header padding, the accent-tinted run button, the three-up island
//! count card, and the preset / Auto-Lift row.
//!
//! Deliberately raw GPUI `div`s rather than `gpui-component` widgets: the point
//! is whether the app's own visual language can be reproduced, not whether the
//! library's default look can.

use gpui_kit::component::Icon;
use gpui_kit::*;

// DragonFruit dark-theme tokens (src/app/globals.css).
const BACKGROUND: u32 = 0x0b0f14;
const SURFACE_0: u32 = 0x111216;
const SURFACE_1: u32 = 0x1a1b21;
const BORDER_SUBTLE: u32 = 0x272a33;
const TEXT_STRONG: u32 = 0xf8f8fb;
const TEXT_MUTED: u32 = 0xc3c7cf;
const ACCENT: u32 = 0xec2a77;

// color-mix(in srgb, var(--accent), var(--surface-0) 86%)
const RUN_FILL: u32 = 0x301524;

const CHEVRON_DOWN: &str = r#"<path d="m6 9 6 6 6-6"/>"#;
const SETTINGS: &str = r#"<path d="M12.22 2h-.44a2 2 0 0 0-2 2v.18a2 2 0 0 1-1 1.73l-.43.25a2 2 0 0 1-2 0l-.15-.08a2 2 0 0 0-2.73.73l-.22.38a2 2 0 0 0 .73 2.73l.15.1a2 2 0 0 1 1 1.72v.51a2 2 0 0 1-1 1.74l-.15.09a2 2 0 0 0-.73 2.73l.22.38a2 2 0 0 0 2.73.73l.15-.08a2 2 0 0 1 2 0l.43.25a2 2 0 0 1 1 1.73V20a2 2 0 0 0 2 2h.44a2 2 0 0 0 2-2v-.18a2 2 0 0 1 1-1.73l.43-.25a2 2 0 0 1 2 0l.15.08a2 2 0 0 0 2.73-.73l.22-.39a2 2 0 0 0-.73-2.73l-.15-.08a2 2 0 0 1-1-1.74v-.5a2 2 0 0 1 1-1.74l.15-.09a2 2 0 0 0 .73-2.73l-.22-.38a2 2 0 0 0-2.73-.73l-.15.08a2 2 0 0 1-2 0l-.43-.25a2 2 0 0 1-1-1.73V4a2 2 0 0 0-2-2z"/><circle cx="12" cy="12" r="3"/>"#;

struct PanelProbe {
    printed: bool,
}

impl Render for PanelProbe {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        if !self.printed {
            self.printed = true;
            eprintln!("panel: rendered the Auto Supports panel");
        }
        div()
            .size_full()
            .bg(rgb(BACKGROUND))
            .flex()
            .items_center()
            .justify_center()
            .child(
                div()
                    .flex()
                    .items_start()
                    .gap(px(8.))
                    .child(auto_support_panel())
                    .child(layer_rail()),
            )
    }
}

/// The narrow layer rail the real screenshot has beside the panel.
fn layer_rail() -> Div {
    div()
        .w(px(40.))
        .h(px(300.))
        .bg(rgb(SURFACE_0))
        .border_1()
        .border_color(rgb(BORDER_SUBTLE))
        .rounded(px(8.))
        .relative()
        .flex()
        .justify_center()
        .py(px(10.))
        .child(
            div()
                .w(px(6.))
                .h_full()
                .bg(rgb(0x23252e))
                .rounded(px(3.))
                .relative()
                .child(
                    div()
                        .absolute()
                        .bottom(px(0.))
                        .left(px(0.))
                        .w_full()
                        .h(px(210.))
                        .bg(rgb(ACCENT))
                        .rounded(px(3.)),
                ),
        )
        .child(
            div()
                .absolute()
                .top(px(58.))
                .left(px(5.))
                .px(px(4.))
                .py(px(2.))
                .rounded(px(4.))
                .bg(rgb(ACCENT))
                .text_size(px(10.))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(rgb(0xffffff))
                .child("960"),
        )
}

fn icon(path: &str, color: u32, size: f32) -> Icon {
    let svg = format!(
        r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="#{color:06x}" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">{path}</svg>"##
    );
    Icon::default()
        .data(svg.as_bytes())
        .w(px(size))
        .h(px(size))
}

fn auto_support_panel() -> Div {
    div()
        .w(px(320.))
        .bg(rgb(SURFACE_0))
        .border_1()
        .border_color(rgb(BORDER_SUBTLE))
        .rounded(px(8.))
        .shadow_lg()
        .flex()
        .flex_col()
        .child(panel_header())
        .child(panel_body())
}

fn icon_tile(path: &str, color: u32, size: f32, tile: f32) -> Div {
    div()
        .w(px(tile))
        .h(px(tile))
        .flex()
        .items_center()
        .justify_center()
        .rounded(px(6.))
        .border_1()
        .border_color(rgb(BORDER_SUBTLE))
        .bg(rgb(SURFACE_1))
        .child(icon(path, color, size))
}

fn panel_header() -> Div {
    div()
        .px(px(10.))
        .py(px(10.))
        .flex()
        .items_center()
        .gap(px(10.))
        .child(icon_tile(CHEVRON_DOWN, ACCENT, 12., 28.))
        .child(
            div()
                .text_size(px(13.))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(rgb(TEXT_STRONG))
                .child("Auto Supports (Beta)"),
        )
        .child(div().flex_1())
        .child(icon_tile(SETTINGS, TEXT_MUTED, 14., 26.))
}

fn panel_body() -> Div {
    div()
        .px(px(10.))
        .pb(px(12.))
        .flex()
        .flex_col()
        .gap(px(10.))
        .child(run_button())
        .child(island_counts())
        .child(preset_row())
}

fn run_button() -> Div {
    div()
        .w_full()
        .h(px(32.))
        .flex()
        .items_center()
        .justify_center()
        .rounded(px(6.))
        .border_1()
        .border_color(rgb(ACCENT))
        .bg(rgb(RUN_FILL))
        .text_size(px(11.))
        .font_weight(FontWeight::SEMIBOLD)
        .text_color(rgb(ACCENT))
        .child("Generate Supports")
}

fn island_counts() -> Div {
    div()
        .rounded(px(6.))
        .border_1()
        .border_color(rgb(BORDER_SUBTLE))
        .bg(rgb(SURFACE_1))
        .p(px(8.))
        .flex()
        .gap(px(8.))
        .child(count_cell("VOXEL", "15", false))
        .child(count_cell("MINIMA", "2", false))
        .child(count_cell("TOTAL", "21", true))
}

fn count_cell(label: &str, value: &str, accent: bool) -> Div {
    div()
        .flex_1()
        .flex()
        .flex_col()
        .items_center()
        .gap(px(2.))
        .child(
            div()
                .text_size(px(10.))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(rgb(TEXT_MUTED))
                .child(label.to_owned()),
        )
        .child(
            div()
                .text_size(px(13.))
                .font_weight(FontWeight::BOLD)
                .text_color(if accent { rgb(ACCENT) } else { rgb(TEXT_STRONG) })
                .child(value.to_owned()),
        )
}

fn preset_row() -> Div {
    div()
        .flex()
        .gap(px(6.))
        .child(preset_select())
        .child(auto_lift_button())
}

fn preset_select() -> Div {
    div()
        .flex_1()
        .h(px(32.))
        .px(px(10.))
        .flex()
        .items_center()
        .justify_between()
        .rounded(px(6.))
        .border_1()
        .border_color(rgb(BORDER_SUBTLE))
        .bg(rgb(SURFACE_1))
        .child(
            div()
                .text_size(px(13.))
                .text_color(rgb(TEXT_STRONG))
                .child("Medium"),
        )
        .child(icon(CHEVRON_DOWN, TEXT_MUTED, 12.))
}

fn auto_lift_button() -> Div {
    div()
        .flex_1()
        .h(px(32.))
        .flex()
        .items_center()
        .justify_center()
        .rounded(px(6.))
        .border_1()
        .border_color(rgb(BORDER_SUBTLE))
        .bg(rgb(SURFACE_1))
        .text_size(px(11.))
        .font_weight(FontWeight::SEMIBOLD)
        .text_color(rgb(TEXT_STRONG))
        .child("Auto-Lift OFF")
}

fn main() {
    gpui_kit::application().run(|cx| {
        gpui_kit::init(cx);
        let options = WindowOptions {
            window_bounds: Some(WindowBounds::centered(size(px(460.), px(520.)), cx)),
            ..Default::default()
        };
        gpui_kit::open_window(options, cx, |_, cx| {
            cx.new(|_| PanelProbe { printed: false })
        })
        .expect("a window");
        cx.activate(true);
    });
}
