# The QuickMath note format (`.qm`)

QuickMath notes are plain text written for one situation: keeping up with a
lecturer at the blackboard. The document layer is Markdown-like. The math layer
is a notation of its own: it uses no `$`, almost no `\`, and no `{}`. Most of
it is letters and unshifted keys, and you can type it in the order your pen
would move.

- [File format](#file-format)
- [Design rationale: keystroke cost and pen order](#design-rationale-keystroke-cost-and-pen-order)
- [Document syntax](#document-syntax)
- [Math syntax](#math-syntax)
- [Display math and aligned derivations](#display-math-and-aligned-derivations)
- [Symbol reference](#symbol-reference)
- [Cheat sheet](#cheat-sheet)

---

## File format

- **Encoding:** UTF-8 plain text. You can type Unicode such as `α`, `≤` or `ℝ`
  directly, including inside math.
- **Extension:** `.qm`.
- **Line endings:** `\n` or `\r\n`.
- **Front matter (optional):** a block at the very top, between two `---`
  lines:

  ```
  ---
  title: Lecture 7: Groups
  author: Ada Lovelace
  date: 2026-10-31
  course: Algebra 101
  tags: [algebra, groups]
  ---
  ```

  Recognised keys (case-insensitive): `title`, `author`, `date`, `course`, and
  `tags` (or `tag`). Values may be quoted. Tags can be written as `[a, b]`,
  `a, b`, `#a #b`, or a YAML list (`tags:` followed by `  - a` lines). Unknown
  keys are ignored. The header only counts as front matter if it is closed and
  every line inside looks like `key: value`, a `- item`, or is blank. Anything
  else means the first `---` is just a horizontal rule.

Parsing never fails. While you type, every intermediate state still produces a
sensible document. Half-finished math shows what it can, and only the part that
truly cannot be read becomes an error node, shown verbatim.

---

## Design rationale: keystroke cost and pen order

Two rules decided every piece of syntax.

**1. Keystroke cost.** A shifted or hard-to-reach character (`$ \ { } ^ _ ( ) & | "`)
costs more than one to three plain letters or unshifted keys
(`- = [ ] ; ' , . / \``). So every common construct has a letter or unshifted
spelling:

| Instead of | QuickMath uses | Why |
|---|---|---|
| `$…$` | `;;…;;` | `;` is unshifted and on the home row. `;;` never appears in prose. |
| `\begin{definition}` | `.def` | A dot plus a short word. |
| `{…}` for grouping | spacing, or `[…]` | Spaces decide what an operator applies to. `[ ]` is unshifted. |
| `\frac{a}{b}` | `a/b`, `a over b` | `/` is unshifted. |
| `\sqrt{…}` | `sqrt x+1` | No braces needed. |
| `\sum_{i=1}^{n}` | `sum i=1 to n` | Read aloud, typed as said. |
| `&` and `\\` | one `;;` line per row | Rows align automatically at the first `=`. |
| `\leq`, `\neq` | `<=` or `le`, `/=` or `ne` | Short and mostly unshifted. |
| `\text{if}` | `if` | Common prose words are reserved inside math. |

**2. Pen order.** You never have to go back and wrap something in brackets.
You type the formula left to right, the way you would write it:

- When you start a superscript, it continues until you type a space:
  `x^n+1` is x to the n+1.
- `a+b/c+d` is (a+b)/(c+d). You just type `a+b`, a slash, then `c+d`. With
  spaces, `a + b/c + d` divides only `b` by `c`.
- `sqrt x+1` puts x+1 under the root, and `sqrt x + 1` does not.
- `sum i=1 to n` reads like the spoken limits.

Keystroke counts (a shifted character counts as two presses):

| Formula | LaTeX | Presses | QuickMath | Presses |
|---|---|---:|---|---:|
| (a+b)/(c+d) | `$\frac{a+b}{c+d}$` | 25 | `;;a+b/c+d;;` | 13 |
| xⁿ⁺¹ | `$x^{n+1}$` | 15 | `;;x^n+1;;` | 11 |
| √(x+1) | `$\sqrt{x+1}$` | 17 | `;;sqrt x+1;;` | 13 |
| ∑ i² for i=1…n | `$\sum_{i=1}^{n} i^2$` | 29 | `;;sum i=1 to n i^2;;` | 21 |
| lim sin x / x | `$\lim_{x \to 0} \frac{\sin x}{x}$` | 42 | `;;lim x to 0 (sin x)/x;;` | 26 |
| 2×2 matrix | `$\begin{pmatrix} 1 & 2 \\ 3 & 4 \end{pmatrix}$` | 54 | `;;mat 1 2, 3 4;;` | 16 |
| \|x\| as cases | `$\|x\| = \begin{cases} x & \text{if } x \ge 0 \\ -x & \text{otherwise} \end{cases}$` | 95 | `;;abs x = cases x if x >= 0, -x otherwise;;` | 44 |
| two-row derivation | `\begin{aligned} (a+b)^2 &= a^2 + 2ab + b^2 \\ &= c \end{aligned}` | 78 | `;; (a+b)^2 = a^2 + 2a b + b^2` ⏎ `;; = c` | 44 |
| definition box | `\begin{definition}[Group] … \end{definition}` | 51 | `.def Group` ⏎ … | 15 |

Shifted forms such as `^`, `_` and `( )` remain because they are universal and
short, but the common cases have cheaper spellings: `x1` for x₁, `x sq` for x²,
and `A inv` for A⁻¹.

---

## Document syntax

### Lines and paragraphs

- Consecutive non-blank lines form a paragraph. **Each line break is kept**,
  so lines you type as separate lines show as separate lines.
- A blank line ends the paragraph.
- Inline markup (emphasis, code, math, links) never spans a line break.
- Leading indentation does not matter for headings, paragraphs and so on. It
  only matters for lists and callout bodies (see below).

### Headings

`#` to `######` followed by a space: `## Series`. `#tag` (no space) is plain
text.

### Inline markup

| Write | Meaning |
|---|---|
| `*text*` | emphasis |
| `**text**` | strong |
| `` `code` `` | inline code. Use more backticks to include a backtick: ``` ``a`b`` ``` |
| `;;x^2;;` | inline math (see [Math syntax](#math-syntax)) |
| `[[Note]]` | link to another note in the project |
| `[[Note\|shown text]]` | link with a custom label |

A `*` only opens emphasis when the next character is not a space, and only
closes it when the previous character is not a space. So `a * b` stays
literal. Unmatched delimiters are shown as typed.

While you type, an unclosed `;;` math span extends to the end of the line, so
the preview already shows the formula.

### Escapes

A backslash before ASCII punctuation produces that character literally:
`\;;` → `;;`, `\*` → `*`, `` \` `` → `` ` ``, `\[` → `[`. `$` has no special
meaning in QuickMath, so `costs $5` needs no escape.

### Lists

- `- item` or `* item` for bullets. `1. item` for numbered lists.
- Nest by indenting at least 2 more columns than the parent marker (a tab
  counts as 4).
- Lines indented under an item belong to it, including display math, nested
  lists and further paragraphs. Separate an item's paragraphs with a blank
  line and keep them indented.
- A line that is not indented ends the list. Switching between bullets and
  numbers starts a new list.

### Callouts (definitions, theorems, proofs, …)

A callout starts with a **dot and a keyword** at the beginning of a line,
optionally followed by a title. The title may contain inline math or be empty.
An optional `:` after the keyword is allowed.

```
.def Group
A group is a set ;;G;; with an operation such that …
;; a * (b * c) = (a * b) * c

.proof
Direct computation.

  The second paragraph of the proof is indented.
```

| Keyword(s) | Box |
|---|---|
| `.def`, `.defn`, `.definition` | Definition |
| `.thm`, `.theorem` | Theorem |
| `.lem`, `.lemma` | Lemma |
| `.cor`, `.corollary` | Corollary |
| `.proof`, `.pf` | Proof |
| `.ex`, `.eg`, `.example` | Example |
| `.note`, `.rem`, `.remark` | Note |
| `.warn`, `.warning`, `.caution` | Warning |

Keywords are case-insensitive. The keyword must be followed by a space, `:`, or
the end of the line, so `.defx` and `.5` are ordinary text.

**Body rule (precise):**

1. Every line directly below the header belongs to the body, with no
   indentation needed. This stops at the first blank line, or at a heading,
   horizontal rule or another callout header that is not indented further
   than this header.
2. After a blank line, the body continues only with lines indented at least 2
   columns more than the header (and any blank lines between such lines).
3. The first line that matches neither rule ends the callout.

So a one-paragraph definition needs no indentation at all, and a
multi-paragraph proof indents its later paragraphs.

### Other blocks

| Write | Block |
|---|---|
| `> text` | quote (may contain any blocks) |
| ```` ``` ```` + optional language, closed by ```` ``` ```` | code block (never parsed). Unclosed: runs to the end of the note. |
| `---` (or `***`, `___`) alone on a line | horizontal rule |
| a line starting with `;;` | display math (see below) |

---

## Math syntax

Math appears between `;;` delimiters inline, and after `;;` at the start of a
line for display math.

### Tokens

- **Numbers:** `42`, `3.14`. Numbers are upright.
- **Words:** runs of ASCII letters. A known word is a keyword (`alpha`, `sum`,
  `sqrt`, `in`, …). A keyword may end in digits (`integral2`); after any
  other word, digits stay a number, so `x1` is still x₁. An unknown word becomes separate italic letters, the way
  TeX treats it: `xy` is x·y and `dx` is d x. Separate keywords from
  variables with a space: `sin x`, `2 pi r`. `sinx` is s·i·n·x.
- **Spaces** never render, but they do decide grouping (next section).
- **Text:** `` `any text` `` or `"any text"` shows upright prose. Math
  spacing does not apply to prose, so quoted text keeps **one space on each
  side where the source has whitespace next to it**: in `a_n "converges"`
  the text is " converges" (LaTeX `\text{ converges}`), while `x"="y` has no
  added space. Prose words (`if`, `otherwise`, …) are always padded on both
  sides.
- **LaTeX habits:** `\alpha`, `\leq` and so on mean the same as the bare word.
  An unknown `\command` is an error. `\,` `\:` `\;` are thin, medium and thick
  spaces, and `\{` `\}` `\$` `\\` are literal characters.

### Grouping: runs

A **run** is a stretch of math without spaces. Three kinds of token always end
a run, even without a space:

- relations: `=`, `<`, `->`, `in`, `to`, …
- punctuation: `,`, `;`, `:`
- prose words: `if`, `else`, `otherwise`, `for`, `when`, `where`

Inside a run:

1. **`/` binds weakest.** The run is split at its slashes into fraction
   parts: `a+b/c+d` = (a+b)/(c+d), `x^2/2` = x²/2, `dy/dx`. Repeated slashes
   nest to the left. Spaces around a slash are allowed (`x^2 / 2`) and divide
   the neighbouring runs.
2. **`^` takes the rest of the run**: `x^n+1` = xⁿ⁺¹, `e^-x` = e⁻ˣ.
   `x^2 + 1` (with spaces) is x² + 1. Watch out: `x^2+1` is x²⁺¹.
   `**` is another spelling of `^`: `x**2` = x².
3. **`_` takes the rest of the run up to a `^`**: `x_i^2`, `a_n+1` = aₙ₊₁.
4. **Digits directly after a variable are a subscript**: `x1` = x₁,
   `a12` = a₁₂, `alpha2` = α₂. This applies only to italic variables and
   Greek letters, so `2x` and `RR2` are unaffected.
5. **Primes:** `f'`, `f''` (shown as superscript primes).

**Brackets** `( )`, `[ ]` and `{ }` show as stretchy delimiters. When a
bracket group makes up a whole operand, its brackets disappear. Here an
operand means a fraction part, a script, the argument of a prefix word, a
matrix cell, or a limit:

- `(a+b)/(c+d)` = (a+b)/(c+d) shown without parentheses
- `x^[n+1]`, `e^(i pi)`, `sqrt[x + 1]` group across spaces
- `[0, 1)`, `[a, b]`, `{x in RR : x > 0}` are shown as typed

`[ ]` is the cheap unshifted way to group. Any closing bracket closes the
innermost open one, so half-open intervals like `[0, 1)` work. Unclosed
brackets are shown open, and stray closers are shown as typed.

### Fractions with `over`

`over` divides **everything since the previous relation, comma, or start of the
enclosing bracket**, by everything up to the next one:

```
y = 1 + x over 1 - x        →  y = (1+x)/(1-x)
(a + b over 2), c           →  ((a+b)/2), c
```

### Prefix words

These take one argument: a directly attached bracket group (`sqrt(x)+1` = √x
+ 1), or otherwise **the rest of the run**. A space right after the word does
not end that run:

| Word | Result |
|---|---|
| `sqrt x+1` | √(x+1) |
| `root 3 x`, `root(3)(x)`, `root(3, x)` | ∛x |
| `abs x`, `norm v`, `floor x`, `ceil x`, `ang v` | \|x\|, ‖v‖, ⌊x⌋, ⌈x⌉, ⟨v⟩ |
| `hat x`, `bar x`, `vec v`, `dot x`, `ddot x`, `tilde x` | accents |
| `bb v`, `cal A`, `bbb R` | bold **v**, script 𝒜, blackboard ℝ |

`1/sqrt 2` is 1/√2. `sqrt x + 1` is √x + 1.

### Postfix words

`x sq` = x², `x cb` = x³, `A inv` = A⁻¹. Each applies to the term just before
it.

### Large operators and spoken limits

`sum`, `prod`, `bigcup`, `bigcap`, `int`, `iint`/`integral2`, `iiint`, `oint`, `lim`,
`limsup`, `liminf`, `max`, `min`, `sup`, `infimum`, `argmax`, `argmin`.

Limits can be written with `_` and `^` (`sum_[i=1]^n`, `int_0^1`,
`lim_[x->0]`), or spoken, directly after the operator and separated by
spaces:

| Write | Means |
|---|---|
| `sum i=1 to n` | ∑ from i=1 up to n |
| `int 0 to 1 f(x) dx` | ∫ from 0 to 1 |
| `sum x in S f(x)` | ∑ over x ∈ S |
| `lim x to 0`, `lim x->0` | lim as x → 0 |
| `max x in S f(x)` | max over x ∈ S |

How the parser decides, given the space-separated chunk after the operator:

- **Sum-like operators** (symbols: ∑ ∏ ⋃ ⋂ ∫ …): the chunk is a lower limit if
  `to` follows it (`to` then introduces the upper limit) or if it contains or
  is joined by `in`. Otherwise the chunk is the body, so `sum a_n = 1` is
  simply ∑ aₙ = 1.
- **Limit-like operators** (word operators: lim, max, sup, …): `to` is the
  arrow. The chunk is a limit if it contains a relation (`x->0`) or is joined
  to the next chunk by `to`, `->` or `in`.

**`inf` vs infimum:** `inf` (like `oo`) is ∞. It means infimum only when
directly followed by `_` (`inf_n a_n`). Otherwise write `infimum`, or `\inf`.
`sup` is always the supremum. For superset use `supset` or `supe`.

### Matrices and cases

| Write | Result |
|---|---|
| `mat 1 2, 3 4` | (parenthesised) 2×2 matrix |
| `bmat a b, c d` | [square-bracketed] matrix |
| `vmat a b, c d` | \|determinant\| |
| `cases x if x >= 0, -x otherwise` | piecewise with a left brace |
| `((1,2),(3,4))` | parenthesised matrix, nested-list style |

Spaces separate cells and `,` (or `;`) separates rows. Group a cell that
contains spaces with brackets: `mat [a + b] c`. Without an attached bracket
group, `mat`/`cases` takes **the rest of the formula**. Use `mat[1 0, 0 1]`
when something follows the matrix on the same line.

A `cases` row is split into value and condition before its first prose word
(`if`, `otherwise`, `for`, `when`, `where`, `else`). Without a prose word, it
splits after the first chunk: `cases 1 x>0, 0 x<=0`.

### Reserved words inside math

Every word in the [symbol reference](#symbol-reference) is reserved, along with
`over`, `to`, `sq`, `cb`, `inv`, `mat`, `pmat`, `bmat`, `vmat`, `cases`,
`quad`, `qquad` and the prose words `if`, `else`, `otherwise`, `for`, `when`,
`where`. To use such letters as variables, separate them (`i n`) or quote
text (`` `in` ``).

### While typing

| Input | Shown as |
|---|---|
| `x^` | x with an empty superscript box |
| `a/` | a over an empty denominator |
| `(a+b` | an open, unclosed parenthesis |
| `sum i=1 to` | ∑ with an empty upper limit |
| `"unfinished` | text up to the end |
| `\foo` | the error node `\foo` (unknown command) |

Extremely deep nesting (dozens of brackets inside one another) is shown
verbatim as an error instead of overflowing.

---

## Display math and aligned derivations

A line starting with `;;` is a display formula, and consecutive `;;` lines
form **one block**. Each line is a row, and rows **align automatically at
their first relation** (`=`, `<`, `<=`, `->`, `in`, …). A row that starts
with a relation continues the derivation:

```
;; (a+b)^2 = (a+b)(a+b)
;; = a^2 + 2a b + b^2
```

To set alignment points yourself, put `&` in a row. The row is then split
only at its `&` characters. A trailing `;;` on a display line is allowed and
ignored. A line that starts with `;;` but closes and continues with text
(`;;x;; is positive`) is a paragraph that begins with inline math. A `;;` with
nothing after it is an empty row and is skipped.

LaTeX export: one unaligned row becomes plain math. Several rows (or aligned
cells) become `\begin{aligned} … \end{aligned}`, with `&` placed before the
first relation and `\\` between rows.

---

## Symbol reference

Every keyword and operator spelling below is defined in one data file,
[`crates/qm-format/src/math/syntax.toml`](../crates/qm-format/src/math/syntax.toml),
which is compiled into the program. To add syntax, add a line to the right
section of that file and rebuild; no Rust code changes are needed. For
example, `integral2 = ["∬", '\iint']` under `[integral]` defines `integral2`
for the preview and every export. The header of the file explains each
section. A mistake in the file stops the build with a message naming the
entry. The editor's Notation panel lists every entry under **Symbols**, so it
always matches the file.

**Greek:** `alpha beta gamma delta eps/epsilon zeta eta theta vartheta iota kappa
lambda mu nu xi pi varpi rho varrho sigma varsigma tau upsilon phi chi psi
omega`. Lowercase Greek letters are italic. Capitals are upright: `Gamma Delta
Theta Lambda Xi Pi Sigma Upsilon Phi Psi Omega`. `eps` is ε (`\varepsilon`) and
`phi` is φ (`\varphi`).

**Letter-like:** `oo`/`inf`/`infty` ∞, `del`/`partial` ∂, `grad`/`nabla` ∇,
`RR NN ZZ QQ CC` ℝ ℕ ℤ ℚ ℂ, `forall` ∀, `exists` ∃, `nexists` ∄,
`not`/`neg` ¬, `empty`/`emptyset` ∅, `aleph` ℵ, `hbar` ℏ, `ell` ℓ,
`angle` ∠, `triangle` △, `qed` ∎, `...`/`dots`/`ldots` …, `cdots` ⋯,
`vdots` ⋮, `ddots` ⋱.

**Relations:** `=`, `/=` `!=` `ne` ≠, `<`, `>`, `<=` `le` ≤, `>=` `ge` ≥,
`~=` `approx` ≈, `==` `equiv` ≡, `~` `sim` ∼, `simeq` ≃, `cong` ≅,
`prop` ∝, `<<` `ll` ≪, `>>` `gg` ≫, `:=` ≔, `in` ∈, `!in` `notin` ∉,
`sub` ⊂, `sube` ⊆, `supset` ⊃, `supe` ⊇, `perp` ⊥, `parallel` ∥, `mid` ∣,
`->` `to` →, `<-` ←, `=>` `implies` ⇒, `<=>` `iff` ⇔, `|->` `mapsto` ↦,
`therefore` ∴, `because` ∵.

**Binary:** `+`, `-` (−), `*` `cdot` ⋅, `xx` `times` ×, `div` ÷, `+-` `pm` ±,
`-+` `mp` ∓, `cup` ∪, `cap` ∩, `and` `wedge` ∧, `or` `vee` ∨, `setminus` ∖,
`@` `circ` ∘, `ox` `otimes` ⊗, `oplus` ⊕, `star` ⋆, `ast` ∗, `mod`.
`//` is a literal slash (no fraction). `||` is ‖.

**Functions (upright operator atoms, spaced from their argument like TeX's
`\sin`; scripts always sit to the right, and they never take spoken limits):** `sin cos tan sec csc cot arcsin arccos arctan sinh cosh
tanh coth log ln lg exp det dim ker gcd lcm deg arg hom Pr sgn tr rank span Re
Im`.

**Spacing:** `quad`, `qquad`, `\,`, `\:`, `\;`.

---

## Cheat sheet

The editor's help panel shows the same examples (from `qm_format::cheat_sheet()`),
followed by every word and operator in the syntax file (from
`qm_format::vocabulary()`).
Math inputs are written as they appear between `;;`.

| QuickMath | Result | LaTeX |
|---|---|---|
| `a+b/c+d` | (a+b)/(c+d) | `\frac{a+b}{c+d}` |
| `a + b/c` | a + b/c | `a+\frac{b}{c}` |
| `dy/dx` | dy/dx | `\frac{dy}{dx}` |
| `1/sqrt 2` | 1/√2 | `\frac{1}{\sqrt{2}}` |
| `y = 1 + x over 1 - x` | y = (1+x)/(1−x) | `y=\frac{1+x}{1-x}` |
| `a/b/c` | (a/b)/c | `\frac{\frac{a}{b}}{c}` |
| `x^n+1` | xⁿ⁺¹ | `x^{n+1}` |
| `x^2 + 1` | x² + 1 | `x^{2}+1` |
| `x_i^2` | xᵢ² | `x_{i}^{2}` |
| `x_i+1` | xᵢ₊₁ | `x_{i+1}` |
| `x1^2` | x₁² | `x_{1}^{2}` |
| `e^[-x^2/2]` | e^(−x²/2) | `e^{\frac{-x^{2}}{2}}` |
| `x sq`, `x cb`, `A inv` | x², x³, A⁻¹ | `x^{2}`, `x^{3}`, `A^{-1}` |
| `z^*` | z* | `z^{*}` |
| `f'(x)` | f′(x) | `f^{\prime}\left(x\right)` |
| `sqrt x+1` | √(x+1) | `\sqrt{x+1}` |
| `root 3 x` | ∛x | `\sqrt[3]{x}` |
| `abs x`, `norm v` | \|x\|, ‖v‖ | `\left\|x\right\|`, `\left\\|v\right\\|` |
| `floor x`, `ceil x` | ⌊x⌋, ⌈x⌉ | `\left\lfloor x\right\rfloor`, `\left\lceil x\right\rceil` |
| `hat x`, `bar x`, `vec v` | x̂, x̄, v⃗ | `\hat{x}`, `\bar{x}`, `\vec{v}` |
| `hat(x+y)` | wide hat | `\widehat{x+y}` |
| `bb v`, `cal A`, `bbb R` | **v**, 𝒜, ℝ | `\mathbf{v}`, `\mathcal{A}`, `\mathbb{R}` |
| `RR NN ZZ QQ CC` | ℝ ℕ ℤ ℚ ℂ | `\mathbb{R}` … |
| `alpha beta Gamma` | α β Γ | `\alpha\beta\Gamma` |
| `oo`, `del`, `grad` | ∞, ∂, ∇ | `\infty`, `\partial`, `\nabla` |
| `sum i=1 to n` | ∑ with limits | `\sum_{i=1}^{n}` |
| `int 0 to 1 f(x) dx` | ∫₀¹ f(x) dx | `\int_{0}^{1}f\left(x\right)dx` |
| `sum x in S f(x)` | ∑ over x∈S | `\sum_{x\in S}f\left(x\right)` |
| `lim x to 0 (sin x)/x` | lim sin x / x | `\lim_{x\to0}\frac{\sin x}{x}` |
| `max x in S f(x)` | max over S | `\max_{x\in S}f\left(x\right)` |
| `inf_n a_n` | infimum | `\inf_{n}a_{n}` |
| `sum_[i=1]^n` | ∑ with limits | `\sum_{i=1}^{n}` |
| `sin x`, `log_2 n` | sin x, log₂ n | `\sin x`, `\log_{2}n` |
| `<= >= /= ~= ==` | ≤ ≥ ≠ ≈ ≡ | `\leq\geq\neq\approx\equiv` |
| `-> => <=> \|->` | → ⇒ ⇔ ↦ | `\to\Rightarrow\Leftrightarrow\mapsto` |
| `f: A to B` | f: A → B | `f:A\to B` |
| `in !in sub sube` | ∈ ∉ ⊂ ⊆ | `\in\notin\subset\subseteq` |
| `xx  *  pm` | × ⋅ ± | `\times\cdot\pm` |
| `cup cap and or not` | ∪ ∩ ∧ ∨ ¬ | `\cup\cap\wedge\vee\neg` |
| `forall exists` | ∀ ∃ | `\forall\exists` |
| `... cdots` | … ⋯ | `\ldots\cdots` |
| `[0, 1)` | [0, 1) | `\left[0,1\right)` |
| `{x in RR : x > 0}` | {x∈ℝ : x>0} | `\left\{x\in\mathbb{R}:x>0\right\}` |
| `mat 1 2, 3 4` | 2×2 matrix | `\begin{pmatrix}1 & 2 \\ 3 & 4\end{pmatrix}` |
| `A = mat[1 0, 0 1]` | A = identity | `A=\begin{pmatrix}1 & 0 \\ 0 & 1\end{pmatrix}` |
| `bmat a b, c d` | [matrix] | `\begin{bmatrix}a & b \\ c & d\end{bmatrix}` |
| `vmat a b, c d` | determinant | `\begin{vmatrix}a & b \\ c & d\end{vmatrix}` |
| `cases x if x >= 0, -x otherwise` | piecewise | `\begin{cases}x & \text{ if }x\geq0 \\ -x & \text{ otherwise }\end{cases}` |
| `` `any text` `` | upright text | `\text{any text}` |

Document-level markup:

| QuickMath | Result |
|---|---|
| `# Title` … `###### Title` | headings |
| `*em*`, `**strong**`, `` `code` `` | emphasis, strong, code |
| `- item`, `1. item` (indent 2 to nest) | lists |
| `[[Note]]`, `[[Note\|label]]` | links to other notes |
| `.def Group` (also `.thm .lem .cor .proof .ex .note .warn`) | callout box |
| `;;x^2;;` | inline math |
| `;; f(x) = x^2` | display math line |
| `;; = 2x + 1` | continue an aligned derivation |
| `> quote` | quote |
| `---` | horizontal rule |
| `\;;` | a literal `;;` |

See [`examples/lecture.qm`](examples/lecture.qm) for a complete lecture note.
