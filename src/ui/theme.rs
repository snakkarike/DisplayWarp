#![allow(dead_code)]

use eframe::egui::{self, Color32, Margin, Vec2};

#[derive(Clone, Debug)]
pub struct Theme {
    pub dark_mode: bool,

    // ── Surfaces & Backgrounds ───────────────────────────────────────────────
    pub window_fill: Color32,
    pub panel_fill: Color32,
    pub card_bg: Color32,
    pub card_stroke: Color32,
    pub nested_bg: Color32,
    pub nested_stroke: Color32,
    pub input_bg: Color32,

    // ── Typography ───────────────────────────────────────────────────────────
    pub text_primary: Color32,
    pub text_secondary: Color32,
    pub text_muted: Color32,
    pub text_accent: Color32,

    // ── Accents & Interactive ────────────────────────────────────────────────
    pub accent_primary: Color32,
    pub accent_hover: Color32,
    pub accent_subtle_bg: Color32,
    pub button_bg: Color32,
    pub button_hover_bg: Color32,
    pub button_active_bg: Color32,

    // ── Metrics / Design Tokens ──────────────────────────────────────────────
    pub card_radius: f32,
    pub nested_radius: f32,
    pub button_radius: f32,
    pub card_padding: Margin,
    pub nested_padding: Margin,
    pub item_spacing: Vec2,
}

impl Theme {
    pub fn new(dark_mode: bool) -> Self {
        if dark_mode {
            Self {
                dark_mode: true,
                window_fill: Color32::from_rgb(14, 14, 14),
                panel_fill: Color32::from_rgb(14, 14, 14),
                card_bg: Color32::from_rgb(25, 25, 25),
                card_stroke: Color32::from_rgb(44, 44, 44),
                nested_bg: Color32::from_rgb(34, 34, 34),
                nested_stroke: Color32::from_rgb(52, 52, 52),
                input_bg: Color32::from_rgb(20, 20, 20),

                text_primary: Color32::WHITE,
                text_secondary: Color32::GRAY,
                text_muted: Color32::from_gray(100),
                text_accent: Color32::from_rgb(196, 181, 253),

                accent_primary: Color32::from_rgb(109, 40, 217),
                accent_hover: Color32::from_rgb(124, 58, 237),
                accent_subtle_bg: Color32::from_rgb(45, 40, 60),
                button_bg: Color32::from_rgb(25, 25, 25),
                button_hover_bg: Color32::from_rgb(40, 40, 40),
                button_active_bg: Color32::from_rgb(50, 50, 50),

                card_radius: 8.0,
                nested_radius: 6.0,
                button_radius: 6.0,
                card_padding: Margin::same(12),
                nested_padding: Margin::same(10),
                item_spacing: egui::vec2(8.0, 6.0),
            }
        } else {
            Self {
                dark_mode: false,
                window_fill: Color32::WHITE,
                panel_fill: Color32::WHITE,
                card_bg: Color32::from_rgb(248, 250, 252),
                card_stroke: Color32::from_rgb(226, 232, 240),
                nested_bg: Color32::from_rgb(241, 245, 249),
                nested_stroke: Color32::from_rgb(209, 213, 219),
                input_bg: Color32::from_rgb(255, 255, 255),

                text_primary: Color32::BLACK,
                text_secondary: Color32::from_gray(100),
                text_muted: Color32::from_gray(140),
                text_accent: Color32::from_rgb(109, 40, 217),

                accent_primary: Color32::from_rgb(124, 58, 237),
                accent_hover: Color32::from_rgb(109, 40, 217),
                accent_subtle_bg: Color32::from_rgb(237, 233, 254),
                button_bg: Color32::from_rgb(203, 213, 225),
                button_hover_bg: Color32::from_rgb(180, 195, 215),
                button_active_bg: Color32::from_rgb(160, 175, 195),

                card_radius: 8.0,
                nested_radius: 6.0,
                button_radius: 6.0,
                card_padding: Margin::same(12),
                nested_padding: Margin::same(10),
                item_spacing: egui::vec2(8.0, 6.0),
            }
        }
    }

    /// Apply theme styling to an active Ui and its Context.
    pub fn apply_to_ui(&self, ui: &mut egui::Ui) {
        self.apply_to_ctx(ui.ctx());
        self.apply_to_style(ui.style_mut());
    }

    /// Apply theme styling to the egui context.
    pub fn apply_to_ctx(&self, ctx: &egui::Context) {
        let theme_pref = if self.dark_mode {
            egui::ThemePreference::Dark
        } else {
            egui::ThemePreference::Light
        };
        ctx.set_theme(theme_pref);
        ctx.global_style_mut(|style| {
            self.apply_to_style(style);
        });
    }

    /// Mutate an egui Style in-place according to this theme.
    pub fn apply_to_style(&self, style: &mut egui::Style) {
        style.spacing.item_spacing = self.item_spacing;
        style.spacing.button_padding = egui::vec2(10.0, 5.0);
        style.spacing.combo_height = 300.0;
        style.visuals.widgets.noninteractive.corner_radius = egui::CornerRadius::same(self.button_radius as u8);
        style.visuals.widgets.inactive.corner_radius = egui::CornerRadius::same(self.button_radius as u8);
        style.visuals.widgets.hovered.corner_radius = egui::CornerRadius::same(self.button_radius as u8);
        style.visuals.widgets.active.corner_radius = egui::CornerRadius::same(self.button_radius as u8);

        if self.dark_mode {
            style.visuals = egui::Visuals::dark();
            style.visuals.window_fill = self.window_fill;
            style.visuals.panel_fill = self.panel_fill;
            style.visuals.widgets.inactive.bg_fill = Color32::from_rgb(7, 7, 7);
        } else {
            style.visuals = egui::Visuals::light();
            style.visuals.window_fill = self.window_fill;
            style.visuals.panel_fill = self.panel_fill;

            // Light text overrides
            style.visuals.override_text_color = Some(self.text_primary);
            style.visuals.widgets.noninteractive.fg_stroke.color = self.text_primary;
            style.visuals.widgets.inactive.fg_stroke.color = self.text_primary;
            style.visuals.widgets.hovered.fg_stroke.color = self.text_primary;
            style.visuals.widgets.active.fg_stroke.color = self.text_primary;

            // Light buttons
            style.visuals.widgets.inactive.bg_fill = self.button_bg;
            style.visuals.widgets.hovered.bg_fill = self.button_hover_bg;
            style.visuals.widgets.active.bg_fill = self.button_active_bg;
        }
    }
}
