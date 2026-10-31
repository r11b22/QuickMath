//! TeX math styles, inter-atom spacing and the font's math constants.

use qm_format::{AtomClass, MathNode};

/// TeX's four math styles plus the "cramped" flag (exponents sit lower).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MathStyle {
    pub level: Level,
    pub cramped: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Level {
    Display,
    Text,
    Script,
    ScriptScript,
}

impl MathStyle {
    pub const DISPLAY: MathStyle = MathStyle {
        level: Level::Display,
        cramped: false,
    };
    pub const TEXT: MathStyle = MathStyle {
        level: Level::Text,
        cramped: false,
    };
    pub const SCRIPT_SCRIPT: MathStyle = MathStyle {
        level: Level::ScriptScript,
        cramped: false,
    };

    /// Font size relative to the surrounding text (NewCMMath's script percentages).
    pub fn scale(self) -> f32 {
        match self.level {
            Level::Display | Level::Text => 1.0,
            Level::Script => SCRIPT_SCALE,
            Level::ScriptScript => SCRIPT_SCRIPT_SCALE,
        }
    }

    pub fn is_display(self) -> bool {
        self.level == Level::Display
    }

    pub fn is_script(self) -> bool {
        matches!(self.level, Level::Script | Level::ScriptScript)
    }

    pub fn cramped(self) -> MathStyle {
        MathStyle {
            cramped: true,
            ..self
        }
    }

    pub fn superscript(self) -> MathStyle {
        let level = match self.level {
            Level::Display | Level::Text => Level::Script,
            Level::Script | Level::ScriptScript => Level::ScriptScript,
        };
        MathStyle { level, ..self }
    }

    pub fn subscript(self) -> MathStyle {
        self.superscript().cramped()
    }

    pub fn numerator(self) -> MathStyle {
        let level = match self.level {
            Level::Display => Level::Text,
            Level::Text => Level::Script,
            Level::Script | Level::ScriptScript => Level::ScriptScript,
        };
        MathStyle { level, ..self }
    }

    pub fn denominator(self) -> MathStyle {
        self.numerator().cramped()
    }
}

pub const SCRIPT_SCALE: f32 = 0.70;
pub const SCRIPT_SCRIPT_SCALE: f32 = 0.50;

/// Constants from NewCMMath's OpenType MATH table, in em.
pub mod constants {
    pub const AXIS_HEIGHT: f32 = 0.250;
    pub const ACCENT_BASE_HEIGHT: f32 = 0.450;

    pub const FRACTION_RULE: f32 = 0.040;
    pub const NUMERATOR_SHIFT_UP: f32 = 0.394;
    pub const NUMERATOR_SHIFT_UP_DISPLAY: f32 = 0.677;
    pub const DENOMINATOR_SHIFT_DOWN: f32 = 0.345;
    pub const DENOMINATOR_SHIFT_DOWN_DISPLAY: f32 = 0.686;
    pub const FRACTION_GAP_MIN: f32 = 0.040;
    pub const FRACTION_GAP_MIN_DISPLAY: f32 = 0.120;

    pub const SUPERSCRIPT_SHIFT_UP: f32 = 0.363;
    pub const SUPERSCRIPT_SHIFT_UP_CRAMPED: f32 = 0.289;
    pub const SUPERSCRIPT_BOTTOM_MIN: f32 = 0.108;
    pub const SUPERSCRIPT_BASELINE_DROP_MAX: f32 = 0.250;
    pub const SUPERSCRIPT_BOTTOM_MAX_WITH_SUBSCRIPT: f32 = 0.344;
    pub const SUBSCRIPT_SHIFT_DOWN: f32 = 0.247;
    pub const SUBSCRIPT_TOP_MAX: f32 = 0.344;
    pub const SUBSCRIPT_BASELINE_DROP_MIN: f32 = 0.200;
    pub const SUB_SUPERSCRIPT_GAP_MIN: f32 = 0.160;
    pub const SPACE_AFTER_SCRIPT: f32 = 0.056;

    pub const UPPER_LIMIT_GAP_MIN: f32 = 0.200;
    pub const UPPER_LIMIT_BASELINE_RISE_MIN: f32 = 0.111;
    pub const LOWER_LIMIT_GAP_MIN: f32 = 0.167;
    pub const LOWER_LIMIT_BASELINE_DROP_MIN: f32 = 0.600;

    pub const RADICAL_VERTICAL_GAP: f32 = 0.050;
    pub const RADICAL_VERTICAL_GAP_DISPLAY: f32 = 0.148;
    pub const RADICAL_RULE: f32 = 0.040;
    pub const RADICAL_EXTRA_ASCENDER: f32 = 0.040;
    pub const RADICAL_KERN_BEFORE_DEGREE: f32 = 0.278;
    pub const RADICAL_KERN_AFTER_DEGREE: f32 = -0.556;
    pub const RADICAL_DEGREE_BOTTOM_RAISE: f32 = 0.60;

    /// TeX's `\delimiterfactor` (901) and `\delimitershortfall` (5pt at 10pt).
    pub const DELIMITER_FACTOR: f32 = 0.901;
    pub const DELIMITER_SHORTFALL: f32 = 0.5;
    /// TeX's `\nulldelimiterspace` (1.2pt at 10pt).
    pub const NULL_DELIMITER_SPACE: f32 = 0.12;
}

/// TeX atom types, used for inter-atom spacing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpacingClass {
    Ord,
    Op,
    Bin,
    Rel,
    Open,
    Close,
    Punct,
    Inner,
}

impl SpacingClass {
    fn index(self) -> usize {
        self as usize
    }
}

/// The spacing class a node takes part in; `None` for explicit spaces, which
/// TeX ignores when choosing the glue between atoms.
pub fn spacing_class(node: &MathNode) -> Option<SpacingClass> {
    Some(match node {
        MathNode::Atom(atom) => match atom.class {
            AtomClass::Ordinary => SpacingClass::Ord,
            AtomClass::LargeOperator { .. } => SpacingClass::Op,
            AtomClass::Binary => SpacingClass::Bin,
            AtomClass::Relation => SpacingClass::Rel,
            AtomClass::Open => SpacingClass::Open,
            AtomClass::Close => SpacingClass::Close,
            AtomClass::Punctuation => SpacingClass::Punct,
        },
        MathNode::Scripts { base, .. } => spacing_class(base).unwrap_or(SpacingClass::Ord),
        MathNode::Fraction { .. } => SpacingClass::Inner,
        // TeX makes `\left…\right` Inner, which puts a thin space in `f(x)`.
        // Notes write plain parentheses, so fenced groups space like an
        // ordinary atom: `f(x)`, `(a+b)(a-b)`, `2(x+1)`.
        MathNode::Fenced { .. } => SpacingClass::Ord,
        MathNode::Matrix { open, close, .. } if !(open.is_empty() && close.is_empty()) => {
            SpacingClass::Inner
        }
        MathNode::Space(_) => return None,
        _ => SpacingClass::Ord,
    })
}

/// TeXbook chapter 18 table, in mu. Negative entries apply only outside
/// script styles ("(n)" in the book).
const SPACING_TABLE: [[i8; 8]; 8] = [
    //  Ord Op  Bin Rel Open Close Punct Inner
    [0, 3, -4, -5, 0, 0, 0, -3],     // Ord
    [3, 3, 0, -5, 0, 0, 0, -3],      // Op
    [-4, -4, 0, 0, -4, 0, 0, -4],    // Bin
    [-5, -5, 0, 0, -5, 0, 0, -5],    // Rel
    [0, 0, 0, 0, 0, 0, 0, 0],        // Open
    [0, 3, -4, -5, 0, 0, 0, -3],     // Close
    [-3, -3, 0, -3, -3, -3, -3, -3], // Punct
    [-3, 3, -4, -5, -3, 0, -3, -3],  // Inner
];

/// Glue between two adjacent atoms, in em.
pub fn space_between(left: SpacingClass, right: SpacingClass, style: MathStyle) -> f32 {
    let entry = SPACING_TABLE[left.index()][right.index()];
    let mu = if entry < 0 && style.is_script() {
        0
    } else {
        entry.unsigned_abs()
    };
    f32::from(mu) / 18.0
}

/// Applies TeX's rules that turn a binary operator into an ordinary atom
/// (e.g. the minus in `-x`, or a `+` right after `=`).
///
/// `starts_after_ord` pretends an ordinary atom precedes the list, as for the
/// second cell of an aligned row (`x &= 1` keeps the space before `=`).
pub fn resolve_binary_operators(classes: &mut [Option<SpacingClass>], starts_after_ord: bool) {
    use SpacingClass::*;
    let mut previous: Option<(Option<usize>, SpacingClass)> =
        starts_after_ord.then_some((None, Ord));
    for index in 0..classes.len() {
        let Some(class) = classes[index] else {
            continue;
        };
        let mut class = class;
        if class == Bin && previous.is_none_or(|(_, p)| matches!(p, Bin | Op | Rel | Open | Punct))
        {
            class = Ord;
        }
        if matches!(class, Rel | Close | Punct) {
            if let Some((Some(p), Bin)) = previous {
                classes[p] = Some(Ord);
            }
        }
        classes[index] = Some(class);
        previous = Some((Some(index), class));
    }
    if let Some((Some(last), Bin)) = previous {
        classes[last] = Some(Ord);
    }
}

#[cfg(test)]
mod tests {
    use super::SpacingClass::*;
    use super::*;

    #[test]
    fn relation_gets_thick_space_and_binary_medium() {
        assert_eq!(space_between(Ord, Rel, MathStyle::TEXT), 5.0 / 18.0);
        assert_eq!(space_between(Ord, Bin, MathStyle::TEXT), 4.0 / 18.0);
        assert_eq!(space_between(Ord, Op, MathStyle::TEXT), 3.0 / 18.0);
        assert_eq!(space_between(Ord, Ord, MathStyle::TEXT), 0.0);
    }

    #[test]
    fn script_styles_drop_conditional_spaces() {
        let script = MathStyle::TEXT.superscript();
        assert_eq!(space_between(Ord, Rel, script), 0.0);
        assert_eq!(space_between(Ord, Op, script), 3.0 / 18.0);
    }

    #[test]
    fn leading_binary_becomes_ordinary() {
        let mut classes = [Some(Bin), Some(Ord)];
        resolve_binary_operators(&mut classes, false);
        assert_eq!(classes, [Some(Ord), Some(Ord)]);
    }

    #[test]
    fn binary_after_relation_or_open_becomes_ordinary() {
        let mut classes = [
            Some(Ord),
            Some(Rel),
            Some(Bin),
            Some(Ord),
            Some(Open),
            Some(Bin),
            Some(Ord),
        ];
        resolve_binary_operators(&mut classes, false);
        assert_eq!(classes[2], Some(Ord));
        assert_eq!(classes[5], Some(Ord));
    }

    #[test]
    fn binary_before_relation_or_at_end_becomes_ordinary() {
        let mut classes = [Some(Ord), Some(Bin), Some(Rel), Some(Ord), Some(Bin)];
        resolve_binary_operators(&mut classes, false);
        assert_eq!(classes[1], Some(Ord));
        assert_eq!(classes[4], Some(Ord));
    }

    #[test]
    fn binary_between_ordinaries_stays_binary_and_spaces_are_transparent() {
        let mut classes = [Some(Ord), None, Some(Bin), None, Some(Ord)];
        resolve_binary_operators(&mut classes, false);
        assert_eq!(classes[2], Some(Bin));
    }

    #[test]
    fn aligned_cell_keeps_leading_relation_spacing() {
        let mut classes = [Some(Bin), Some(Ord)];
        resolve_binary_operators(&mut classes, true);
        assert_eq!(classes[0], Some(Bin));
    }

    #[test]
    fn styles_shrink_through_scripts_and_fractions() {
        assert_eq!(MathStyle::DISPLAY.superscript().scale(), SCRIPT_SCALE);
        assert_eq!(
            MathStyle::DISPLAY.superscript().superscript().scale(),
            SCRIPT_SCRIPT_SCALE
        );
        assert_eq!(MathStyle::DISPLAY.numerator(), MathStyle::TEXT);
        assert!(MathStyle::TEXT.denominator().cramped);
        assert_eq!(MathStyle::TEXT.numerator().scale(), SCRIPT_SCALE);
    }
}
