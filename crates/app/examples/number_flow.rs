//! Run `cargo run -p quarterdeck --example number_flow` to exercise the
//! production control without a GitHub account or the user's cache.

#[path = "../src/controls/number_flow/mod.rs"]
mod number_flow;
#[path = "../src/controls/progress_bar.rs"]
mod progress_bar;
#[path = "../src/controls/value_motion.rs"]
mod value_motion;

use std::borrow::Cow;

use gpui::{
    App, Application, Bounds, Context, FontWeight, TitlebarOptions, Window, WindowBounds,
    WindowOptions, div, prelude::*, px, rgb, size,
};
use number_flow::NumberFlow;
use progress_bar::ProgressBar;

const VALUES: [u32; 9] = [8, 9, 10, 99, 100, 1_000, 42, 0, 12];

struct Preview {
    index: usize,
    light: bool,
}

impl Render for Preview {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let value = VALUES[self.index];
        let (background, foreground, muted) = if self.light {
            (0x00f6_f5f1, 0x0029_2d30, 0x0062_676a)
        } else {
            (0x0019_1d20, 0x00e8_e8e5, 0x00a6_acb0)
        };
        div()
            .size_full()
            .p_8()
            .flex()
            .flex_col()
            .gap_6()
            .bg(rgb(background))
            .text_color(rgb(foreground))
            .font_family("Schibsted Grotesk")
            .child(
                div()
                    .text_xl()
                    .font_weight(FontWeight::SEMIBOLD)
                    .child("Number Flow · native preview"),
            )
            .child(
                div()
                    .text_size(px(64.))
                    .line_height(px(80.))
                    .child(NumberFlow::new("main", value.to_string())),
            )
            .child(
                div()
                    .text_sm()
                    .flex()
                    .gap_4()
                    .child(NumberFlow::new("mine", format!("Mine  {value}")))
                    .child(NumberFlow::new(
                        "review",
                        format!("Review  {}", 1_000 - value),
                    ))
                    .child(NumberFlow::new(
                        "progress",
                        format!("{value} of 1000 viewed"),
                    ))
                    .items_center()
                    .child(div().flex_1().child(ProgressBar::new(
                        "viewed-progress",
                        f32::from(u16::try_from(value).expect("preview values fit")) / 1_000.,
                        rgb(muted),
                        rgb(0x0060_a880),
                    ))),
            )
            .child(
                div()
                    .text_xs()
                    .flex()
                    .gap_4()
                    .child(
                        div()
                            .text_color(rgb(0x006c_a9e6))
                            .child(NumberFlow::label(format!("+{value}"))),
                    )
                    .child(
                        div()
                            .text_color(rgb(0x00db_a56b))
                            .child(NumberFlow::label(format!("−{}", 1_000 - value))),
                    )
                    .child(NumberFlow::new(
                        "decimal",
                        format!("${:.2}", f64::from(value) / 100.),
                    ))
                    .child(NumberFlow::new("percent", format!("{}%", value / 10))),
            )
            .child(
                div()
                    .w(px(200.))
                    .text_right()
                    .font_family("JetBrains Mono")
                    .child(NumberFlow::new("right", format!("{value} files"))),
            )
            .child(
                div()
                    .w(px(140.))
                    .truncate()
                    .text_sm()
                    .child(NumberFlow::new(
                        "truncated",
                        format!("{value} of 1000 files reviewed in this repository"),
                    )),
            )
            .child(div().w(px(250.)).text_sm().child(NumberFlow::new(
                "wrapped",
                format!("Read {value} of 1000 files while preparing the review."),
            )))
            .child(Self::repository_notes(value))
            .child(Self::controls(muted, cx))
            .child(div().text_xs().text_color(rgb(muted)).child(
                "Click quickly to interrupt a roll. macOS Reduce Motion takes effect immediately.",
            ))
    }
}

impl Preview {
    /// Repeated labels from one call site must retain independent transitions.
    fn repository_notes(value: u32) -> impl IntoElement {
        div()
            .flex()
            .gap_6()
            .text_xs()
            .children(
                [("repo-a", value), ("repo-b", 1_000 - value)].map(|(repo, total)| {
                    div()
                        .id(gpui::SharedString::from(format!("others-nothing-{repo}")))
                        .child(NumberFlow::label(format!("The filter hides all {total}")))
                }),
            )
    }

    fn controls(muted: u32, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .gap_3()
            .children(
                [("previous", "Previous", false), ("next", "Next", true)].map(
                    |(id, label, next)| {
                        div()
                            .id(id)
                            .px_3()
                            .py_2()
                            .border_1()
                            .border_color(rgb(muted))
                            .rounded_md()
                            .cursor_pointer()
                            .child(label)
                            .on_click(cx.listener(move |view, _, _, cx| {
                                view.index = if next {
                                    (view.index + 1) % VALUES.len()
                                } else {
                                    (view.index + VALUES.len() - 1) % VALUES.len()
                                };
                                cx.notify();
                            }))
                    },
                ),
            )
            .child(
                div()
                    .id("theme")
                    .px_3()
                    .py_2()
                    .cursor_pointer()
                    .child("Light / dark")
                    .on_click(cx.listener(|view, _, _, cx| {
                        view.light = !view.light;
                        cx.notify();
                    })),
            )
    }
}

fn main() {
    Application::new().run(|cx: &mut App| {
        cx.text_system()
            .add_fonts(vec![
                Cow::Borrowed(include_bytes!(
                    "../assets/fonts/SchibstedGrotesk-Regular.ttf"
                )),
                Cow::Borrowed(include_bytes!(
                    "../assets/fonts/SchibstedGrotesk-SemiBold.ttf"
                )),
                Cow::Borrowed(include_bytes!("../assets/fonts/JetBrainsMono-Regular.ttf")),
            ])
            .expect("preview fonts load");
        let bounds = Bounds::centered(None, size(px(680.), px(650.)), cx);
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                titlebar: Some(TitlebarOptions {
                    title: Some("Quarterdeck · Number Flow".into()),
                    ..Default::default()
                }),
                ..Default::default()
            },
            |_, cx| {
                cx.new(|_| Preview {
                    index: 0,
                    light: false,
                })
            },
        )
        .expect("preview opens");
        cx.activate(true);
    });
}
