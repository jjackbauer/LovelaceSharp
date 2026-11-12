//! Buffer-target formatting for numeric values: the reference's TryFormat contract.
//!
//! The reference's Integer implements ISpanFormattable.TryFormat(Span<char>, out int,
//! ReadOnlySpan<char>, IFormatProvider?): it writes the canonical decimal representation into a
//! caller-supplied char buffer, reports the count written and returns true, or writes nothing,
//! reports zero and returns false when the buffer is too small. Rust has no out parameters, so
//! the same contract is the [TryFormat::try_format] method returning Some(count) on success and
//! None on the false branch; the buffer is untouched in the None case.

use lovelace_int::Int;

/// The reference's ISpanFormattable.TryFormat contract.
pub trait TryFormat {
    /// Writes the canonical representation into destination.
    ///
    /// Returns the number of chars written on success, or None (and writes nothing) when the
    /// destination cannot hold the whole representation.
    fn try_format(&self, destination: &mut [char]) -> Option<usize>;
}

impl TryFormat for Int {
    fn try_format(&self, destination: &mut [char]) -> Option<usize> {
        let text = self.to_decimal();
        if destination.len() < text.len() {
            return None;
        }
        for (slot, ch) in destination.iter_mut().zip(text.chars()) {
            *slot = ch;
        }
        Some(text.len())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_sufficient_buffer_receives_the_decimal_representation() {
        let value = Int::from_i64(-42);
        let mut destination = ['\0'; 10];
        let written = value
            .try_format(&mut destination)
            .expect("the buffer holds -42");
        assert_eq!(written, 3);
        assert_eq!(destination[..written].iter().collect::<String>(), "-42");
    }

    #[test]
    fn an_insufficient_buffer_reports_nothing_written() {
        let value = Int::from_i64(-12345);
        let mut destination = ['\0'; 3];
        assert_eq!(value.try_format(&mut destination), None);
        assert!(destination.iter().all(|ch| *ch == '\0'));
    }
}
