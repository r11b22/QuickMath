//! Font registration: the New Computer Modern family bundled by `typst-assets`.
//!
//! The renderer draws everything with named egui font families so the app's
//! own `Proportional`/`Monospace` fonts are left untouched.

use std::sync::Arc;

use egui::{FontData, FontDefinitions, FontFamily};

/// OpenType math font (NewCMMath): symbols, math alphanumerics, operators.
pub const MATH: &str = "qm-math";
/// Upright text serif (New Computer Modern 10).
pub const SERIF: &str = "qm-serif";
pub const SERIF_ITALIC: &str = "qm-serif-italic";
pub const SERIF_BOLD: &str = "qm-serif-bold";
pub const SERIF_BOLD_ITALIC: &str = "qm-serif-bold-italic";
/// Monospace (DejaVu Sans Mono) for code.
pub const MONO: &str = "qm-mono";

/// The egui family for one of the names above.
pub fn family(name: &str) -> FontFamily {
    FontFamily::Name(name.into())
}

/// Which bundled face (by PostScript name) is the primary font of which family.
const FACES: [(&str, &str); 6] = [
    ("NewCMMath-Regular", MATH),
    ("NewCM10-Regular", SERIF),
    ("NewCM10-Italic", SERIF_ITALIC),
    ("NewCM10-Bold", SERIF_BOLD),
    ("NewCM10-BoldItalic", SERIF_BOLD_ITALIC),
    ("DejaVuSansMono", MONO),
];

const MATH_FACE: &str = "NewCMMath-Regular";
const TEXT_FACE: &str = "NewCM10-Regular";

/// Registers the renderer's font families in `fonts`.
///
/// Every family falls back to the math font (so stray symbols in prose still
/// render) and then to whatever `Proportional` already used (e.g. emoji).
pub fn install_fonts(fonts: &mut FontDefinitions) {
    let mut found = Vec::new();
    for data in typst_assets::fonts() {
        let Some(name) = postscript_name(data) else {
            continue;
        };
        let Some(&(face, family_name)) = FACES.iter().find(|(face, _)| *face == name) else {
            continue;
        };
        fonts
            .font_data
            .insert(face_key(face), Arc::new(FontData::from_static(data)));
        found.push((face, family_name));
    }
    debug_assert_eq!(
        found.len(),
        FACES.len(),
        "typst-assets no longer ships every expected font face"
    );

    let proportional_fallbacks = fonts
        .families
        .get(&FontFamily::Proportional)
        .cloned()
        .unwrap_or_default();
    let is_found = |face: &str| found.iter().any(|(f, _)| *f == face);
    for &(face, family_name) in &found {
        let mut chain = vec![face_key(face)];
        for fallback in [MATH_FACE, TEXT_FACE] {
            if fallback != face && is_found(fallback) {
                chain.push(face_key(fallback));
            }
        }
        chain.extend(proportional_fallbacks.iter().cloned());
        fonts.families.insert(family(family_name), chain);
    }
}

fn face_key(face: &str) -> String {
    format!("qm:{face}")
}

/// Reads the PostScript name (name ID 6) from an OpenType/TrueType file.
///
/// `typst_assets::fonts()` yields anonymous byte slices, so this is how faces
/// are identified without depending on the iterator's order.
fn postscript_name(font: &[u8]) -> Option<String> {
    let table_count = read_u16(font, 4)? as usize;
    let name_table = (0..table_count).find_map(|i| {
        let record = 12 + 16 * i;
        (font.get(record..record + 4)? == b"name")
            .then(|| read_u32(font, record + 8))
            .flatten()
    })? as usize;

    let record_count = read_u16(font, name_table + 2)? as usize;
    let strings = name_table + read_u16(font, name_table + 4)? as usize;
    (0..record_count).find_map(|i| {
        let record = name_table + 6 + 12 * i;
        let platform = read_u16(font, record)?;
        if read_u16(font, record + 6)? != 6 {
            return None;
        }
        let length = read_u16(font, record + 8)? as usize;
        let start = strings + read_u16(font, record + 10)? as usize;
        let bytes = font.get(start..start + length)?;
        match platform {
            // Macintosh Roman: PostScript names are plain ASCII.
            1 => Some(bytes.iter().map(|&b| b as char).collect()),
            // Unicode / Windows: UTF-16BE.
            0 | 3 => {
                let units: Vec<u16> = bytes
                    .chunks_exact(2)
                    .map(|c| u16::from_be_bytes([c[0], c[1]]))
                    .collect();
                String::from_utf16(&units).ok()
            }
            _ => None,
        }
    })
}

fn read_u16(bytes: &[u8], at: usize) -> Option<u16> {
    Some(u16::from_be_bytes(bytes.get(at..at + 2)?.try_into().ok()?))
}

fn read_u32(bytes: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_be_bytes(bytes.get(at..at + 4)?.try_into().ok()?))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registers_every_family_with_math_fallback() {
        let mut fonts = FontDefinitions::default();
        install_fonts(&mut fonts);
        for (face, family_name) in FACES {
            let chain = &fonts.families[&family(family_name)];
            assert_eq!(chain[0], face_key(face));
            assert!(fonts.font_data.contains_key(&face_key(face)));
            assert!(chain.contains(&face_key(MATH_FACE)));
        }
    }

    #[test]
    fn reads_postscript_names_of_bundled_fonts() {
        let names: Vec<String> = typst_assets::fonts().filter_map(postscript_name).collect();
        assert!(names.iter().any(|n| n == "NewCMMath-Regular"));
        assert!(names.iter().any(|n| n == "NewCM10-BoldItalic"));
    }

    #[test]
    fn postscript_name_rejects_garbage() {
        assert_eq!(postscript_name(&[]), None);
        assert_eq!(postscript_name(&[0, 1, 0, 0, 0, 9]), None);
    }
}
