# QuickMath

A desktop editor for taking math notes during lectures, written entirely in Rust.
Notes are plain-text `.qm` files in a notation built for typing speed: no `$`,
no backslashes, no braces, and formulas are typed left to right in the order
your pen would move.

```
.thm Geometric series
For ;;abs q < 1;;:
;; sum n=0 to oo q^n = 1 over 1 - q

;; (a+b)^2 = (a+b)(a+b)
;; = a^2 + 2a b + b^2
```

The full language reference, including the keystroke-cost rationale and a
cheat sheet, is in [docs/FORMAT.md](docs/FORMAT.md). A complete example note is
[docs/examples/lecture.qm](docs/examples/lecture.qm).

## Features

- **Live preview**: real math typesetting (fractions, scripts, roots, stretchy
  delimiters, matrices, aligned derivations) next to the editor, updated on
  every keystroke. Double-click the preview to jump to the source line.
- **Overleaf-style layout**: file tree and heading outline on the left,
  source editor in the middle, preview on the right, status bar at the bottom.
  Toggle between editor, split and preview views.
- **Projects like Obsidian vaults**: any folder is a project. Notes link to
  each other with `[[Note name]]`; deleted files go to `.quickmath/trash/`.
- **Exports** (current note or whole project):
  - Obsidian Markdown: `$…$` math, callouts, wikilinks; a project export is a
    ready-to-open vault.
  - LaTeX: standalone `.tex` with amsmath/amsthm environments.
  - PDF: built in (typst as a library), no TeX installation needed.
- **Practice game** (welcome screen or ☰ → Practice notation): a rendered
  formula appears and you type it; any spelling that renders the same counts.
  Twelve levels unlock one by one, from operators to lecture-speed formulas,
  with stars for speed (WPM) and a 60-second sprint over everything unlocked.
  Progress is kept in `practice.json` next to the settings.
- **Spell checking** in several languages at once (☰ → Settings…). English
  and Dutch are built in; any Hunspell `.aff`/`.dic` pair in
  `quickmath/dictionaries/` in the config directory (or installed system-wide
  on Linux) adds another. Only prose is checked, never math or code.
  Right-click an underlined word for corrections or to add it to your word
  list (`words.txt` next to the settings).
- Light and dark themes, autosave, syntax highlighting, notation help panel.

## Building

Requires Rust 1.94 or newer.

```sh
cargo run --release -p qm-app      # starts the `quick-math` editor
cargo test --workspace
```

## Code layout

| Crate | Responsibility |
|---|---|
| `crates/qm-format` | The `.qm` notation: parser, syntax tree (`src/ast.rs`), math vocabulary (`src/math/syntax.toml`), LaTeX conversion, highlighting |
| `crates/qm-core` | Projects, note storage, settings, spell checking, Obsidian/LaTeX/PDF exporters |
| `crates/qm-render` | Math typesetting and document preview on egui |
| `crates/qm-app` | The eframe desktop app (binary `quick-math`) |

`qm-render --example preview` shows the preview on its own;
`qm-render --example screenshot` renders it headlessly to a PNG.
