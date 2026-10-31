//! Builds a [`MathList`] from math tokens.
//!
//! Grouping follows the "pen order" rules from `docs/FORMAT.md`:
//!
//! * A *run* is a stretch of tokens without whitespace. Relations (`=`, `<`,
//!   `in`, …), punctuation (`,` `;` `:`) and prose words (`if`, …) always end
//!   a run. Inside a run `/` binds weakest, so `a+b/c+d` is (a+b)/(c+d).
//! * `^` takes the rest of its run as the superscript (`x^n+1`); `_` takes
//!   the rest of the run up to a `^` (`x_i+1^2`).
//! * Prefix words (`sqrt`, `abs`, `hat`, …) take a directly attached bracket
//!   group, or otherwise the rest of the run; a space right after the word
//!   does not end that run (`sqrt x+1`).
//! * A bracket group that forms a whole operand is invisible, so
//!   `(a+b)/(c+d)` and `x^[n+1]` drop their brackets.
//! * Sequence-level words: `over` (big fraction), `sq`/`cb`/`inv` (postfix
//!   powers), word limits (`sum i=1 to n`, `lim x to 0`), `mat`/`cases`.
//!
//! The parser never fails: every token ends up in the tree, and anything that
//! cannot be interpreted becomes [`MathNode::Error`].

use crate::ast::{Atom, AtomClass, AtomStyle, MathList, MathNode, SpaceWidth};
use crate::math::lexer::{lex, Token, TokenKind};
use crate::math::symbols::{self, Keyword, Symbol};

/// Bounds recursion so pathological input (`((((…`, `x^x^x^…`) cannot
/// overflow the stack; deeper input is shown verbatim as an error.
const MAX_DEPTH: usize = 48;
/// Bounds how many `/` in one run are turned into nested fractions.
const MAX_FRACTION_PARTS: usize = 16;

/// Parses math source into a list of nodes.
pub(crate) fn parse_math(source: &str) -> MathList {
    let parser = MathParser::new(source);
    parser.parse_sequence(0, parser.tokens.len(), 0)
}

/// How a single token behaves when splitting a sequence into runs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Role {
    Relation,
    Punctuation,
    Prose,
    Over,
    Postfix(Symbol),
    /// `mat`, `cases`, …
    MatrixWord,
    /// `sqrt`, `abs`, `hat`, …: takes one argument.
    Prefix,
    /// `root`: takes an index and a radicand.
    RootWord,
    LargeOperator,
    Plain,
}

impl Role {
    fn ends_run(self) -> bool {
        matches!(
            self,
            Role::Relation | Role::Punctuation | Role::Prose | Role::Over | Role::Postfix(_)
        )
    }

    fn argument_count(self) -> usize {
        match self {
            Role::Prefix => 1,
            Role::RootWord => 2,
            _ => 0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ItemKind {
    Run,
    Breaker(Role),
    /// `mat 1 2, 3 4` without brackets: the matrix takes the rest of the sequence.
    MatrixToEnd,
}

/// A run or a breaker token within a sequence, as a token range.
#[derive(Debug, Clone, Copy)]
struct Item {
    start: usize,
    end: usize,
    has_space_before: bool,
    kind: ItemKind,
}

/// Where a whitespace-separated stretch of elements ends.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Stretch {
    /// Also stops at relations, punctuation and prose words.
    Run,
    /// Stops only at whitespace (matrix cells, cases).
    Chunk,
}

/// Output of the sequence pass before `over` is resolved.
enum Piece {
    Nodes(MathList),
    /// Relations and punctuation; `over` fractions never cross these.
    Boundary(MathNode),
    Over,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ScriptSlot {
    Sub,
    Sup,
}

struct MathParser<'a> {
    source: &'a str,
    tokens: Vec<Token>,
    /// For each opening bracket, the index of its closing bracket if present.
    closers: Vec<Option<usize>>,
    /// Each token's role, computed once because run splitting asks often.
    roles: Vec<Role>,
}

impl<'a> MathParser<'a> {
    fn new(source: &'a str) -> Self {
        let tokens = lex(source);
        let closers = match_brackets(&tokens);
        let mut parser = MathParser {
            source,
            tokens,
            closers,
            roles: Vec::new(),
        };
        parser.roles = (0..parser.tokens.len())
            .map(|index| parser.compute_role(index))
            .collect();
        parser
    }

    fn text(&self, index: usize) -> &'a str {
        let token = self.tokens[index];
        &self.source[token.start..token.end]
    }

    fn range_text(&self, start: usize, end: usize) -> &'a str {
        if start >= end {
            return "";
        }
        &self.source[self.tokens[start].start..self.tokens[end - 1].end]
    }

    fn kind(&self, index: usize) -> TokenKind {
        self.tokens[index].kind
    }

    /// Index just past the element starting at `index` (a token or a whole
    /// bracket group), clamped to `end`.
    fn element_end(&self, index: usize, end: usize) -> usize {
        match self.closers[index] {
            Some(closer) if self.kind(index) == TokenKind::Open => (closer + 1).min(end),
            None if self.kind(index) == TokenKind::Open => end,
            _ => index + 1,
        }
    }

    fn is_single_group(&self, start: usize, end: usize) -> bool {
        start < end && self.kind(start) == TokenKind::Open && self.element_end(start, end) == end
    }

    fn is_attached_group(&self, index: usize, end: usize) -> bool {
        index < end && self.kind(index) == TokenKind::Open && !self.tokens[index].has_space_before
    }

    /// The meaning of a word token, honouring the backslash command form.
    fn keyword_at(&self, index: usize) -> Option<Keyword> {
        match self.kind(index) {
            TokenKind::Word => symbols::keyword(self.text(index)),
            TokenKind::Command => symbols::keyword(&self.text(index)[1..]),
            _ => None,
        }
    }

    fn role(&self, index: usize) -> Role {
        self.roles[index]
    }

    fn compute_role(&self, index: usize) -> Role {
        if self.kind(index) == TokenKind::Operator {
            return match symbols::operator_class(self.text(index)) {
                AtomClass::Relation => Role::Relation,
                AtomClass::Punctuation => Role::Punctuation,
                _ => Role::Plain,
            };
        }
        match self.keyword_at(index) {
            Some(Keyword::Symbol(symbol)) => match symbol.class {
                AtomClass::Relation => Role::Relation,
                AtomClass::LargeOperator { .. } => Role::LargeOperator,
                _ => Role::Plain,
            },
            Some(Keyword::Prose) => Role::Prose,
            Some(Keyword::Over) => Role::Over,
            Some(Keyword::Postfix(symbol)) => Role::Postfix(symbol),
            Some(Keyword::Matrix { .. }) => Role::MatrixWord,
            Some(Keyword::Root) => Role::RootWord,
            Some(Keyword::Sqrt | Keyword::Fence { .. } | Keyword::Accent(_) | Keyword::Font(_)) => {
                Role::Prefix
            }
            Some(Keyword::Space { .. }) | None => Role::Plain,
        }
    }

    /// End of the run or chunk starting at `start`. Whitespace does not end
    /// it right after a prefix word (so the word reaches its argument) or on
    /// either side of a `/` (so `x^2 / 2` divides its neighbours).
    fn stretch_end(&self, start: usize, end: usize, stretch: Stretch) -> usize {
        let mut pending_arguments = self.role(start).argument_count();
        let mut follows_slash = self.kind(start) == TokenKind::Slash;
        let mut index = self.element_end(start, end);
        while index < end {
            let role = self.role(index);
            if stretch == Stretch::Run && role.ends_run() {
                break;
            }
            let is_slash = self.kind(index) == TokenKind::Slash;
            let is_glued = pending_arguments > 0 || follows_slash || is_slash;
            if self.tokens[index].has_space_before && !is_glued {
                break;
            }
            pending_arguments = pending_arguments.saturating_sub(1) + role.argument_count();
            follows_slash = is_slash;
            index = self.element_end(index, end);
        }
        index
    }

    /// Indices of top-level tokens in `start..end` (bracket groups skipped).
    fn top_level(&self, start: usize, end: usize) -> impl Iterator<Item = usize> + '_ {
        let mut next = start;
        std::iter::from_fn(move || {
            let index = next;
            (index < end).then(|| {
                next = self.element_end(index, end);
                index
            })
        })
    }

    // ----- sequences -------------------------------------------------------

    fn parse_sequence(&self, start: usize, end: usize, depth: usize) -> MathList {
        if start >= end {
            return Vec::new();
        }
        if depth > MAX_DEPTH {
            return vec![self.too_deep(start, end)];
        }
        if end == start + 1 && self.is_self_contained(start) {
            // Fast path for the very common one-token operand (`x^2`, `a_i`).
            let mut nodes = Vec::with_capacity(1);
            self.parse_element(start, end, depth, &mut nodes);
            return nodes;
        }
        let items = self.split_items(start, end);
        let mut pieces = Vec::new();
        let mut index = 0;
        while index < items.len() {
            index = self.parse_item(&items, index, depth, &mut pieces);
        }
        resolve_over(pieces)
    }

    /// Whether a lone token means the same inside a run as on its own.
    fn is_self_contained(&self, index: usize) -> bool {
        !matches!(
            self.role(index),
            Role::Prose | Role::Over | Role::Postfix(_) | Role::MatrixWord
        )
    }

    fn too_deep(&self, start: usize, end: usize) -> MathNode {
        error_node(self.range_text(start, end), "nested too deeply to display")
    }

    fn split_items(&self, start: usize, end: usize) -> Vec<Item> {
        let mut items = Vec::new();
        let mut index = start;
        while index < end {
            let has_space_before = self.tokens[index].has_space_before;
            let role = self.role(index);
            let (item_end, kind) = if role.ends_run() {
                (index + 1, ItemKind::Breaker(role))
            } else if role == Role::MatrixWord && !self.is_attached_group(index + 1, end) {
                (end, ItemKind::MatrixToEnd)
            } else {
                (self.stretch_end(index, end, Stretch::Run), ItemKind::Run)
            };
            items.push(Item {
                start: index,
                end: item_end,
                has_space_before,
                kind,
            });
            index = item_end;
        }
        items
    }

    /// Parses `items[index]` (and, for word limits, the items it consumes),
    /// returning the index of the next unparsed item.
    fn parse_item(
        &self,
        items: &[Item],
        index: usize,
        depth: usize,
        pieces: &mut Vec<Piece>,
    ) -> usize {
        let item = items[index];
        match item.kind {
            ItemKind::Run => {
                if let Some((node, next)) = self.parse_word_limits(items, index, depth) {
                    push_nodes(pieces, vec![node]);
                    return next;
                }
                push_nodes(pieces, self.parse_run(item.start, item.end, depth));
            }
            ItemKind::MatrixToEnd => {
                push_nodes(
                    pieces,
                    vec![self.parse_matrix_word(item.start, item.start + 1, item.end, depth)],
                );
            }
            ItemKind::Breaker(Role::Over) => pieces.push(Piece::Over),
            ItemKind::Breaker(Role::Prose) => {
                push_nodes(pieces, vec![prose_node(self.text(item.start))])
            }
            ItemKind::Breaker(Role::Postfix(symbol)) => attach_postfix(pieces, symbol),
            ItemKind::Breaker(_) => pieces.push(Piece::Boundary(self.parse_leaf(item.start))),
        }
        index + 1
    }

    // ----- word limits: `sum i=1 to n`, `lim x to 0` -----------------------

    /// Recognises a lone large operator followed by spoken limits.
    fn parse_word_limits(
        &self,
        items: &[Item],
        index: usize,
        depth: usize,
    ) -> Option<(MathNode, usize)> {
        let item = items[index];
        if item.end - item.start != 1 || self.role(item.start) != Role::LargeOperator {
            return None;
        }
        let MathNode::Atom(operator) = self.parse_leaf(item.start) else {
            return None;
        };
        if !symbols::takes_word_limits(&operator) {
            return None;
        }
        let reads_to_as_arrow = symbols::reads_to_as_arrow(&operator);
        let lower_first = index + 1;
        let mut next = chunk_end(items, lower_first)?;
        let mut has_limit =
            self.chunk_has_limit_relation(&items[lower_first..next], reads_to_as_arrow);
        if next < items.len() && self.is_limit_joiner(items[next], reads_to_as_arrow) {
            if let Some(after) = chunk_end(items, next + 1) {
                next = after;
                has_limit = true;
            }
        }
        let lower = self.parse_operand(items[lower_first].start, items[next - 1].end, depth + 1);
        let mut upper = None;
        if !reads_to_as_arrow && next < items.len() && self.is_word(items[next].start, "to") {
            let upper_first = next + 1;
            let upper_end = chunk_end(items, upper_first).unwrap_or(upper_first);
            upper = Some(match upper_end > upper_first {
                true => self.parse_operand(
                    items[upper_first].start,
                    items[upper_end - 1].end,
                    depth + 1,
                ),
                false => Vec::new(),
            });
            next = upper_end;
        }
        if !has_limit && upper.is_none() {
            return None;
        }
        let base = Box::new(MathNode::Atom(operator));
        Some((
            MathNode::Scripts {
                base,
                sub: Some(lower),
                sup: upper,
            },
            next,
        ))
    }

    fn is_word(&self, index: usize, word: &str) -> bool {
        self.kind(index) == TokenKind::Word && self.text(index) == word
    }

    /// Whether a relation inside the first limit chunk makes it a limit:
    /// any relation for `lim`-like operators (`x->0`), only `in` for sums
    /// (`sum i=1` is ambiguous with `sum a_n=1` unless `to` follows).
    fn chunk_has_limit_relation(&self, chunk: &[Item], reads_to_as_arrow: bool) -> bool {
        chunk.iter().any(|item| {
            item.kind == ItemKind::Breaker(Role::Relation)
                && (reads_to_as_arrow || self.relation_text(item.start) == "∈")
        })
    }

    /// A spaced relation joining a second chunk into the lower limit:
    /// `lim x to 0`, `lim x -> 0`, `max x in S`, `sum x in S`.
    fn is_limit_joiner(&self, item: Item, reads_to_as_arrow: bool) -> bool {
        if item.kind != ItemKind::Breaker(Role::Relation) {
            return false;
        }
        match self.relation_text(item.start) {
            "∈" => true,
            "→" => reads_to_as_arrow,
            _ => false,
        }
    }

    /// The Unicode text of a relation token without building its atom.
    fn relation_text(&self, index: usize) -> &'static str {
        match self.keyword_at(index) {
            Some(Keyword::Symbol(symbol)) => symbol.text,
            _ => symbols::operator_text(self.text(index)),
        }
    }

    // ----- runs ------------------------------------------------------------

    /// A run splits at its top-level `/` first (weakest binding).
    fn parse_run(&self, start: usize, end: usize, depth: usize) -> MathList {
        let mut slashes: Vec<usize> = self
            .top_level(start, end)
            .filter(|&i| self.kind(i) == TokenKind::Slash)
            .collect();
        if slashes.is_empty() {
            return self.parse_simple_run(start, end, depth);
        }
        slashes.truncate(MAX_FRACTION_PARTS - 1);
        let mut fraction = self.parse_operand(start, slashes[0], depth + 1);
        for (position, &slash) in slashes.iter().enumerate() {
            let part_end = slashes.get(position + 1).copied().unwrap_or(end);
            let denominator = self.parse_operand(slash + 1, part_end, depth + 1);
            fraction = vec![MathNode::Fraction {
                numerator: fraction,
                denominator,
            }];
        }
        fraction
    }

    /// An operand: a lone bracket group loses its brackets.
    fn parse_operand(&self, start: usize, end: usize, depth: usize) -> MathList {
        if self.is_single_group(start, end) {
            let inner_end = self.closers[start].unwrap_or(end).min(end);
            return self.parse_sequence(start + 1, inner_end, depth + 1);
        }
        self.parse_sequence(start, end, depth + 1)
    }

    /// A run without top-level `/`: primaries, prefix words and scripts.
    fn parse_simple_run(&self, start: usize, end: usize, depth: usize) -> MathList {
        let mut nodes = Vec::new();
        let mut index = start;
        while index < end {
            index = match self.kind(index) {
                TokenKind::Underscore | TokenKind::Caret | TokenKind::Prime => {
                    let base = nodes.pop().unwrap_or_else(|| MathNode::Group(Vec::new()));
                    let (node, next) = self.parse_scripts(base, index, end, depth);
                    nodes.push(node);
                    next
                }
                _ => self.parse_element(index, end, depth, &mut nodes),
            };
        }
        nodes
    }

    fn parse_scripts(
        &self,
        base: MathNode,
        start: usize,
        end: usize,
        depth: usize,
    ) -> (MathNode, usize) {
        let mut node = base;
        let mut index = start;
        while index < end {
            match self.kind(index) {
                TokenKind::Prime => {
                    let primes_end = (index..end)
                        .find(|&i| self.kind(i) != TokenKind::Prime)
                        .unwrap_or(end);
                    let primes = (index..primes_end)
                        .map(|_| MathNode::Atom(symbols::PRIME.to_atom()))
                        .collect();
                    node = attach_script(node, ScriptSlot::Sup, primes);
                    index = primes_end;
                }
                TokenKind::Underscore => {
                    let operand_end = self
                        .top_level(index + 1, end)
                        .find(|&i| self.kind(i) == TokenKind::Caret)
                        .unwrap_or(end);
                    let script = self.parse_script_operand(index + 1, operand_end, depth);
                    node = attach_script(node, ScriptSlot::Sub, script);
                    index = operand_end;
                }
                TokenKind::Caret => {
                    let script = self.parse_script_operand(index + 1, end, depth);
                    node = attach_script(node, ScriptSlot::Sup, script);
                    index = end;
                }
                _ => break,
            }
        }
        (node, index)
    }

    fn parse_script_operand(&self, start: usize, end: usize, depth: usize) -> MathList {
        if end == start + 1 && self.kind(start) == TokenKind::Operator && self.text(start) == "*" {
            return vec![MathNode::Atom(symbols::ASTERISK.to_atom())];
        }
        self.parse_operand(start, end, depth + 1)
    }

    /// Parses the element at `index` into `nodes`; returns the next index.
    fn parse_element(&self, index: usize, end: usize, depth: usize, nodes: &mut MathList) -> usize {
        match self.kind(index) {
            TokenKind::Word | TokenKind::Command => self.parse_word(index, end, depth, nodes),
            TokenKind::Open => {
                nodes.push(self.parse_group(index, end, depth));
                self.element_end(index, end)
            }
            _ => {
                nodes.push(self.parse_leaf(index));
                self.attach_digit_subscript(index + 1, end, nodes)
            }
        }
    }

    fn parse_word(&self, index: usize, end: usize, depth: usize, nodes: &mut MathList) -> usize {
        let after = index + 1;
        match self.keyword_at(index) {
            Some(Keyword::Sqrt) => {
                let (radicand, next) = self.parse_prefix_argument(after, end, depth);
                nodes.push(MathNode::Root {
                    index: None,
                    radicand,
                });
                next
            }
            Some(Keyword::Root) => {
                let (node, next) = self.parse_root(after, end, depth);
                nodes.push(node);
                next
            }
            Some(Keyword::Fence { open, close }) => {
                let (body, next) = self.parse_prefix_argument(after, end, depth);
                nodes.push(MathNode::Fenced {
                    open: open.to_string(),
                    close: close.to_string(),
                    body,
                });
                next
            }
            Some(Keyword::Accent(accent)) => {
                let (body, next) = self.parse_prefix_argument(after, end, depth);
                nodes.push(MathNode::Accent { accent, body });
                next
            }
            Some(Keyword::Font(font)) => {
                let (body, next) = self.parse_prefix_argument(after, end, depth);
                nodes.extend(
                    body.into_iter()
                        .map(|node| map_atoms(node, &|a| symbols::apply_font_to_atom(font, a))),
                );
                next
            }
            Some(Keyword::Matrix { .. }) => {
                let group_end = if self.is_attached_group(after, end) {
                    self.element_end(after, end)
                } else {
                    after
                };
                nodes.push(self.parse_matrix_word(index, after, group_end, depth));
                group_end
            }
            Some(Keyword::Space { width, count }) => {
                nodes.extend(std::iter::repeat_n(MathNode::Space(width), count));
                after
            }
            Some(Keyword::Postfix(symbol)) => {
                let base = nodes.pop().unwrap_or_else(|| MathNode::Group(Vec::new()));
                nodes.push(attach_script(
                    base,
                    ScriptSlot::Sup,
                    vec![MathNode::Atom(symbol.to_atom())],
                ));
                after
            }
            Some(Keyword::Prose | Keyword::Over) => {
                nodes.push(prose_node(self.text(index)));
                after
            }
            Some(Keyword::Symbol(_)) | None => {
                self.push_word_atoms(index, end, nodes);
                self.attach_digit_subscript(after, end, nodes)
            }
        }
    }

    fn push_word_atoms(&self, index: usize, end: usize, nodes: &mut MathList) {
        let word = self.text(index);
        let is_command = self.kind(index) == TokenKind::Command;
        let is_infimum =
            (word == "inf" && index + 1 < end && self.kind(index + 1) == TokenKind::Underscore)
                || (is_command && word == "\\inf");
        if is_infimum {
            nodes.push(MathNode::Atom(symbols::INFIMUM.to_atom()));
            return;
        }
        match self.keyword_at(index) {
            Some(Keyword::Symbol(symbol)) => nodes.push(MathNode::Atom(symbol.to_atom())),
            _ if is_command => nodes.push(error_node(word, &format!("unknown command `{word}`"))),
            _ => nodes.extend(word.chars().map(|c| MathNode::Atom(letter_atom(c)))),
        }
    }

    /// `x1` → x₁: a number directly after an italic variable is its subscript.
    fn attach_digit_subscript(&self, index: usize, end: usize, nodes: &mut MathList) -> usize {
        let is_number_next = index < end && self.kind(index) == TokenKind::Number;
        let is_variable =
            matches!(nodes.last(), Some(MathNode::Atom(atom)) if atom.style == AtomStyle::Italic);
        if !is_number_next || !is_variable {
            return index;
        }
        let base = nodes.pop().unwrap_or_else(|| MathNode::Group(Vec::new()));
        nodes.push(attach_script(
            base,
            ScriptSlot::Sub,
            vec![self.parse_leaf(index)],
        ));
        index + 1
    }

    /// The argument of a prefix word: an attached bracket group, or else the
    /// rest of the run.
    fn parse_prefix_argument(&self, start: usize, end: usize, depth: usize) -> (MathList, usize) {
        if self.is_attached_group(start, end) {
            let group_end = self.element_end(start, end);
            return (self.parse_operand(start, group_end, depth + 1), group_end);
        }
        (self.parse_operand(start, end, depth + 1), end)
    }

    /// `root 3 x`, `root(3)(x)` or `root(3, x)`.
    fn parse_root(&self, start: usize, end: usize, depth: usize) -> (MathNode, usize) {
        if start >= end {
            return (
                MathNode::Root {
                    index: Some(Vec::new()),
                    radicand: Vec::new(),
                },
                start,
            );
        }
        let index_end = self.element_end(start, end);
        if self.is_single_group(start, index_end) {
            if let Some(node) = self.parse_root_with_comma(start, index_end, depth) {
                return (node, index_end);
            }
        }
        let root_index = self.parse_operand(start, index_end, depth + 1);
        let (radicand, next) = self.parse_prefix_argument(index_end, end, depth);
        (
            MathNode::Root {
                index: Some(root_index),
                radicand,
            },
            next,
        )
    }

    fn parse_root_with_comma(
        &self,
        group_start: usize,
        group_end: usize,
        depth: usize,
    ) -> Option<MathNode> {
        let inner_end = self.closers[group_start]
            .unwrap_or(group_end)
            .min(group_end);
        let comma = self
            .top_level(group_start + 1, inner_end)
            .find(|&i| self.kind(i) == TokenKind::Operator && self.text(i) == ",")?;
        let root_index = self.parse_sequence(group_start + 1, comma, depth + 1);
        let radicand = self.parse_sequence(comma + 1, inner_end, depth + 1);
        Some(MathNode::Root {
            index: Some(root_index),
            radicand,
        })
    }

    fn parse_group(&self, index: usize, end: usize, depth: usize) -> MathNode {
        let closer = self.closers[index].filter(|&closer| closer < end);
        let inner_end = closer.unwrap_or(end);
        let body = self.parse_sequence(index + 1, inner_end, depth + 1);
        let open = self.text(index).to_string();
        let close = closer.map_or(String::new(), |closer| self.text(closer).to_string());
        make_fenced(open, close, body)
    }

    // ----- matrices --------------------------------------------------------

    /// `mat 1 2, 3 4`, `mat[a b, c d]`, `cases x if x >= 0, -x otherwise`.
    /// `body_start..body_end` is either an attached group or the rest.
    fn parse_matrix_word(
        &self,
        word: usize,
        body_start: usize,
        body_end: usize,
        depth: usize,
    ) -> MathNode {
        let Some(Keyword::Matrix { open, close }) = self.keyword_at(word) else {
            return error_node(self.text(word), "expected a matrix");
        };
        let (start, end) = if self.is_single_group(body_start, body_end) {
            (
                body_start + 1,
                self.closers[body_start].unwrap_or(body_end).min(body_end),
            )
        } else {
            (body_start, body_end)
        };
        let is_cases = open == "{" && close.is_empty();
        let rows = self
            .split_top_level(start, end, |parser, i| parser.is_row_separator(i))
            .into_iter()
            .map(|(row_start, row_end)| self.parse_matrix_row(row_start, row_end, is_cases, depth))
            .collect();
        MathNode::Matrix {
            open: open.to_string(),
            close: close.to_string(),
            rows,
        }
    }

    fn is_row_separator(&self, index: usize) -> bool {
        self.kind(index) == TokenKind::Operator && matches!(self.text(index), "," | ";")
    }

    fn parse_matrix_row(
        &self,
        start: usize,
        end: usize,
        is_cases: bool,
        depth: usize,
    ) -> Vec<MathList> {
        let cells = match is_cases {
            true => self.split_case_row(start, end),
            false => self.split_chunks(start, end),
        };
        cells
            .into_iter()
            .map(|(cell_start, cell_end)| self.parse_operand(cell_start, cell_end, depth + 1))
            .collect()
    }

    /// A case row splits before its first prose word (`if`, `otherwise`), or
    /// else after its first chunk.
    fn split_case_row(&self, start: usize, end: usize) -> Vec<(usize, usize)> {
        let prose = self
            .top_level(start, end)
            .find(|&i| self.role(i) == Role::Prose);
        let split = match prose {
            Some(position) => position,
            None if start < end => self.stretch_end(start, end, Stretch::Chunk),
            None => end,
        };
        match (start < split, split < end) {
            (true, true) => vec![(start, split), (split, end)],
            (false, true) => vec![(split, split), (split, end)],
            _ => vec![(start, end)],
        }
    }

    fn split_chunks(&self, start: usize, end: usize) -> Vec<(usize, usize)> {
        if start >= end {
            return vec![(start, end)];
        }
        let mut chunks = Vec::new();
        let mut index = start;
        while index < end {
            let chunk_end = self.stretch_end(index, end, Stretch::Chunk);
            chunks.push((index, chunk_end));
            index = chunk_end;
        }
        chunks
    }

    /// Splits `start..end` at top-level tokens matching `is_separator`.
    fn split_top_level(
        &self,
        start: usize,
        end: usize,
        is_separator: impl Fn(&Self, usize) -> bool,
    ) -> Vec<(usize, usize)> {
        let mut parts = Vec::new();
        let mut part_start = start;
        for index in self.top_level(start, end) {
            if is_separator(self, index) {
                parts.push((part_start, index));
                part_start = index + 1;
            }
        }
        parts.push((part_start, end));
        parts
    }

    // ----- single tokens ---------------------------------------------------

    /// A node for a single token that has no structure of its own.
    fn parse_leaf(&self, index: usize) -> MathNode {
        let text = self.text(index);
        match self.kind(index) {
            TokenKind::Number => MathNode::Atom(Atom {
                text: text.to_string(),
                latex: text.to_string(),
                class: AtomClass::Ordinary,
                style: AtomStyle::Upright,
            }),
            TokenKind::Operator => MathNode::Atom(symbols::operator_atom(text)),
            TokenKind::Word | TokenKind::Command => {
                let mut nodes = Vec::new();
                self.push_word_atoms(index, index + 1, &mut nodes);
                match nodes.len() {
                    1 => nodes.remove(0),
                    _ => MathNode::Group(nodes),
                }
            }
            TokenKind::Quote { is_closed } => {
                self.quoted_text(index, quote_content(text, is_closed))
            }
            TokenKind::Escape => escaped_node(text),
            TokenKind::LoneBackslash => {
                error_node(text, "a backslash must be followed by a symbol")
            }
            TokenKind::Close => MathNode::Atom(stray_closer(text)),
            TokenKind::Slash => MathNode::Atom(symbols::operator_atom("//")),
            TokenKind::Open
            | TokenKind::Underscore
            | TokenKind::Caret
            | TokenKind::Prime
            | TokenKind::Other => text
                .chars()
                .next()
                .map_or(MathNode::Group(Vec::new()), |c| {
                    MathNode::Atom(symbols::atom_for_char(c))
                }),
        }
    }
}

impl MathParser<'_> {
    /// Quoted text keeps one space on each side where the source had
    /// whitespace next to a neighbouring token, since math spacing would
    /// otherwise glue prose to formulas (`a_n "converges"`).
    fn quoted_text(&self, index: usize, content: &str) -> MathNode {
        let has_space_before = index > 0 && self.tokens[index].has_space_before;
        let has_space_after = self
            .tokens
            .get(index + 1)
            .is_some_and(|next| next.has_space_before);
        let mut text = String::with_capacity(content.len() + 2);
        if has_space_before {
            text.push(' ');
        }
        text.push_str(content);
        if has_space_after {
            text.push(' ');
        }
        MathNode::Text(text)
    }
}

/// Pairs each opening bracket with the next unmatched closing bracket of any
/// kind, so `[0, 1)` is a half-open interval.
fn match_brackets(tokens: &[Token]) -> Vec<Option<usize>> {
    let mut closers = vec![None; tokens.len()];
    let mut open_stack = Vec::new();
    for (index, token) in tokens.iter().enumerate() {
        match token.kind {
            TokenKind::Open => open_stack.push(index),
            TokenKind::Close => {
                if let Some(open) = open_stack.pop() {
                    closers[open] = Some(index);
                }
            }
            _ => {}
        }
    }
    closers
}

/// End (exclusive) of the chunk of items starting at `first`: the first item
/// plus every following item glued to it without whitespace.
fn chunk_end(items: &[Item], first: usize) -> Option<usize> {
    let first_item = items.get(first)?;
    if !matches!(
        first_item.kind,
        ItemKind::Run | ItemKind::Breaker(Role::Relation)
    ) {
        return None;
    }
    let mut end = first + 1;
    while end < items.len()
        && !items[end].has_space_before
        && items[end].kind != ItemKind::MatrixToEnd
    {
        end += 1;
    }
    Some(end)
}

fn push_nodes(pieces: &mut Vec<Piece>, nodes: MathList) {
    match pieces.last_mut() {
        Some(Piece::Nodes(existing)) => existing.extend(nodes),
        _ => pieces.push(Piece::Nodes(nodes)),
    }
}

fn attach_postfix(pieces: &mut Vec<Piece>, symbol: Symbol) {
    let base = match pieces.last_mut() {
        Some(Piece::Nodes(nodes)) => nodes.pop(),
        _ => None,
    };
    let base = base.unwrap_or_else(|| MathNode::Group(Vec::new()));
    push_nodes(
        pieces,
        vec![attach_script(
            base,
            ScriptSlot::Sup,
            vec![MathNode::Atom(symbol.to_atom())],
        )],
    );
}

/// Turns `… over …` into fractions; numerator and denominator extend to the
/// nearest relation or punctuation on either side.
fn resolve_over(pieces: Vec<Piece>) -> MathList {
    if !pieces.iter().any(|piece| matches!(piece, Piece::Over)) {
        return pieces
            .into_iter()
            .flat_map(|piece| match piece {
                Piece::Nodes(nodes) => nodes,
                Piece::Boundary(node) => vec![node],
                Piece::Over => Vec::new(),
            })
            .collect();
    }
    let mut output = Vec::new();
    let mut parts: Vec<MathList> = vec![Vec::new()];
    for piece in pieces {
        match piece {
            Piece::Nodes(nodes) => {
                if let Some(part) = parts.last_mut() {
                    part.extend(nodes);
                }
            }
            Piece::Over => parts.push(Vec::new()),
            Piece::Boundary(node) => {
                output.extend(fold_fractions(std::mem::replace(
                    &mut parts,
                    vec![Vec::new()],
                )));
                output.push(node);
            }
        }
    }
    output.extend(fold_fractions(parts));
    output
}

fn fold_fractions(parts: Vec<MathList>) -> MathList {
    let mut parts = parts.into_iter();
    let mut result = parts.next().unwrap_or_default();
    for denominator in parts {
        let numerator = std::mem::take(&mut result);
        result = vec![MathNode::Fraction {
            numerator: strip_parentheses(numerator),
            denominator: strip_parentheses(denominator),
        }];
    }
    result
}

/// `(a+b) over 2` drops the parentheses just like `(a+b)/2`.
fn strip_parentheses(mut list: MathList) -> MathList {
    let is_single_paren =
        matches!(list.as_slice(), [MathNode::Fenced { open, .. }] if open == "(" || open == "[");
    if !is_single_paren {
        return list;
    }
    match list.remove(0) {
        MathNode::Fenced { body, .. } => body,
        other => vec![other],
    }
}

fn attach_script(node: MathNode, slot: ScriptSlot, script: MathList) -> MathNode {
    let MathNode::Scripts { base, sub, sup } = node else {
        return new_scripts(node, slot, script);
    };
    match (slot, sub, sup) {
        (ScriptSlot::Sub, None, sup) => MathNode::Scripts {
            base,
            sub: Some(script),
            sup,
        },
        (ScriptSlot::Sup, sub, None) => MathNode::Scripts {
            base,
            sub,
            sup: Some(script),
        },
        (_, sub, sup) => new_scripts(MathNode::Scripts { base, sub, sup }, slot, script),
    }
}

fn new_scripts(base: MathNode, slot: ScriptSlot, script: MathList) -> MathNode {
    let base = Box::new(base);
    match slot {
        ScriptSlot::Sub => MathNode::Scripts {
            base,
            sub: Some(script),
            sup: None,
        },
        ScriptSlot::Sup => MathNode::Scripts {
            base,
            sub: None,
            sup: Some(script),
        },
    }
}

/// `((a, b), (c, d))` is a matrix: a parenthesised list of at least two
/// bracketed rows (or one row with several cells).
fn make_fenced(open: String, close: String, body: MathList) -> MathNode {
    if open == "(" && is_matrix_shape(&body) {
        let rows = split_on_commas(body)
            .into_iter()
            .map(|mut row| match row.pop() {
                Some(MathNode::Fenced { body, .. }) => split_on_commas(body),
                _ => Vec::new(),
            })
            .collect();
        return MathNode::Matrix { open, close, rows };
    }
    MathNode::Fenced { open, close, body }
}

fn is_comma(node: &MathNode) -> bool {
    matches!(node, MathNode::Atom(atom) if atom.text == "," && atom.class == AtomClass::Punctuation)
}

fn is_matrix_shape(body: &[MathNode]) -> bool {
    let rows: Vec<&[MathNode]> = body.split(is_comma).collect();
    let mut row_open: Option<&str> = None;
    let mut every_row_has_cells = true;
    for row in &rows {
        let [MathNode::Fenced {
            open,
            close,
            body: cells,
        }] = row
        else {
            return false;
        };
        let is_bracket = open == "(" || open == "[";
        if !is_bracket || close.is_empty() || *row_open.get_or_insert(open) != open.as_str() {
            return false;
        }
        every_row_has_cells &= cells.iter().any(is_comma);
    }
    rows.len() >= 2 || every_row_has_cells
}

fn split_on_commas(list: MathList) -> Vec<MathList> {
    let mut parts = vec![Vec::new()];
    for node in list {
        if is_comma(&node) {
            parts.push(Vec::new());
        } else if let Some(part) = parts.last_mut() {
            part.push(node);
        }
    }
    parts
}

/// Applies `transform` to every atom in `node`, recursively.
fn map_atoms(node: MathNode, transform: &impl Fn(Atom) -> Atom) -> MathNode {
    let map_list = |list: MathList| {
        list.into_iter()
            .map(|n| map_atoms(n, transform))
            .collect::<MathList>()
    };
    match node {
        MathNode::Atom(atom) => MathNode::Atom(transform(atom)),
        MathNode::Fraction {
            numerator,
            denominator,
        } => MathNode::Fraction {
            numerator: map_list(numerator),
            denominator: map_list(denominator),
        },
        MathNode::Scripts { base, sub, sup } => MathNode::Scripts {
            base: Box::new(map_atoms(*base, transform)),
            sub: sub.map(map_list),
            sup: sup.map(map_list),
        },
        MathNode::Root { index, radicand } => MathNode::Root {
            index: index.map(map_list),
            radicand: map_list(radicand),
        },
        MathNode::Fenced { open, close, body } => MathNode::Fenced {
            open,
            close,
            body: map_list(body),
        },
        MathNode::Matrix { open, close, rows } => MathNode::Matrix {
            open,
            close,
            rows: rows
                .into_iter()
                .map(|row| row.into_iter().map(map_list).collect())
                .collect(),
        },
        MathNode::Accent { accent, body } => MathNode::Accent {
            accent,
            body: map_list(body),
        },
        MathNode::Group(list) => MathNode::Group(map_list(list)),
        other @ (MathNode::Text(_) | MathNode::Space(_) | MathNode::Error { .. }) => other,
    }
}

fn letter_atom(c: char) -> Atom {
    Atom {
        text: c.to_string(),
        latex: c.to_string(),
        class: AtomClass::Ordinary,
        style: AtomStyle::Italic,
    }
}

/// Prose words are padded so they read as words between formulas.
fn prose_node(word: &str) -> MathNode {
    MathNode::Text(format!(" {word} "))
}

fn quote_content(text: &str, is_closed: bool) -> &str {
    let without_open = &text[1..];
    match is_closed {
        true => &without_open[..without_open.len() - 1],
        false => without_open,
    }
}

fn escaped_node(text: &str) -> MathNode {
    let c = text[1..].chars().next().unwrap_or('\\');
    match c {
        ',' => MathNode::Space(SpaceWidth::Thin),
        ':' | ' ' => MathNode::Space(SpaceWidth::Medium),
        ';' => MathNode::Space(SpaceWidth::Thick),
        _ => MathNode::Atom(symbols::escaped_atom(c)),
    }
}

fn stray_closer(text: &str) -> Atom {
    let latex = if text == "}" { "\\}" } else { text };
    Atom {
        text: text.to_string(),
        latex: latex.to_string(),
        class: AtomClass::Close,
        style: AtomStyle::Upright,
    }
}

fn error_node(source: &str, message: &str) -> MathNode {
    MathNode::Error {
        source: source.to_string(),
        message: message.to_string(),
    }
}
