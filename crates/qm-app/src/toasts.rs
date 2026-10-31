//! Transient notifications in the bottom-right corner (save errors, export results, …).

use egui::{Align2, Color32, RichText};
use qm_render::Theme;

const INFO_SECONDS: f64 = 4.0;
const ERROR_SECONDS: f64 = 10.0;
const MAX_TOAST_WIDTH: f32 = 380.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ToastKind {
    Success,
    Error,
}

struct Toast {
    kind: ToastKind,
    message: String,
    /// Set the first frame the toast is drawn, so its lifetime starts when it is seen.
    shown_since: Option<f64>,
}

#[derive(Default)]
pub struct Toasts {
    toasts: Vec<Toast>,
}

impl Toasts {
    pub fn success(&mut self, message: impl Into<String>) {
        self.push(ToastKind::Success, message.into());
    }

    pub fn error(&mut self, message: impl Into<String>) {
        self.push(ToastKind::Error, message.into());
    }

    fn push(&mut self, kind: ToastKind, message: String) {
        self.toasts.push(Toast {
            kind,
            message,
            shown_since: None,
        });
    }

    /// Draws the live toasts and drops expired ones. Clicking a toast dismisses it.
    pub fn show(&mut self, ctx: &egui::Context, theme: &Theme) {
        let now = ctx.input(|input| input.time);
        self.toasts.retain(|toast| {
            toast
                .shown_since
                .is_none_or(|since| now - since < lifetime(toast.kind))
        });
        if self.toasts.is_empty() {
            return;
        }
        let mut dismissed = None;
        egui::Area::new(egui::Id::new("toasts"))
            .anchor(Align2::RIGHT_BOTTOM, egui::vec2(-16.0, -40.0))
            .order(egui::Order::Foreground)
            .show(ctx, |ui| {
                for (index, toast) in self.toasts.iter_mut().enumerate() {
                    toast.shown_since.get_or_insert(now);
                    if show_toast(ui, toast, theme).clicked() {
                        dismissed = Some(index);
                    }
                }
            });
        if let Some(index) = dismissed {
            self.toasts.remove(index);
        }
        ctx.request_repaint_after(std::time::Duration::from_millis(250));
    }
}

fn lifetime(kind: ToastKind) -> f64 {
    match kind {
        ToastKind::Success => INFO_SECONDS,
        ToastKind::Error => ERROR_SECONDS,
    }
}

fn show_toast(ui: &mut egui::Ui, toast: &Toast, theme: &Theme) -> egui::Response {
    let (icon, accent): (&str, Color32) = match toast.kind {
        ToastKind::Success => ("✔", theme.accent_text),
        ToastKind::Error => ("⚠", theme.error),
    };
    let frame = egui::Frame::new()
        .fill(theme.surface_raised)
        .stroke(egui::Stroke::new(1.0, accent))
        .corner_radius(crate::theme_style::CORNER_RADIUS)
        .inner_margin(egui::Margin::symmetric(12, 8))
        .shadow(ui.visuals().popup_shadow);
    let response = frame
        .show(ui, |ui| {
            ui.set_max_width(MAX_TOAST_WIDTH);
            ui.horizontal(|ui| {
                ui.label(RichText::new(icon).color(accent));
                ui.add(egui::Label::new(RichText::new(&toast.message).color(theme.text)).wrap());
            });
        })
        .response;
    response
        .interact(egui::Sense::click())
        .on_hover_text("Click to dismiss")
}
