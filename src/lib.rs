#![doc = include_str!("../README.md")]
#![forbid(
    unsafe_code,
    missing_docs,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic
)]
#![warn(clippy::nursery, clippy::pedantic, clippy::cargo, rustdoc::all)]
#![no_std]

extern crate alloc;

use alloc::{string::String, vec::Vec};
use core::fmt::{Display, Error, Write as _};

/// A primitive Lisp value.
pub trait Atom: Display {
    /// The width of the [`Atom`] when displayed with a monospace font.
    fn width(&self) -> usize;

    /// How much to indent the rest of the list when this is used as the head.
    /// Mainly useful for special forms.
    ///
    /// ```lisp
    /// (long-name arg-1
    ///   less-indented-arg2)
    /// ```
    fn custom_indentation(&self) -> Option<usize>;
}

/// The smallest unit of Lisp syntax.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Token<'src, A: Atom> {
    /// An [`Atom`].
    Atom(A),
    /// An opening parenthesis.
    LParen,
    /// A closing parenthesis.
    RParen,
    /// A prefix operator, usually for quoting or unquoting. The width of this
    /// must equal the number of bytes, which is true for all printable ASCII
    /// characters.
    PrefixOperator(&'src str),
    /// An explicit line break.
    NewLine,
    /// A line comment with an implicit line break after it. Embedded line
    /// breaks are not allowed.
    Comment(&'src str),
}

/// Formats a [`Token`] stream, producing a [`String`].
///
/// # Errors
///
/// This function returns an error if it fails to display any of the [`Atom`]s.
pub fn format<'src, A: Atom>(
    tokens: &mut impl Iterator<Item = Token<'src, A>>,
    default_indentation: usize,
) -> Result<String, Error> {
    let mut formatter = Formatter {
        output: String::new(),
        default_indentation,
        preceded_by_expression: false,
        levels: Vec::<usize>::new(),
        is_operator: Vec::new(),
        awaiting_new_level: false,
        x: 0,
    };

    let mut tokens = tokens.peekable();
    while let Some(token) = tokens.next() {
        if matches!(token, Token::LParen) {
            while tokens.peek().is_some_and(|it| matches!(it, Token::NewLine)) {
                tokens.next();
            }
        }
        formatter.token(token)?;
    }

    if formatter.output.ends_with(|c: char| c != '\n') {
        formatter.output.push('\n');
    } else if formatter.output.ends_with("\n\n") {
        formatter.output.pop();
    }

    Ok(formatter.output)
}

struct Formatter {
    output: String,
    default_indentation: usize,
    preceded_by_expression: bool,
    levels: Vec<usize>,
    is_operator: Vec<bool>,
    awaiting_new_level: bool,
    x: usize,
}

impl Formatter {
    fn token<A: Atom>(&mut self, token: Token<A>) -> Result<(), Error> {
        match token {
            Token::Atom(atom) => {
                let width = atom.width();
                if self.awaiting_new_level {
                    self.levels.push(
                        atom.custom_indentation()
                            .map_or(self.x + width + 1, |indentation| {
                                self.x + indentation - 1
                            }),
                    );
                    self.is_operator.push(false);
                } else {
                    self.leading_space();
                }
                self.x += width;
                write!(self.output, "{atom}")?;
                self.preceded_by_expression = true;
                self.awaiting_new_level = false;
                if self.is_operator.last() == Some(&true) {
                    self.levels.pop();
                    self.is_operator.pop();
                }
            }
            Token::LParen => {
                self.put_default_level_or_leading_space();
                self.output.push('(');
                self.x += 1;
                self.preceded_by_expression = false;
                self.awaiting_new_level = true;
            }
            Token::RParen => {
                while self.output.ends_with('\n') {
                    self.output.pop();
                }
                self.output.push(')');
                self.x += 1;
                if !self.awaiting_new_level {
                    if self.is_operator.last() == Some(&true) {
                        self.levels.pop();
                        self.is_operator.pop();
                    }
                    self.levels.pop();
                    self.is_operator.pop();
                }
                self.preceded_by_expression = true;
                self.awaiting_new_level = false;
            }
            Token::PrefixOperator(op) => {
                self.put_default_level_or_leading_space();
                self.output.push_str(op);
                self.x += op.len();
                self.preceded_by_expression = false;
                self.awaiting_new_level = false;
                if let ([.., level], [.., true]) =
                    (&mut *self.levels, &*self.is_operator)
                {
                    *level = self.x;
                } else {
                    self.levels.push(self.x);
                    self.is_operator.push(true);
                }
            }
            Token::NewLine => {
                if !(self.output.is_empty() || self.output.ends_with("\n\n")) {
                    self.output.push('\n');
                    self.preceded_by_expression = false;
                    self.awaiting_new_level = false;
                }
            }
            Token::Comment(comment) => {
                self.put_default_level_or_leading_space();
                if self.awaiting_new_level {
                    self.output.push(' ');
                }
                writeln!(self.output, "{comment}")?;
                self.preceded_by_expression = false;
                self.awaiting_new_level = false;
            }
        }

        Ok(())
    }

    fn leading_space(&mut self) {
        if self.output.ends_with('\n') {
            self.x = self.levels.last().copied().unwrap_or(0);
            self.output.extend(core::iter::repeat(' ').take(self.x));
        } else if self.preceded_by_expression {
            self.output.push(' ');
            self.x += 1;
        }
    }

    fn put_default_level_or_leading_space(&mut self) {
        if self.awaiting_new_level {
            self.levels.push(self.x + self.default_indentation - 1);
            self.is_operator.push(false);
        } else {
            self.leading_space();
        }
    }
}
