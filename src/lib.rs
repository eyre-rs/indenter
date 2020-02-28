use std::fmt;

/// Helper struct for efficiently numbering and correctly indenting multi line display
/// implementations
pub struct Indented<'a, D> {
    inner: &'a mut D,
    ind: Option<usize>,
    started: bool,
}

impl<'a, D> Indented<'a, D> {
    /// Wrap a formatter number the first line and indent all lines of input before forwarding the
    /// output to the inner formatter
    pub fn numbered(inner: &'a mut D, ind: usize) -> Self {
        Self {
            inner,
            ind: Some(ind),
            started: false,
        }
    }

    pub fn new(inner: &'a mut D) -> Self {
        Self {
            inner,
            ind: None,
            started: false,
        }
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
                match self.ind {
                    Some(ind) => self.inner.write_fmt(format_args!("{: >5}: ", ind))?,
                    None => self.inner.write_fmt(format_args!("    "))?,
                }
            } else if ind > 0 {
                self.inner.write_char('\n')?;
                if self.ind.is_some() {
                    self.inner.write_fmt(format_args!("       "))?;
                } else {
                    self.inner.write_fmt(format_args!("    "))?;
                }
            }

            self.inner.write_fmt(format_args!("{}", line))?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fmt::Write as _;

    #[test]
    fn one_digit() {
        let input = "verify\nthis";
        let expected = "    2: verify\n       this";
        let mut output = String::new();

        Indented {
            inner: &mut output,
            ind: Some(2),
            started: false,
        }
        .write_str(input)
        .unwrap();

        assert_eq!(expected, output);
    }

    #[test]
    fn two_digits() {
        let input = "verify\nthis";
        let expected = "   12: verify\n       this";
        let mut output = String::new();

        Indented {
            inner: &mut output,
            ind: Some(12),
            started: false,
        }
        .write_str(input)
        .unwrap();

        assert_eq!(expected, output);
    }

    #[test]
    fn no_digits() {
        let input = "verify\nthis";
        let expected = "    verify\n    this";
        let mut output = String::new();

        Indented {
            inner: &mut output,
            ind: None,
            started: false,
        }
        .write_str(input)
        .unwrap();

        assert_eq!(expected, output);
    }
}
