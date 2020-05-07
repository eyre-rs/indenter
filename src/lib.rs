//! A wrapper for the `fmt::Write` objects that efficiently appends indentation after every newline
#![doc(html_root_url = "https://docs.rs/indenter/0.1.4")]
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

/// Helper struct for efficiently numbering and correctly indenting multi line display
/// implementations
#[allow(missing_debug_implementations)]
pub struct Indented<'a, D> {
    inner: &'a mut D,
    started: bool,
    format: Format,
}

/// A callback used to insert indenation after a new line
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
    /// Wrap a formatter number the first line and indent all lines of input before forwarding the
    /// output to the inner formatter
    pub fn numbered(inner: &'a mut D, ind: usize) -> Self {
        Self {
            inner,
            started: false,
            format: Format::Numbered { ind },
        }
    }

    /// Construct an indenter which defaults to `Format::Uniform` with 4 spaces as the indenation
    /// string
    pub fn new(inner: &'a mut D) -> Self {
        Self {
            inner,
            started: false,
            format: Format::Uniform {
                indentation: "    ",
            },
        }
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
    Indented::new(f)
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

        Indented::numbered(&mut output, 2).write_str(input).unwrap();

        assert_eq!(expected, output);
    }

    #[test]
    fn two_digits() {
        let input = "verify\nthis";
        let expected = "  12: verify\n       this";
        let mut output = String::new();

        Indented::numbered(&mut output, 12)
            .write_str(input)
            .unwrap();

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
