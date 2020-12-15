//! A wrapper for the `fmt::Write` objects that efficiently appends indentation after every newline
//!
//! # Setup
//!
//! Add this to your `Cargo.toml`:
//!
//! ```toml
//! [dependencies]
//! indenter = "0.2"
//! ```
//!
//! # Example
//!
//! ```rust
//! use std::error::Error;
//! use core::fmt::{self, Write};
//! use indenter::indented;
//!
//! struct ErrorReporter<'a>(&'a dyn Error);
//!
//! impl fmt::Debug for ErrorReporter<'_> {
//!     fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
//!         let mut source = Some(self.0);
//!         let mut i = 0;
//!
//!         while let Some(error) = source {
//!             writeln!(f)?;
//!             write!(indented(f).ind(i), "{}", error)?;
//!
//!             source = error.source();
//!             i += 1;
//!         }
//!
//!         Ok(())
//!     }
//! }
//! ```
#![no_std]
#![doc(html_root_url = "https://docs.rs/indenter/0.3.0")]
#![warn(
    missing_debug_implementations,
    missing_docs,
    missing_doc_code_examples,
    rust_2018_idioms,
    unreachable_pub,
    bad_style,
    const_err,
    dead_code,
    improper_ctypes,
    non_shorthand_field_patterns,
    no_mangle_generic_items,
    overflowing_literals,
    path_statements,
    patterns_in_fns_without_body,
    private_in_public,
    unconditional_recursion,
    unused,
    unused_allocation,
    unused_comparisons,
    unused_parens,
    while_true
)]
use core::fmt;

/// The set of supported formats for indentation
#[allow(missing_debug_implementations)]
pub enum Format<'a> {
    /// Insert uniform indentation before every line
    ///
    /// This format takes a static string as input and inserts it after every newline
    Uniform {
        /// The string to insert as indentation
        indentation: &'static str,
    },
    /// Inserts a number before the first line
    ///
    /// This format hard codes the indentation level to match the indentation from
    /// `core::backtrace::Backtrace`
    Numbered {
        /// The index to insert before the first line of output
        ind: usize,
    },
    /// A custom indenter which is executed after every newline
    ///
    /// Custom indenters are passed the current line number and the buffer to be written to as args
    Custom {
        /// The custom indenter
        inserter: &'a mut Inserter,
    },
}

/// Helper struct for efficiently indenting multi line display implementations
///
/// # Explanation
///
/// This type will never allocate a string to handle inserting indentation. It instead leverages
/// the `write_str` function that serves as the foundation of the `core::fmt::Write` trait. This
/// lets it intercept each piece of output as its being written to the output buffer. It then
/// splits on newlines giving slices into the original string. Finally we alternate writing these
/// lines and the specified indentation to the output buffer.
#[allow(missing_debug_implementations)]
pub struct Indented<'a, D: ?Sized> {
    inner: &'a mut D,
    started: bool,
    format: Format<'a>,
}

/// A callback for `Format::Custom` used to insert indenation after a new line
///
/// The first argument is the line number within the output, starting from 0
pub type Inserter = dyn FnMut(usize, &mut dyn fmt::Write) -> fmt::Result;

impl Format<'_> {
    fn insert_indentation(&mut self, line: usize, f: &mut dyn fmt::Write) -> fmt::Result {
        match self {
            Format::Uniform { indentation } => write!(f, "{}", indentation),
            Format::Numbered { ind } => {
                if line == 0 {
                    write!(f, "{: >4}: ", ind)
                } else {
                    write!(f, "      ")
                }
            }
            Format::Custom { inserter } => inserter(line, f),
        }
    }
}

impl<'a, D> Indented<'a, D> {
    /// Sets the format to `Format::Numbered` with the provided index
    pub fn ind(self, ind: usize) -> Self {
        self.with_format(Format::Numbered { ind })
    }

    /// Construct an indenter with a user defined format
    pub fn with_format(mut self, format: Format<'a>) -> Self {
        self.format = format;
        self
    }
}

impl<T> fmt::Write for Indented<'_, T>
where
    T: fmt::Write + ?Sized,
{
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for (ind, mut line) in s.split('\n').enumerate() {
            if !self.started {
                // trim first line to ensure it lines up with the number nicely
                line = line.trim_start();
                // Don't render the first line unless its actually got text on it
                if line.is_empty() {
                    continue;
                }

                self.started = true;
                self.format.insert_indentation(ind, &mut self.inner)?;
            } else if ind > 0 {
                self.inner.write_char('\n')?;
                self.format.insert_indentation(ind, &mut self.inner)?;
            }

            self.inner.write_fmt(format_args!("{}", line))?;
        }

        Ok(())
    }
}

/// Helper function for creating a default indenter
pub fn indented<D: ?Sized>(f: &mut D) -> Indented<'_, D> {
    Indented {
        inner: f,
        started: false,
        format: Format::Uniform {
            indentation: "    ",
        },
    }
}

#[cfg(test)]
mod tests {
    extern crate alloc;

    use super::*;
    use alloc::string::String;
    use core::fmt::Write as _;

    #[test]
    fn one_digit() {
        let input = "verify\nthis";
        let expected = "   2: verify\n      this";
        let mut output = String::new();

        indented(&mut output).ind(2).write_str(input).unwrap();

        assert_eq!(expected, output);
    }

    #[test]
    fn two_digits() {
        let input = "verify\nthis";
        let expected = "  12: verify\n      this";
        let mut output = String::new();

        indented(&mut output).ind(12).write_str(input).unwrap();

        assert_eq!(expected, output);
    }

    #[test]
    fn no_digits() {
        let input = "verify\nthis";
        let expected = "    verify\n    this";
        let mut output = String::new();

        indented(&mut output).write_str(input).unwrap();

        assert_eq!(expected, output);
    }

    #[test]
    fn dyn_write() {
        let input = "verify\nthis";
        let expected = "    verify\n    this";
        let mut output = String::new();
        let writer: &mut dyn core::fmt::Write = &mut output;

        indented(writer).write_str(input).unwrap();

        assert_eq!(expected, output);
    }

    #[test]
    fn nice_api() {
        let input = "verify\nthis";
        let expected = "   1: verify\n       this";
        let output = &mut String::new();
        let n = 1;

        write!(
            indented(output).with_format(Format::Custom {
                inserter: &mut move |line_no, f| {
                    if line_no == 0 {
                        write!(f, "{: >4}: ", n)
                    } else {
                        write!(f, "       ")
                    }
                }
            }),
            "{}",
            input
        )
        .unwrap();

        assert_eq!(expected, output);
    }

    #[test]
    fn nice_api_2() {
        let input = "verify\nthis";
        let expected = "  verify\n  this";
        let output = &mut String::new();

        write!(
            indented(output).with_format(Format::Uniform { indentation: "  " }),
            "{}",
            input
        )
        .unwrap();

        assert_eq!(expected, output);
    }
}
