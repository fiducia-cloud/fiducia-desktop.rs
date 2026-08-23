#![forbid(unsafe_code)]

use fiducia_desktop::{AppLifecycleMachine, AppPhase};
use gpui::{
    App, Application, Context, IntoElement, Render, Window, WindowOptions, div, prelude::*, px, rgb,
};

#[derive(Default)]
struct FiduciaDesktop {
    lifecycle: AppLifecycleMachine,
}

impl Render for FiduciaDesktop {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let snapshot = self.lifecycle.snapshot();
        let status = format!(
            "{:?} · generation {} · authority {}",
            snapshot.phase(),
            snapshot.generation(),
            snapshot.authority_epoch()
        );
        let authority = if snapshot.phase() == AppPhase::ReadyOnline {
            "Protected actions available after confirmation"
        } else {
            "Protected actions unavailable"
        };

        div()
            .flex()
            .flex_col()
            .gap_3()
            .size_full()
            .p_8()
            .bg(rgb(0x071411))
            .text_color(rgb(0xe6fff8))
            .child(div().text_size(px(30.0)).child("Fiducia desktop"))
            .child(div().child(status))
            .child(div().text_color(rgb(0x62d9c2)).child(authority))
    }
}

fn main() {
    Application::new().run(|cx: &mut App| {
        cx.open_window(WindowOptions::default(), |_window, cx| {
            cx.new(|_cx| FiduciaDesktop::default())
        })
        .expect("the native Fiducia window must open");
        cx.activate(true);
    });
}
