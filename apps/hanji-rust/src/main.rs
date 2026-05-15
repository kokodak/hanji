use gpui::{
    App, Application, Bounds, Context, IntoElement, Render, Window, WindowBounds, WindowOptions,
    div, prelude::*, px, rgb, size,
};
use hanji_core::Document;

struct Hanji {
    document: Document,
}

impl Render for Hanji {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .gap_3()
            .bg(rgb(0xf8f7f2))
            .size(px(720.0))
            .justify_center()
            .items_center()
            .text_color(rgb(0x25231f))
            .child(div().text_xl().child("Hanji"))
            .child(div().child(self.document.text().to_owned()))
    }
}

fn main() {
    Application::new().run(|cx: &mut App| {
        let bounds = Bounds::centered(None, size(px(720.0), px(520.0)), cx);

        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                ..Default::default()
            },
            |_, cx| {
                cx.new(|_| Hanji {
                    document: Document::new("# Hanji\n\nCapture the thought."),
                })
            },
        )
        .unwrap();
    });
}
