//! PDF export, typeset in-process with Typst (no external binaries).
//!
//! The note is converted to Typst markup (see `typst_markup`), compiled in a
//! minimal [`World`] that only knows that single source and the fonts
//! embedded in `typst-assets`, and written with `typst-pdf`.

use std::sync::LazyLock;

use qm_format::Document;
use typst::diag::{FileError, FileResult, Severity, SourceDiagnostic};
use typst::foundations::{Bytes, Datetime, Duration};
use typst::syntax::{FileId, RootedPath, Source, VirtualPath, VirtualRoot};
use typst::text::{Font, FontBook};
use typst::utils::LazyHash;
use typst::{Library, LibraryExt, World, WorldExt};
use typst_layout::PagedDocument;
use typst_pdf::PdfOptions;

use super::typst_markup;
use crate::{CoreError, Result};

/// How many compilations memoized results may stay unused before they are
/// evicted, so repeated exports do not grow memory without bound.
const CACHE_MAX_AGE: usize = 10;

/// Renders `document` as PDF bytes.
///
/// Fails with [`CoreError::Export`] carrying Typst's error messages if the
/// generated markup does not compile (which indicates a bug in the converter).
pub fn to_pdf(document: &Document) -> Result<Vec<u8>> {
    compile_typst(&to_typst(document)).map(|compiled| compiled.pdf)
}

/// The Typst source the PDF exporter compiles, for debugging or for users
/// who want to tweak the layout themselves.
pub fn to_typst(document: &Document) -> String {
    typst_markup::to_typst(document)
}

/// A successful compilation: the PDF and any warnings Typst reported.
pub(crate) struct CompiledPdf {
    pub(crate) pdf: Vec<u8>,
    #[cfg_attr(not(test), allow(dead_code))]
    pub(crate) warnings: Vec<String>,
}

/// Compiles a standalone Typst source to PDF.
pub(crate) fn compile_typst(source: &str) -> Result<CompiledPdf> {
    let world = NoteWorld::new(source);
    let compiled = typst::compile::<PagedDocument>(&world);
    let warnings = describe_all(&world, &compiled.warnings);
    let result = compiled
        .output
        .and_then(|document| typst_pdf::pdf(&document, &PdfOptions::default()));
    typst::comemo::evict(CACHE_MAX_AGE);
    match result {
        Ok(pdf) => Ok(CompiledPdf { pdf, warnings }),
        Err(errors) => Err(CoreError::Export(describe_all(&world, &errors).join("\n"))),
    }
}

fn describe_all(world: &NoteWorld, diagnostics: &[SourceDiagnostic]) -> Vec<String> {
    diagnostics
        .iter()
        .map(|diagnostic| describe(world, diagnostic))
        .collect()
}

/// Formats a diagnostic like `typst error (generated line 12: `…`): message`,
/// followed by any hints.
fn describe(world: &NoteWorld, diagnostic: &SourceDiagnostic) -> String {
    let severity = match diagnostic.severity {
        Severity::Error => "error",
        Severity::Warning => "warning",
    };
    let location = world
        .range(diagnostic.span)
        .map(|range| {
            let text = world.main.text();
            let start = range.start.min(text.len());
            let line_number = text[..start].matches('\n').count() + 1;
            let line = text.lines().nth(line_number - 1).unwrap_or_default().trim();
            format!(" (generated line {line_number}: `{line}`)")
        })
        .unwrap_or_default();
    let mut message = format!("typst {severity}{location}: {}", diagnostic.message);
    for hint in &diagnostic.hints {
        message.push_str(&format!("\n  hint: {}", hint.v));
    }
    message
}

/// Fonts and the standard library are expensive to build and never change,
/// so they are shared by all compilations.
struct SharedResources {
    library: LazyHash<Library>,
    book: LazyHash<FontBook>,
    fonts: Vec<Font>,
}

static SHARED: LazyLock<SharedResources> = LazyLock::new(|| {
    let fonts: Vec<Font> = typst_assets::fonts()
        .flat_map(|data| Font::iter(Bytes::new(data)))
        .collect();
    SharedResources {
        library: LazyHash::new(Library::default()),
        book: LazyHash::new(FontBook::from_fonts(&fonts)),
        fonts,
    }
});

/// A world with one source file, the embedded fonts, no file or package
/// access, and a fixed date so output is reproducible.
struct NoteWorld {
    main: Source,
}

impl NoteWorld {
    fn new(source: &str) -> NoteWorld {
        let path = VirtualPath::new("/main.typ").expect("a valid virtual path");
        let id = FileId::new(RootedPath::new(VirtualRoot::Project, path));
        NoteWorld {
            main: Source::new(id, source.to_string()),
        }
    }
}

impl World for NoteWorld {
    fn library(&self) -> &LazyHash<Library> {
        &SHARED.library
    }

    fn book(&self) -> &LazyHash<FontBook> {
        &SHARED.book
    }

    fn main(&self) -> FileId {
        self.main.id()
    }

    fn source(&self, id: FileId) -> FileResult<Source> {
        if id == self.main.id() {
            Ok(self.main.clone())
        } else {
            Err(FileError::AccessDenied)
        }
    }

    fn file(&self, id: FileId) -> FileResult<Bytes> {
        if id == self.main.id() {
            Ok(Bytes::from_string(self.main.clone()))
        } else {
            Err(FileError::AccessDenied)
        }
    }

    fn font(&self, index: usize) -> Option<Font> {
        SHARED.fonts.get(index).cloned()
    }

    fn today(&self, _offset: Option<Duration>) -> Option<Datetime> {
        Datetime::from_ymd(2000, 1, 1)
    }
}

#[cfg(test)]
mod tests {
    use qm_format::{Block, BlockKind, Inline};

    use super::*;
    use crate::export::fixtures::{display, every_math_node, kitchen_sink, paragraph};

    fn assert_compiles_cleanly(document: &Document) -> Vec<u8> {
        let source = to_typst(document);
        let compiled = compile_typst(&source)
            .unwrap_or_else(|error| panic!("{error}\n--- generated Typst ---\n{source}"));
        assert!(
            compiled.warnings.is_empty(),
            "unexpected warnings:\n{}\n--- generated Typst ---\n{source}",
            compiled.warnings.join("\n")
        );
        compiled.pdf
    }

    #[test]
    fn kitchen_sink_compiles_to_pdf_without_errors_or_warnings() {
        let pdf = assert_compiles_cleanly(&kitchen_sink());
        assert!(pdf.starts_with(b"%PDF"));
    }

    #[test]
    fn empty_document_compiles() {
        let pdf = to_pdf(&Document::default()).unwrap();
        assert!(pdf.starts_with(b"%PDF"));
    }

    #[test]
    fn every_math_node_compiles_on_its_own_inline_and_in_display() {
        for node in every_math_node() {
            let document = Document {
                blocks: vec![
                    paragraph(vec![Inline::Math(vec![node.clone()])]),
                    display(vec![vec![vec![node.clone()], vec![node.clone()]]]),
                    Block {
                        kind: BlockKind::Paragraph(vec![Inline::Math(vec![
                            qm_format::MathNode::Fraction {
                                numerator: vec![node.clone()],
                                denominator: vec![node],
                            },
                        ])]),
                        line: 0,
                    },
                ],
                ..Document::default()
            };
            assert_compiles_cleanly(&document);
        }
    }

    #[test]
    fn invalid_typst_is_reported_with_location() {
        let Err(CoreError::Export(message)) = compile_typst("#unknown-function()") else {
            panic!("expected an export error");
        };
        assert!(message.contains("unknown variable"), "{message}");
        assert!(message.contains("generated line 1"), "{message}");
    }

    #[test]
    fn output_is_deterministic() {
        let document = kitchen_sink();
        assert_eq!(to_pdf(&document).unwrap(), to_pdf(&document).unwrap());
    }
}
