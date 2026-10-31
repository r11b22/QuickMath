//! Exports of real parser output. Whatever the parser produces for arbitrary
//! input must export without errors (the parser never fails, so neither may
//! the exporters).

use qm_core::ExportFormat;
use qm_format::parse_document;

const SAMPLE_NOTES: [&str; 4] = [
    "",
    "---\ntitle: Groups\nauthor: Ada\ntags: [algebra]\n---\n\n# Groups\n\nA *group* is a set $G$ with **one** operation.\n\n$$ a * (b * c) = (a * b) * c $$\n\n- closure\n- identity $e$\n  1. nested\n\n> [!definition] Group\n> Body with [[week1/intro|intro]].\n\n```rust\nfn main() {}\n```\n\n---\n",
    "$$\nf(x) &= sum_(i=1)^n x_i^2 \\\\\n&= frac(1, 2) sqrt(x) + {1 if x > 0; 0 otherwise}\n$$\n\n$unclosed (\n",
    "Specials: \\ * _ ` # [ ] $ < > @ = - + / ~ : % & ^ { } | \" ' 1. // $$ $ \u{0} \t",
];

#[test]
fn every_format_exports_parsed_notes_without_errors() {
    for source in SAMPLE_NOTES {
        let document = parse_document(source);
        for format in [
            ExportFormat::Obsidian,
            ExportFormat::Latex,
            ExportFormat::Pdf,
        ] {
            let bytes = format
                .render(&document)
                .unwrap_or_else(|error| panic!("{format:?} failed for {source:?}: {error}"));
            assert!(!bytes.is_empty());
        }
        let pdf = ExportFormat::Pdf.render(&document).unwrap();
        assert!(pdf.starts_with(b"%PDF"));
    }
}
