# QuickMath code design

This document describes how the QuickMath code is organised and why. It is
the intended design: when the code and this document disagree, one of them
should be changed to match the other.

The note language itself (what users type) is specified in
[FORMAT.md](FORMAT.md). This document covers the program around it.

## 0. How to use this document

- Sections are numbered so they can be referred to ("see 4.2").
- To change the design, edit the text directly: add, remove or reword rules,
  responsibilities or modules. Then ask for the code to be updated to match.
- To mark something as not built yet, put **(planned)** at the start of the
  bullet. To mark something for removal, put **(remove)**. Anything without a
  marker describes the code as it is.
- Keep each bullet to one fact or rule, so a change to it maps to a clear
  change in the code.
- Section 9 lists open questions. Put anything undecided there instead of
  writing it as a rule.

---

## 1. Goals

1. **Typing speed first.** The app exists to keep up with a lecturer. Any
   design choice that adds keystrokes or delay while typing loses.
2. **The preview never stops.** Half-typed input must always render. Parsing
   never fails, rendering never fails, and errors show up inline.
3. **Plain files.** Notes are `.qm` text files in an ordinary folder. The app
   adds only a hidden `.quickmath/` folder to a project.
4. **No external tools.** PDF export, fonts and math typesetting are built in.
   Nothing needs TeX, a browser or a network connection.
5. **One source of truth for the notation.** The math vocabulary is defined
   once (`syntax.toml`) and every consumer (parser, highlighter, help panel,
   exporters, preview) gets it from there.

---

## 2. Workspace and crates

The project is a Cargo workspace with four crates under `crates/`.

### 2.1 Crates

- **qm-format**: the `.qm` language. Parser, syntax tree, math vocabulary,
  LaTeX conversion of math, syntax highlighting, cheat sheet. Pure functions
  on strings. No I/O, no GUI.
- **qm-core**: the backend. Projects (folders of notes), file operations,
  settings, practice progress, spell checking and all exporters (Obsidian
  Markdown, LaTeX, PDF).
- **qm-render**: draws a parsed document or a single formula inside egui. Its
  own math typesetter and paragraph layout, plus the colour theme.
- **qm-app**: the desktop program (binary `quick-math`). Screens, panes,
  dialogs and the glue between the other three crates.

### 2.2 Dependency rules

- qm-format depends on nothing at runtime. It uses `toml` only in its build
  script and tests.
- qm-core depends on qm-format, plus typst (PDF), serde/serde_json (settings
  files), dirs (config directory) and spellbook (Hunspell-compatible spell
  checking).
- qm-render depends on qm-format and egui, plus typst-assets for fonts.
- qm-app depends on all three, plus eframe and rfd (native file dialogs).
- qm-core and qm-render do not depend on each other.
- Only qm-app may know about windows, screens and user interaction.
- Only qm-core may touch the file system. qm-app goes through qm-core for
  every read and write of notes, settings and exports. (File dialogs from rfd
  are the exception: they only pick paths.)
- Shared dependency versions are declared once in the root `Cargo.toml` under
  `[workspace.dependencies]`.

### 2.3 Where shared things live

- The syntax tree types (`ast.rs` in qm-format) are the contract between the
  parser, the renderer and the exporters.
- The colour `Theme` lives in qm-render and is also used by qm-app for the
  editor chrome, so preview and chrome always match.

### 2.4 Model, view and controller

The code follows model–view–controller, adapted to egui's immediate mode.

- **Model**: the data and the rules for changing it, with no drawing code.
  This is qm-format and qm-core, plus the app's own state types: `OpenNote`,
  `Spelling`, the practice `Session` and `curriculum.rs`, and the pure
  helpers (`text_metrics.rs`, `autosave.rs`).
- **View**: code that draws. This is qm-render, plus the `show` function of
  each pane and screen in qm-app. A view reads the model and returns what
  the user did as an action enum (see 7.2). It does not change the model.
- A view may keep its own widget state (scroll position, open or closed,
  text typed into a field), such as `FileTreeState` or `HelpPanel`. That
  state is never part of the model.
- **Controller**: `Workspace` and `QuickMathApp`. Each frame they call the
  views, handle the returned actions by calling the model, and pass up the
  actions they cannot handle themselves.
- Views redraw from the model every frame, so the model never notifies views
  of changes (no observers or callbacks).
- Exception: the editor widget edits `OpenNote::text` in place, because
  egui's text field needs a `&mut String`. It then calls
  `OpenNote::mark_edited` so the model stays consistent.

---

## 3. Main flows

### 3.1 Starting the app

1. `main.rs` creates the eframe window and `QuickMathApp`.
2. `QuickMathApp::new` installs the renderer's fonts, loads `AppSettings`,
   applies the light or dark theme, starts loading the spelling
   dictionaries on a background thread, and reopens the project that was
   open when the app last quit (if it still exists).

### 3.2 Typing in a note (every keystroke)

1. The editor widget edits `OpenNote::text` in place.
2. On change, `OpenNote::mark_edited` re-parses the whole note with
   `qm_format::parse_document`, updates the word count, marks the note dirty
   and restarts the autosave timer.
3. The editor's highlighting comes from `qm_format::highlight`, turned into
   an egui layout job and cached on the source text.
4. The preview calls `qm_render::show_document` with the new `Document`. The
   layout cache recomputes only blocks whose content changed.
5. When the cursor ends a partly typed math word, `qm_format::math_completions`
   suggests words from the syntax file and the editor lists them under the
   cursor (`completion_popup.rs`).
6. `qm_format::spell_check_words` picks out the prose words, the spell
   checker rejects some, and the editor draws a wavy line under those
   (`misspellings.rs`). The word at the cursor is skipped, so a word being
   typed is not flagged until it is finished.
7. After one second without edits the note is saved (`autosave.rs`).

### 3.3 Saving

- Saves go through `Project::write_note`, which writes atomically (temp file
  in the same folder, flush, rename). A crash leaves the old or the new file,
  never a truncated one.
- A failed save leaves the note dirty and shows a toast. Text is never
  silently dropped.
- Before closing a project or the window, the note and workspace state are
  saved. If saving fails on window close, the first close is cancelled with a
  warning; a second close quits anyway.

### 3.4 Exporting

1. The export dialog asks for format (Obsidian, LaTeX, PDF), scope (current
   note or whole project) and destination.
2. The export runs on a background thread so the UI stays responsive. The
   dialog polls the thread each frame and reports the result as a toast.
3. Single notes export the already-parsed `Document`. Project exports re-read
   and parse every note from disk.

### 3.5 Practice game

1. A level shows a target formula, rendered from a reference answer.
2. The player's input is parsed on every keystroke. It counts as correct when
   it produces the same LaTeX, or the same drawn symbols, as the answer, so
   any spelling that looks the same is accepted.
3. Results (stars, best WPM) are stored through `qm_core::PracticeProgress`.

---

## 4. qm-format: the note language

### 4.1 Public API

Everything other crates use is re-exported from `lib.rs`:

- `parse_document(source) -> Document`: parses a whole note.
- `parse_math(source) -> MathList`: parses what goes between `;;`.
- `math_to_latex`, `display_math_to_latex`: LaTeX math-mode source.
- `highlight(source) -> Vec<HighlightSpan>`: spans for editor colouring.
- `cheat_sheet()`: examples for the help panel.
- `vocabulary()`: every keyword and operator from the syntax file, by section.
- `math_completions(source, cursor) -> Option<MathCompletions>`: syntax-file
  words for the math word ending at the cursor, best first.
- `spell_check_words(source) -> Vec<Range<usize>>`: the prose words spell
  checking looks at.
- All syntax tree types from `ast.rs`.

### 4.2 Rules

- **Parsing never fails and never panics.** Input that cannot be understood
  becomes `MathNode::Error` with the original source and a message. There is
  no `Result` in the parsing API.
- Parsing must stay fast enough to run on every keystroke for a full lecture
  note. `tests/robustness.rs` checks for panics, hangs and stack overflows on
  adversarial input.
- The parser and the highlighter share their line classification
  (`line_syntax.rs`) and math lexer, so highlighting always agrees with
  parsing.
- Highlight spans cover every byte of the source, without gaps or overlaps.
- Completion uses the highlighter to decide whether the cursor is in math, so
  words are suggested exactly where the parser reads them as math.
- Spell checking also uses the highlighter: only text, heading, emphasis and
  strong spans are prose. Single letters, words with digits or `_`,
  abbreviations in capitals and web addresses are skipped, because in math
  notes they are nearly always variables or names, not misspellings.
- Completion ranks prefix matches first, then letters-in-order matches (from
  three typed letters), then the shortest spelling. Aliases of one symbol
  (`eps`, `epsilon`) are listed once, under the best-ranked spelling.
- Every block records its zero-based source line, used to sync the editor
  cursor and the preview.

### 4.3 Syntax tree (`ast.rs`)

- `Document` = front matter + list of `Block`s.
- `BlockKind`: heading, paragraph, display math, list, callout (definition,
  theorem, lemma, corollary, proof, example, note, warning), code, quote,
  rule. Containers (lists, callouts, quotes) hold nested blocks.
- `Inline`: text, strong, emphasis, code, inline math, wikilink, line break.
- `MathNode`: atom, text, fraction, scripts, root, fenced, matrix, accent,
  invisible group, space, error.
- An `Atom` carries its Unicode text, its LaTeX, a spacing class (TeX atom
  types) and a style (italic, upright, bold). Consumers never need to look
  symbols up again.
- Changes to these types must be backwards-compatible where possible, and
  every consumer (renderer, all exporters, practice game) must be updated in
  the same change.

### 4.4 Document parsing

Done in stages, each in its own module:

1. `lines.rs`: split into numbered lines; indentation helpers.
2. `front_matter.rs`: the optional `---` header.
3. `line_syntax.rs`: what kind of block each line starts.
4. `block.rs`: builds blocks. Containers dedent their lines and parse them
   recursively, which keeps the original line numbers.
5. `inline.rs`: inline markup within one line. Inline markup never spans
   lines.

### 4.5 Math parsing (`math/`)

- `lexer.rs`: splits math into tokens. Whitespace is not a token; it is
  recorded on the next token, because spacing decides grouping.
- `parser.rs`: builds the tree following the pen-order rules in FORMAT.md
  (runs without spaces group together, `/` binds weakest inside a run, `^`
  takes the rest of its run, prefix words take a bracket group or the rest of
  the run, and so on).
- `mod.rs`: display-math rows and their alignment (first relation, or
  explicit `&`).
- `symbols.rs`: keyword and operator lookup, built from the generated tables.
- `latex.rs`: LaTeX output (amsmath + amssymb).

### 4.6 The syntax file

- `src/math/syntax.toml` defines every math word and operator spelling, with
  its Unicode and LaTeX form, grouped into sections (letters, relations,
  large operators, fences, accents, fonts, …). Its header documents the
  format.
- `build.rs` reads it at compile time, using `build/syntax_tables.rs`, and
  generates Rust tables that `symbols.rs` includes. The program itself
  contains no TOML parser.
- Mistakes in the file (unknown section, duplicate spelling, operator in a
  word-only section, …) fail the build with a message naming the entry.
- `tests/syntax_file.rs` tests the generator directly.
- Adding a symbol means adding one line to the file. Nothing else needs to
  change.
- A few rules stay in code because they are grammar, not vocabulary: `inf_`
  as infimum, `^*` as conjugate, `'` as prime.

---

## 5. qm-core: projects, storage and export

### 5.1 Public API

- `Project`: an opened project folder.
- `FileNode`, `FileKind`: the scanned file tree.
- `WorkspaceState`: per-project UI state.
- `AppSettings`: app-wide settings.
- `PracticeProgress`, `LevelRecord`: practice game results.
- `Language`, `Dictionary`, `SpellChecker`, `WordList`: spell checking (see
  5.5).
- `ExportFormat`, `export_note`, `export_project`, and the string renderers in
  `export::{latex, obsidian, pdf}`.
- `CoreError` and `Result`.

### 5.2 Projects

- Any folder can be a project. Opening it creates `.quickmath/` and
  `.quickmath/trash/` if missing.
- The project root is stored canonicalised, so containment checks are
  reliable.
- Every method that takes a path accepts it absolute or relative to the root,
  and refuses paths outside the root or inside `.quickmath/`
  (`CoreError::OutsideProject`).
- Names for new notes, folders and renames are validated (`paths.rs`).
- Deleting moves the entry to `.quickmath/trash/`, adding a number if a file
  of that name is already there. Nothing is ever deleted for real.
- Wikilinks (`[[Note name]]`) are resolved Obsidian-style against the file
  tree (`wikilink.rs`).

### 5.3 Stored files

- Notes: `.qm` files anywhere in the project.
- Workspace state: `.quickmath/workspace.json` (open note, expanded folders).
  Paths are stored relative to the root with `/` separators, so a project can
  be moved or synced between machines.
- Settings: `quickmath/settings.json` in the OS config directory (recent
  projects, dark mode, project open at quit, spelling languages).
- Practice progress: `quickmath/practice.json` next to the settings.
- Personal word list: `quickmath/words.txt` next to the settings, one word
  per line.
- Extra spelling dictionaries: `quickmath/dictionaries/<code>.aff` and
  `<code>.dic` next to the settings.
- All writes are atomic (`fs_util::write_atomically`).
- Missing or unreadable settings and progress files fall back to defaults
  instead of failing.

### 5.4 Exporters

- Every exporter takes a parsed `Document`. Exporters never parse source
  text themselves (except project export, which reads each note first).
- Because the parser never fails, exporters must accept any tree it can
  produce. `tests/parsed_notes.rs` exports real parser output to check this.
- **Obsidian Markdown** (`obsidian.rs`): `$…$` math, callouts mapped to
  Obsidian's built-in callout types, wikilinks kept, front matter as YAML
  properties.
- **LaTeX** (`latex.rs`): a standalone `article`; callouts become amsthm
  environments.
- **PDF** (`pdf.rs`): the document is converted to Typst markup
  (`typst_markup.rs`, `typst_math.rs`) and compiled in-process with the typst
  library and the fonts embedded in typst-assets. The Typst output escapes
  everything and uses explicit function calls, so no user text can be read as
  markup.
- Project export mirrors the folder structure. For Obsidian it also copies
  non-note files, giving a ready-to-open vault. Exporting into the project
  folder itself or one of its parents is refused.
- `export/fixtures.rs` holds hand-built documents covering every syntax tree
  variant, for exporter tests.

### 5.5 Spell checking

- Dictionaries are Hunspell `.aff`/`.dic` pairs, checked with the spellbook
  crate (pure Rust, so no C library is needed).
- English (`en_US`) and Dutch (`nl_NL`) are compiled in from
  `qm-core/dictionaries/`, which also records their source and licences.
- `Language::available` adds every pair found in the user's dictionary
  folder and the Linux system folders (`/usr/share/hunspell`,
  `/usr/share/myspell`). Files win over a built-in dictionary with the same
  code.
- Any number of languages can be on at once. A word is correct when any of
  them accepts it, so notes that mix languages need no switching.
  Suggestions take turns between the languages.
- Settings store the codes of the languages that are on. An empty list turns
  spell checking off; new installs start with `en_US`.
- Words in the personal word list are added to every loaded dictionary, so
  they follow Hunspell's casing rules (`eigenbasis` also accepts
  `Eigenbasis`).
- Dictionary files must be UTF-8. A dictionary that fails to load is
  reported and left out; the other languages keep working.

### 5.6 Errors

- One error type, `CoreError`: I/O (always with the path), invalid name,
  already exists, outside project, export failure, dictionary failure.
- Messages are meant to be shown to the user as they are.

---

## 6. qm-render: the live preview

### 6.1 Public API

- `install_fonts`: registers the renderer's fonts (New Computer Modern from
  typst-assets) under their own family names. The app's own fonts are left
  alone.
- `show_document(ui, document, theme, options) -> PreviewResponse`: the
  scrollable page preview. Options can scroll to a source line; the response
  reports double-clicked lines and clicked wikilinks.
- `show_math(ui, math, is_display, theme)`: one formula, for the help panel
  and practice game.
- `Theme`, `SyntaxColors`: the light and dark palettes.

### 6.2 Layout pipeline

1. **Math layout** (`math/`): a TeX-inspired box model. Each `MathNode`
   becomes a `MathBox` (width, ascent, descent, drawing). Spacing follows TeX
   atom classes and the font's math constants (`style.rs`). Stretchy
   delimiters taller than the font's glyphs are drawn as paths.
2. **Paragraph flow** (`document/flow.rs`): words and inline math boxes are
   placed on shared baselines with QuickMath's own line breaking, because
   egui's text layout cannot embed custom boxes.
3. **Block layout** (`document/blocks.rs`): headings, lists, callouts (with
   numbering), code, quotes, display math and the title block.
4. **Painting** (`paint.rs`): layouts produce drawing lists that are painted
   with egui's painter each frame.

### 6.3 Rules

- Layout talks to egui's font system only through the `TextMeasure` trait
  (`measure.rs`), so layout can be tested headless.
- Drawing lists store colour *roles*, not colours, so cached layouts survive
  a theme switch.
- Drawing lists store text as strings, not galleys, so they never go stale
  when egui rebuilds its font atlas.
- Layouts are cached per block (`document/cache.rs`). The key hashes the
  block's content, layout width, font size, pixel density and callout
  counters. Nested lines are hashed relative to their top-level block, so
  inserting a line above a block does not invalidate it. Unused entries are
  dropped each frame.
- Only blocks near the visible area are painted.
- Math nesting is capped (`MAX_DEPTH`) so deep input cannot overflow the
  stack.

### 6.4 Examples

- `examples/preview.rs`: the preview in its own window.
- `examples/screenshot.rs`: renders the preview to a PNG with a small
  software rasteriser (no GPU needed), and has a benchmark mode.

---

## 7. qm-app: the desktop editor

### 7.1 Structure

- `app.rs` (`QuickMathApp`): top-level state. Owns settings, theme, the
  spell checker, toasts and the current screen: the welcome screen or a workspace. The practice
  game is shown on top of either while open.
- `welcome.rs`: shown when no project is open. Open, create or reopen a
  project, or start practice.
- `workspace.rs` (`Workspace`): an open project. Owns the project, file tree,
  open note, editor, preview sync, pane layout, dialogs and help panel, and
  ties them together.
- `note.rs` (`OpenNote`): the open note's text, parsed document, dirty flag,
  autosave timer and cursor position.
- `spelling.rs` (`Spelling`): the available languages, the personal word
  list and the loaded spell checker. Dictionaries load on a background
  thread, polled each frame.
- One module per pane or widget: `toolbar.rs`, `file_tree.rs`, `outline.rs`,
  `editor.rs`, `completion_popup.rs`, `misspellings.rs`, `preview_pane.rs`, `status_bar.rs`,
  `help_panel.rs`, `export_dialog.rs`, `settings_window.rs`, `prompts.rs` (small modals),
  `toasts.rs` (notifications).
- Pure helpers with no egui state: `text_metrics.rs` (cursor positions, word
  count, text edits), `autosave.rs` (debounce timer), `highlight.rs` (span to
  layout job conversion), `theme_style.rs` (maps `Theme` onto egui visuals).
- `practice/`: the practice game. `curriculum.rs` (levels and their
  answers), `session.rs` (one run: queue, clock, grading), `play.rs` and
  `menu.rs` (the screens).

### 7.2 Interaction pattern

- egui is immediate mode: everything is drawn every frame.
- Widgets do not change app state directly. Each `show_*` function returns an
  action enum (`ToolbarAction`, `FileTreeAction`, `WelcomeAction`,
  `PromptOutcome`, `PracticeAction`, …) and its owner handles it.
- Actions that only the app can handle (switching projects, theme, quit,
  spelling languages, adding a word to the word list) are passed up from the
  workspace to `QuickMathApp` as a `WorkspaceAction`.
- Requests to the editor that need its widget state (jump to line, insert
  text) are queued in `OpenNote::requests` and applied on its next frame.
- The preview only scrolls when the editor cursor moves to another line, or
  on an explicit request (e.g. from the outline), so the user can scroll it
  freely otherwise.
- Long-running work (exports, loading dictionaries) runs on a background
  thread and is polled each frame.

### 7.3 Layout

- Toolbar at the top, status bar at the bottom.
- Left sidebar: file tree above the heading outline.
- Centre: editor, preview, or both side by side (`ViewMode`).
- Right: help drawer with the cheat sheet, every symbol from the syntax file
  (click to insert) and keyboard shortcuts.
- Misspelled words get a wavy underline in the error colour. Right-clicking
  one opens a menu with corrections and "Add to dictionary". Suggestions are
  only computed then, because they take tens of milliseconds per language.
- The settings window (☰ → Settings…) holds every setting: dark theme,
  view mode, sidebar and cheat sheet visibility, and spelling languages.
  Changes apply at once.
- The toolbar keeps quick toggles for the theme, view mode, sidebar and
  cheat sheet. They give the same actions as the settings window, so both
  stay in step.
- Math suggestions float under the word being typed. Only Tab accepts one;
  Up/Down choose and Escape hides them. Enter and Space keep their normal
  meaning, so typing on never picks a suggestion by accident.

### 7.4 Errors in the UI

- User-facing failures (save, open, export, settings) are shown as toasts.
  They are never only logged and never silently ignored.

### 7.5 Practice game rules

- Session times use egui's clock passed in by the caller, so a session is
  deterministic and testable.
- Sprints pick exercises with a seeded xorshift generator (no dependency) and
  never repeat the same formula twice in a row.
- Level ids in `curriculum.rs` are keys in saved progress and must never
  change once released.
- Stars come from typing speed: two stars from 20 WPM, three from 35 WPM (one
  word = five characters).

---

## 8. Testing

- Integration tests live in each crate's `tests/` folder and use only the
  public API. Unit tests live next to the code in `#[cfg(test)]` modules.
- qm-format: document structure (`document.rs`), math behaviour checked
  mostly through LaTeX output (`math.rs`), highlighting (`highlight.rs`),
  completion (`completion.rs`), which words are spell checked
  (`spelling.rs`), robustness on bad input (`robustness.rs`), the syntax file generator
  (`syntax_file.rs`).
- qm-core: project operations and exports against real temporary folders
  (`project.rs`, `export.rs`), exporting real parser output
  (`parsed_notes.rs`), and finding languages and checking words with small
  test dictionaries and the built-in ones (`spelling.rs`).
- qm-render: layout metrics and cache behaviour with real fonts in a
  headless egui context (`math/tests.rs`, `document/tests.rs`).
- qm-app: unit tests inside most modules, covering the pure helpers, the
  practice session logic and how each pane turns input into actions. The
  editor tests drive the real widget in a headless egui context.
- Run everything with `cargo test --workspace`.

---

## 9. Open questions and planned work

Feature ideas are tracked in [TODO.md](TODO.md). Design questions that are
not decided yet go here.

- The export dialog (`export_dialog.rs`) is a view, but it starts the export
  thread and calls `qm_core::export_note` / `export_project` itself instead
  of returning an action. Should it return an action for `Workspace` to
  handle (see 2.4)?
- `welcome::create_project_folder` creates the folder with `std::fs`
  directly, which breaks the rule that only qm-core touches the file system
  (see 2.2). Should this move into qm-core?
