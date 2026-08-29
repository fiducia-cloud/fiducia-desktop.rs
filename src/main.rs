#![forbid(unsafe_code)]

use fiducia_desktop::{AppLifecycleMachine, AppPhase, DeepLinkAdmissionMachine};
use gpui::{
    App, Application, Context, IntoElement, Render, Window, WindowOptions, div, prelude::*, px, rgb,
};

struct FiduciaDesktop {
    lifecycle: AppLifecycleMachine,
    deep_links: DeepLinkAdmissionMachine,
    deep_link_audit: String,
}

impl Default for FiduciaDesktop {
    fn default() -> Self {
        let mut deep_links = DeepLinkAdmissionMachine::default();
        let mut deep_link_audit = "no deep link captured".to_owned();
        if let Some(raw) = std::env::args()
            .skip(1)
            .find(|argument| argument.contains("://"))
        {
            let begin = deep_links.begin(&raw);
            let completed = match begin.effect {
                Some(effect) => deep_links.complete(effect.generation),
                None => begin,
            };
            deep_link_audit = format!(
                "deep link {:?} · {}",
                completed.disposition,
                completed.reason.wire_name()
            );
        }
        Self {
            lifecycle: AppLifecycleMachine::default(),
            deep_links,
            deep_link_audit,
        }
    }
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
        let deep_link = format!(
            "{} · phase {:?} · generation {}",
            self.deep_link_audit,
            self.deep_links.snapshot().phase(),
            self.deep_links.snapshot().generation()
        );

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
            .child(div().child(deep_link))
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
