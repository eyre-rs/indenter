use std::fmt;

/// The set of supported formats for indentation
#[non_exhaustive]
pub enum Format {
    Uniform { indentation: &'static str },
    Numbered { ind: usize },
    Custom { inserter: Box<Inserter> },
}

/// Helper struct for efficiently numbering and correctly indenting multi line display
/// implementations
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

    pub fn new(inner: &'a mut D) -> Self {
        Self {
            inner,
            started: false,
            format: Format::Uniform {
                indentation: "    ",
            },
        }
    }

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

pub fn indented<'a, D>(f: &'a mut D) -> Indented<'a, D> {
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
