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
//! use std::fmt::{self, Write};
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
#![doc(html_root_url = "https://docs.rs/indenter/0.2.0")]
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
use std::fmt;

/// The set of supported formats for indentation
#[non_exhaustive]
#[allow(missing_debug_implementations)]
pub enum Format {
    /// Insert uniform indentation before every line
    ///
    /// This format takes a static string as input and inserts that after every newline
    Uniform {
        /// The string to insert as indentation
        indentation: &'static str,
    },
    /// Inserts a number before the first line
    ///
    /// This format hard codes the indentation level to match the indentation from
    /// `std::backtrace::Backtrace`
    Numbered {
        /// The index to insert before the first line of output
        ind: usize,
    },
    /// A custom indenter which is executed after every newline
    ///
    /// Custom indenters are given as input the buffer to be written to and the current line number
    Custom {
        /// The custom indenter
        inserter: Box<Inserter>,
    },
}

/// Helper struct for efficiently indenting multi line display implementations
///
/// # Explanation
///
/// This type will never allocate a string to handle inserting indentation. It instead leverages
/// the `write_fmt` function that serves as the foundation of the `std::fmt::Write` trait. This
/// lets it intercept each piece of output as its being written to the output buffer. It then
/// splits on newlines giving slices into the original string. We then selectively insert
/// indentation into the output buffer when appropriate between writing these split parts of the
/// input string.
#[allow(missing_debug_implementations)]
pub struct Indented<'a, D> {
    inner: &'a mut D,
    started: bool,
    format: Format,
}

/// A callback for `Format::Custom` used to insert indenation after a new line
///
/// The first argument is the line number within the output, starting from 0
pub type Inserter = dyn FnMut(usize, &mut dyn fmt::Write) -> fmt::Result;

impl Format {
    fn insert_indentation(&mut self, line: usize, f: &mut dyn fmt::Write) -> fmt::Result {
        match self {
            Self::Uniform { indentation } => write!(f, "{}", indentation),
            Self::Numbered { ind } => {
                if line == 0 {
                    write!(f, "{: >4}: ", ind)
                } else {
                    write!(f, "       ")
                }
            }
            Self::Custom { inserter } => inserter(line, f),
        }
    }
}

impl<'a, D> Indented<'a, D> {
    /// Sets the format to `Format::Numbered` with the provided index
    pub fn ind(self, ind: usize) -> Self {
        self.with_format(Format::Numbered { ind })
    }

    /// Construct an indenter with a user defined format
    pub fn with_format(mut self, format: Format) -> Self {
        self.format = format;
        self
    }
}

impl<T> fmt::Write for Indented<'_, T>
where
    T: fmt::Write,
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
                self.format.insert_indentation(ind, self.inner)?;
            } else if ind > 0 {
                self.inner.write_char('\n')?;
                self.format.insert_indentation(ind, self.inner)?;
            }

            self.inner.write_fmt(format_args!("{}", line))?;
        }

        Ok(())
    }
}

/// Helper function for creating a default indenter
pub fn indented<D>(f: &mut D) -> Indented<'_, D> {
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
    use super::*;
    use std::fmt::Write as _;

    #[test]
    fn one_digit() {
        let input = "verify\nthis";
        let expected = "   2: verify\n       this";
        let mut output = String::new();

        indented(&mut output).ind(2).write_str(input).unwrap();

        assert_eq!(expected, output);
    }

    #[test]
    fn two_digits() {
        let input = "verify\nthis";
        let expected = "  12: verify\n       this";
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
    fn nice_api() {
        let input = "verify\nthis";
        let expected = "   1: verify\n       this";
        let output = &mut String::new();
        let n = 1;

        write!(
            indented(output).with_format(Format::Custom {
                inserter: Box::new(move |line_no, f| {
                    if line_no == 0 {
                        write!(f, "{: >4}: ", n)
                    } else {
                        write!(f, "       ")
                    }
                })
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
