//! Splits math source into tokens. Whitespace is not a token; it is recorded
//! on the following token because spacing decides grouping in QuickMath.

use crate::math::symbols::{MULTI_CHAR_OPERATORS, SINGLE_CHAR_OPERATORS};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TokenKind {
    /// Digits with an optional decimal part: `42`, `3.14`.
    Number,
    /// A run of ASCII letters: `x`, `alpha`, `sqrt`.
    Word,
    /// `\` followed by letters, e.g. `\alpha` (accepted for LaTeX habits).
    Command,
    /// `\` followed by one non-letter character, e.g. `\;` or `\,`.
    Escape,
    /// A `\` at the very end of the input.
    LoneBackslash,
    /// An operator spelling from the symbol table: `+`, `<=`, `->`, `,`, …
    Operator,
    /// `(`, `[` or `{`.
    Open,
    /// `)`, `]` or `}`.
    Close,
    Underscore,
    Caret,
    Slash,
    Prime,
    /// `"text"` or `` `text` ``; `is_closed` is false while still typing.
    Quote {
        is_closed: bool,
    },
    /// Any other character, typically Unicode typed directly (`α`, `≤`).
    Other,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Token {
    pub kind: TokenKind,
    pub start: usize,
    pub end: usize,
    pub has_space_before: bool,
}

/// Tokenizes `source`. Never fails; every non-whitespace byte is covered by
/// exactly one token and token boundaries are char boundaries.
pub(crate) fn lex(source: &str) -> Vec<Token> {
    let mut lexer = Lexer {
        source,
        pos: 0,
        tokens: Vec::new(),
    };
    lexer.run();
    lexer.tokens
}

struct Lexer<'a> {
    source: &'a str,
    pos: usize,
    tokens: Vec<Token>,
}

impl Lexer<'_> {
    fn run(&mut self) {
        let mut has_space_before = false;
        while let Some(c) = self.peek_char() {
            if c.is_whitespace() {
                self.pos += c.len_utf8();
                has_space_before = true;
                continue;
            }
            let start = self.pos;
            let kind = self.lex_token(c);
            self.tokens.push(Token {
                kind,
                start,
                end: self.pos,
                has_space_before,
            });
            has_space_before = false;
        }
    }

    fn peek_char(&self) -> Option<char> {
        self.source[self.pos..].chars().next()
    }

    fn rest(&self) -> &str {
        &self.source[self.pos..]
    }

    fn advance_while(&mut self, predicate: impl Fn(char) -> bool) {
        let length = self
            .rest()
            .find(|c: char| !predicate(c))
            .unwrap_or(self.rest().len());
        self.pos += length;
    }

    fn lex_token(&mut self, c: char) -> TokenKind {
        match c {
            '0'..='9' => self.lex_number(),
            'a'..='z' | 'A'..='Z' => {
                self.advance_while(|c| c.is_ascii_alphabetic());
                TokenKind::Word
            }
            '"' | '`' => self.lex_quote(c),
            '\\' => self.lex_backslash(),
            '(' | '[' | '{' => self.single(TokenKind::Open),
            ')' | ']' | '}' => self.single(TokenKind::Close),
            '_' => self.single(TokenKind::Underscore),
            '^' => self.single(TokenKind::Caret),
            '\'' => self.single(TokenKind::Prime),
            _ => self.lex_operator_or_other(c),
        }
    }

    fn single(&mut self, kind: TokenKind) -> TokenKind {
        self.pos += 1;
        kind
    }

    fn lex_number(&mut self) -> TokenKind {
        self.advance_while(|c| c.is_ascii_digit());
        let rest = self.rest().as_bytes();
        if rest.len() >= 2 && rest[0] == b'.' && rest[1].is_ascii_digit() {
            self.pos += 1;
            self.advance_while(|c| c.is_ascii_digit());
        }
        TokenKind::Number
    }

    fn lex_quote(&mut self, quote: char) -> TokenKind {
        self.pos += 1;
        match self.rest().find(quote) {
            Some(offset) => {
                self.pos += offset + 1;
                TokenKind::Quote { is_closed: true }
            }
            None => {
                self.pos = self.source.len();
                TokenKind::Quote { is_closed: false }
            }
        }
    }

    fn lex_backslash(&mut self) -> TokenKind {
        self.pos += 1;
        match self.peek_char() {
            None => TokenKind::LoneBackslash,
            Some(c) if c.is_ascii_alphabetic() => {
                self.advance_while(|c| c.is_ascii_alphabetic());
                TokenKind::Command
            }
            Some(c) => {
                self.pos += c.len_utf8();
                TokenKind::Escape
            }
        }
    }

    fn lex_operator_or_other(&mut self, c: char) -> TokenKind {
        let rest = self.rest();
        if is_negated_in(rest) {
            self.pos += 3;
            return TokenKind::Operator;
        }
        if let Some(spelling) = MULTI_CHAR_OPERATORS
            .iter()
            .find(|spelling| rest.starts_with(**spelling))
        {
            self.pos += spelling.len();
            return TokenKind::Operator;
        }
        if c == '/' {
            return self.single(TokenKind::Slash);
        }
        self.pos += c.len_utf8();
        if SINGLE_CHAR_OPERATORS.contains(c) {
            TokenKind::Operator
        } else {
            TokenKind::Other
        }
    }
}

/// `!in` (∉), but not the start of `!int` or similar words.
fn is_negated_in(rest: &str) -> bool {
    rest.starts_with("!in") && !rest[3..].starts_with(|c: char| c.is_ascii_alphabetic())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kinds(source: &str) -> Vec<(TokenKind, &str)> {
        lex(source)
            .into_iter()
            .map(|t| (t.kind, &source[t.start..t.end]))
            .collect()
    }

    #[test]
    fn splits_words_numbers_and_operators() {
        assert_eq!(
            kinds("x<=3.5"),
            vec![
                (TokenKind::Word, "x"),
                (TokenKind::Operator, "<="),
                (TokenKind::Number, "3.5")
            ]
        );
    }

    #[test]
    fn records_whitespace_on_following_token() {
        let tokens = lex("a  b");
        assert!(!tokens[0].has_space_before);
        assert!(tokens[1].has_space_before);
    }

    #[test]
    fn number_followed_by_period_keeps_period_separate() {
        assert_eq!(
            kinds("1."),
            vec![(TokenKind::Number, "1"), (TokenKind::Operator, ".")]
        );
    }

    #[test]
    fn negated_in_is_one_operator_but_not_inside_words() {
        assert_eq!(kinds("!in"), vec![(TokenKind::Operator, "!in")]);
        assert_eq!(
            kinds("!int"),
            vec![(TokenKind::Operator, "!"), (TokenKind::Word, "int")]
        );
    }

    #[test]
    fn slash_variants_are_distinguished() {
        assert_eq!(kinds("/"), vec![(TokenKind::Slash, "/")]);
        assert_eq!(kinds("//"), vec![(TokenKind::Operator, "//")]);
        assert_eq!(kinds("/="), vec![(TokenKind::Operator, "/=")]);
    }

    #[test]
    fn unclosed_quote_runs_to_end() {
        assert_eq!(
            kinds("\"abc"),
            vec![(TokenKind::Quote { is_closed: false }, "\"abc")]
        );
    }

    #[test]
    fn backslash_forms() {
        assert_eq!(kinds("\\alpha"), vec![(TokenKind::Command, "\\alpha")]);
        assert_eq!(kinds("\\;"), vec![(TokenKind::Escape, "\\;")]);
        assert_eq!(kinds("\\"), vec![(TokenKind::LoneBackslash, "\\")]);
    }

    #[test]
    fn unicode_is_a_single_other_token() {
        assert_eq!(kinds("α"), vec![(TokenKind::Other, "α")]);
    }
}
