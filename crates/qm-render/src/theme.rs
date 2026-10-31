//! Colour palette shared by the editor chrome and the preview.
//!
//! Neutrals follow the claude.ai website (warm cream / warm charcoal); the
//! brand accent is light green instead of claude.ai's clay orange.

use egui::Color32;

#[derive(Debug, Clone, PartialEq)]
pub struct Theme {
    pub is_dark: bool,
    /// Main content background (editor, preview page).
    pub background: Color32,
    /// Side panels and toolbars.
    pub surface: Color32,
    /// Hovered rows, selected tree items, inactive buttons.
    pub surface_raised: Color32,
    pub border: Color32,
    pub text: Color32,
    pub text_muted: Color32,
    pub text_faint: Color32,
    /// Primary buttons, active tab underline, focus rings.
    pub accent: Color32,
    /// Accent dark enough to read as text/links on `background`.
    pub accent_text: Color32,
    /// Text drawn on top of `accent`.
    pub on_accent: Color32,
    /// Subtle accent wash for selections and callout backgrounds.
    pub accent_soft: Color32,
    pub error: Color32,
    pub syntax: SyntaxColors,
}

/// Editor syntax-highlighting colours.
#[derive(Debug, Clone, PartialEq)]
pub struct SyntaxColors {
    pub heading: Color32,
    pub markup: Color32,
    pub math: Color32,
    pub math_keyword: Color32,
    pub math_number: Color32,
    pub math_operator: Color32,
    pub code: Color32,
    pub link: Color32,
    pub callout: Color32,
}

impl Theme {
    pub fn light() -> Theme {
        let accent_text = Color32::from_rgb(0x3F, 0x8A, 0x45);
        Theme {
            is_dark: false,
            background: Color32::from_rgb(0xFA, 0xF9, 0xF5),
            surface: Color32::from_rgb(0xF5, 0xF4, 0xED),
            surface_raised: Color32::from_rgb(0xF0, 0xEE, 0xE6),
            border: Color32::from_rgb(0xE5, 0xE3, 0xD8),
            text: Color32::from_rgb(0x14, 0x14, 0x13),
            text_muted: Color32::from_rgb(0x3D, 0x3D, 0x3A),
            text_faint: Color32::from_rgb(0x73, 0x72, 0x6C),
            accent: Color32::from_rgb(0x8F, 0xCF, 0x8C),
            accent_text,
            on_accent: Color32::from_rgb(0x14, 0x2B, 0x16),
            accent_soft: Color32::from_rgb(0xE6, 0xF3, 0xE3),
            error: Color32::from_rgb(0xB5, 0x3A, 0x2E),
            syntax: SyntaxColors {
                heading: Color32::from_rgb(0x14, 0x14, 0x13),
                markup: Color32::from_rgb(0x9C, 0x9A, 0x92),
                math: Color32::from_rgb(0x2F, 0x6B, 0x35),
                math_keyword: Color32::from_rgb(0x6A, 0x4C, 0x9C),
                math_number: Color32::from_rgb(0x9A, 0x5B, 0x1E),
                math_operator: Color32::from_rgb(0x55, 0x6B, 0x7A),
                code: Color32::from_rgb(0x73, 0x72, 0x6C),
                link: accent_text,
                callout: Color32::from_rgb(0x2E, 0x6E, 0x8E),
            },
        }
    }

    pub fn dark() -> Theme {
        let accent_text = Color32::from_rgb(0x9F, 0xD9, 0x9B);
        Theme {
            is_dark: true,
            background: Color32::from_rgb(0x26, 0x26, 0x24),
            surface: Color32::from_rgb(0x1F, 0x1E, 0x1D),
            surface_raised: Color32::from_rgb(0x30, 0x30, 0x2E),
            border: Color32::from_rgb(0x3E, 0x3E, 0x3A),
            text: Color32::from_rgb(0xFA, 0xF9, 0xF5),
            text_muted: Color32::from_rgb(0xC2, 0xC0, 0xB6),
            text_faint: Color32::from_rgb(0x9C, 0x9A, 0x92),
            accent: Color32::from_rgb(0x8F, 0xCF, 0x8C),
            accent_text,
            on_accent: Color32::from_rgb(0x14, 0x2B, 0x16),
            accent_soft: Color32::from_rgb(0x2F, 0x3D, 0x2E),
            error: Color32::from_rgb(0xE5, 0x7B, 0x6E),
            syntax: SyntaxColors {
                heading: Color32::from_rgb(0xFA, 0xF9, 0xF5),
                markup: Color32::from_rgb(0x7A, 0x78, 0x70),
                math: Color32::from_rgb(0xA8, 0xDB, 0xA4),
                math_keyword: Color32::from_rgb(0xC4, 0xA7, 0xF0),
                math_number: Color32::from_rgb(0xE8, 0xB4, 0x7A),
                math_operator: Color32::from_rgb(0x9F, 0xB8, 0xC8),
                code: Color32::from_rgb(0xC2, 0xC0, 0xB6),
                link: accent_text,
                callout: Color32::from_rgb(0x8C, 0xC4, 0xE0),
            },
        }
    }
}
