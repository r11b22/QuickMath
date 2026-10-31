//! Maps the shared [`Theme`] palette onto egui's global `Style`/`Visuals`.

use egui::style::{Selection, WidgetVisuals, Widgets};
use egui::{Color32, CornerRadius, Shadow, Stroke, Visuals};
use qm_render::Theme;

/// Corner radius used for buttons, inputs, windows and menus.
pub const CORNER_RADIUS: u8 = 6;

/// Installs the light and dark styles and switches egui to the one matching `is_dark`.
pub fn apply_theme(ctx: &egui::Context, is_dark: bool) {
    ctx.set_style_of(egui::Theme::Light, build_style(&Theme::light()));
    ctx.set_style_of(egui::Theme::Dark, build_style(&Theme::dark()));
    let preference = if is_dark {
        egui::ThemePreference::Dark
    } else {
        egui::ThemePreference::Light
    };
    ctx.set_theme(preference);
}

/// The app palette for the current mode.
pub fn palette(is_dark: bool) -> Theme {
    if is_dark {
        Theme::dark()
    } else {
        Theme::light()
    }
}

fn build_style(theme: &Theme) -> egui::Style {
    let mut style = egui::Style {
        visuals: build_visuals(theme),
        ..egui::Style::default()
    };
    let spacing = &mut style.spacing;
    spacing.item_spacing = egui::vec2(8.0, 6.0);
    spacing.button_padding = egui::vec2(10.0, 5.0);
    spacing.menu_margin = egui::Margin::same(8);
    spacing.window_margin = egui::Margin::same(16);
    spacing.interact_size.y = 26.0;
    style
}

fn build_visuals(theme: &Theme) -> Visuals {
    let base = if theme.is_dark {
        Visuals::dark()
    } else {
        Visuals::light()
    };
    let corner_radius = CornerRadius::same(CORNER_RADIUS);
    let shadow_color = Color32::from_black_alpha(if theme.is_dark { 96 } else { 24 });
    let shadow = Shadow {
        offset: [0, 4],
        blur: 16,
        spread: 0,
        color: shadow_color,
    };
    let text_cursor = egui::style::TextCursorStyle {
        stroke: Stroke::new(2.0, theme.accent_text),
        ..base.text_cursor.clone()
    };
    Visuals {
        dark_mode: theme.is_dark,
        text_cursor,
        override_text_color: None,
        widgets: build_widgets(theme, corner_radius),
        selection: Selection {
            bg_fill: theme.accent_soft,
            stroke: Stroke::new(1.0, theme.accent_text),
        },
        hyperlink_color: theme.accent_text,
        faint_bg_color: theme.surface,
        extreme_bg_color: theme.background,
        text_edit_bg_color: Some(theme.background),
        code_bg_color: theme.surface_raised,
        warn_fg_color: theme.syntax.math_number,
        error_fg_color: theme.error,
        window_corner_radius: CornerRadius::same(CORNER_RADIUS * 2),
        window_shadow: shadow,
        window_fill: theme.background,
        window_stroke: Stroke::new(1.0, theme.border),
        menu_corner_radius: corner_radius,
        panel_fill: theme.surface,
        popup_shadow: shadow,
        ..base
    }
}

fn build_widgets(theme: &Theme, corner_radius: CornerRadius) -> Widgets {
    let widget = |fill: Color32, stroke: Color32, text: Color32| WidgetVisuals {
        bg_fill: fill,
        weak_bg_fill: fill,
        bg_stroke: Stroke::new(1.0, stroke),
        corner_radius,
        fg_stroke: Stroke::new(1.0, text),
        expansion: 0.0,
    };
    Widgets {
        noninteractive: WidgetVisuals {
            bg_stroke: Stroke::new(1.0, theme.border),
            ..widget(theme.surface, theme.border, theme.text_muted)
        },
        inactive: widget(theme.surface_raised, theme.surface_raised, theme.text),
        hovered: widget(theme.border, theme.border, theme.text),
        active: widget(theme.accent_soft, theme.accent, theme.text),
        open: widget(theme.surface_raised, theme.border, theme.text),
    }
}
