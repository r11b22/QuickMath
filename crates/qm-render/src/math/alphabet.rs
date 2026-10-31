//! Maps plain letters to the Unicode Mathematical Alphanumeric Symbols block,
//! so the math font draws real math italic / bold glyphs.

use std::borrow::Cow;

use qm_format::AtomStyle;

const GREEK_CAPITAL_ALPHA: u32 = 0x391;
const GREEK_CAPITAL_OMEGA: u32 = 0x3A9;
const GREEK_SMALL_ALPHA: u32 = 0x3B1;
const GREEK_SMALL_OMEGA: u32 = 0x3C9;

/// Applies an atom's style to its text.
pub fn styled_text(text: &str, style: AtomStyle) -> Cow<'_, str> {
    match style {
        AtomStyle::Upright => Cow::Borrowed(text),
        AtomStyle::Italic => Cow::Owned(text.chars().map(italic).collect()),
        AtomStyle::Bold => Cow::Owned(text.chars().map(bold).collect()),
    }
}

/// Math italic form of `c`. Capital Greek stays upright, as in TeX.
pub fn italic(c: char) -> char {
    let code = c as u32;
    let mapped = match c {
        // U+1D455 is a hole in the block: italic h is the Planck constant.
        'h' => Some(0x210E),
        'A'..='Z' => Some(0x1D434 + code - 'A' as u32),
        'a'..='z' => Some(0x1D44E + code - 'a' as u32),
        'ı' => Some(0x1D6A4),
        'ȷ' => Some(0x1D6A5),
        _ if (GREEK_SMALL_ALPHA..=GREEK_SMALL_OMEGA).contains(&code) => {
            Some(0x1D6FC + code - GREEK_SMALL_ALPHA)
        }
        '∂' => Some(0x1D715),
        'ϵ' => Some(0x1D716),
        'ϑ' => Some(0x1D717),
        'ϰ' => Some(0x1D718),
        'ϕ' => Some(0x1D719),
        'ϱ' => Some(0x1D71A),
        'ϖ' => Some(0x1D71B),
        _ => None,
    };
    mapped.and_then(char::from_u32).unwrap_or(c)
}

/// Math bold (upright) form of `c`.
pub fn bold(c: char) -> char {
    let code = c as u32;
    let mapped = match c {
        'A'..='Z' => Some(0x1D400 + code - 'A' as u32),
        'a'..='z' => Some(0x1D41A + code - 'a' as u32),
        '0'..='9' => Some(0x1D7CE + code - '0' as u32),
        // U+03A2 is unassigned, which lines the capitals up with the block's
        // own gap (it holds bold capital theta symbol there).
        _ if (GREEK_CAPITAL_ALPHA..=GREEK_CAPITAL_OMEGA).contains(&code) => {
            Some(0x1D6A8 + code - GREEK_CAPITAL_ALPHA)
        }
        _ if (GREEK_SMALL_ALPHA..=GREEK_SMALL_OMEGA).contains(&code) => {
            Some(0x1D6C2 + code - GREEK_SMALL_ALPHA)
        }
        '∇' => Some(0x1D6C1),
        '∂' => Some(0x1D6DB),
        'ϵ' => Some(0x1D6DC),
        'ϑ' => Some(0x1D6DD),
        'ϰ' => Some(0x1D6DE),
        'ϕ' => Some(0x1D6DF),
        'ϱ' => Some(0x1D6E0),
        'ϖ' => Some(0x1D6E1),
        _ => None,
    };
    mapped.and_then(char::from_u32).unwrap_or(c)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn italic_maps_latin_and_skips_the_planck_hole() {
        assert_eq!(italic('a'), '\u{1D44E}');
        assert_eq!(italic('z'), '\u{1D467}');
        assert_eq!(italic('A'), '\u{1D434}');
        assert_eq!(italic('h'), 'ℎ');
        assert_eq!(italic('g'), '\u{1D454}');
        assert_eq!(italic('i'), '\u{1D456}');
    }

    #[test]
    fn italic_maps_lowercase_greek_but_keeps_capitals_upright() {
        assert_eq!(italic('α'), '\u{1D6FC}');
        assert_eq!(italic('ω'), '\u{1D714}');
        assert_eq!(italic('ϕ'), '\u{1D719}');
        assert_eq!(italic('Γ'), 'Γ');
    }

    #[test]
    fn bold_maps_latin_digits_and_greek() {
        assert_eq!(bold('A'), '\u{1D400}');
        assert_eq!(bold('x'), '\u{1D431}');
        assert_eq!(bold('7'), '\u{1D7D5}');
        assert_eq!(bold('Σ'), '\u{1D6BA}');
        assert_eq!(bold('α'), '\u{1D6C2}');
    }

    #[test]
    fn non_letters_pass_through() {
        assert_eq!(italic('+'), '+');
        assert_eq!(bold('≤'), '≤');
        assert_eq!(styled_text("sin", AtomStyle::Upright), "sin");
    }
}
