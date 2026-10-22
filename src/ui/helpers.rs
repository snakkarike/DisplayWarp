#![allow(dead_code)]

use eframe::egui::{self, Color32, RichText, Ui};
use super::theme::Theme;

/// Reusable outer card container with standard rounded corners and theme stroke/fill.
pub fn card<R>(
    ui: &mut Ui,
    theme: &Theme,
    add_contents: impl FnOnce(&mut Ui) -> R,
) -> egui::InnerResponse<R> {
    egui::Frame::group(ui.style())
        .inner_margin(theme.card_padding)
        .corner_radius(egui::CornerRadius::same(theme.card_radius as u8))
        .fill(theme.card_bg)
        .stroke(egui::Stroke::new(1.0_f32, theme.card_stroke))
        .show(ui, add_contents)
}

/// Reusable nested/inset container for sub-sections and cards within cards.
pub fn nested_card<R>(
    ui: &mut Ui,
    theme: &Theme,
    add_contents: impl FnOnce(&mut Ui) -> R,
) -> egui::InnerResponse<R> {
    egui::Frame::group(ui.style())
        .inner_margin(theme.nested_padding)
        .corner_radius(egui::CornerRadius::same(theme.nested_radius as u8))
        .fill(theme.nested_bg)
        .stroke(egui::Stroke::new(1.0_f32, theme.nested_stroke))
        .show(ui, add_contents)
}

/// Reusable status/tag badge with custom fg/bg and rounded pill styling.
pub fn badge(ui: &mut Ui, text: impl Into<String>, fg: Color32, bg: Color32) -> egui::Response {
    let padding = egui::vec2(7.0, 2.5);
    let text = text.into();
    let font_id = egui::TextStyle::Small.resolve(ui.style());
    let galley = ui.painter().layout_no_wrap(text, font_id, fg);
    let desired_size = galley.size() + padding * 2.0;

    let (rect, response) = ui.allocate_exact_size(desired_size, egui::Sense::hover());
    if ui.is_rect_visible(rect) {
        ui.painter().rect_filled(rect, egui::CornerRadius::same(4), bg);
        let text_pos = rect.min + padding;
        ui.painter().galley(text_pos, galley, fg);
    }
    response
}

/// Reusable section title with icon, font styling, and optional secondary subtitle.
pub fn section_header(
    ui: &mut Ui,
    theme: &Theme,
    icon: &str,
    title: &str,
    subtitle: Option<&str>,
) {
    ui.horizontal(|ui| {
        ui.label(
            RichText::new(format!("{} {}", icon, title))
                .size(15.0)
                .strong(),
        );
    });
    if let Some(sub) = subtitle {
        ui.label(RichText::new(sub).small().color(theme.text_secondary));
    }
}

pub fn truncate_text(text: &str, max_chars: usize) -> String {
    if text.chars().count() > max_chars {
        text.chars().take(max_chars - 1).collect::<String>() + "…"
    } else {
        text.to_string()
    }
}
