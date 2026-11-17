use std::fmt;
use std::time::Duration;

const BYTES_PER_KILOBYTE: usize = 1024;
const MILLIS_PER_SECOND: u128 = 1000;

pub struct ByteSize(pub usize);

pub struct ElapsedText(pub Duration);

impl fmt::Display for ByteSize {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let bytes = self.0;
        if bytes < BYTES_PER_KILOBYTE {
            write!(formatter, "{bytes} B")
        } else if bytes < crate::response::BYTES_PER_MEGABYTE {
            write!(
                formatter,
                "{:.1} KB",
                bytes as f64 / BYTES_PER_KILOBYTE as f64
            )
        } else {
            write!(
                formatter,
                "{:.2} MB",
                bytes as f64 / crate::response::BYTES_PER_MEGABYTE as f64
            )
        }
    }
}

impl fmt::Display for ElapsedText {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let millis = self.0.as_millis();
        if millis < MILLIS_PER_SECOND {
            write!(formatter, "{millis} ms")
        } else {
            write!(formatter, "{:.2} s", self.0.as_secs_f64())
        }
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::{ByteSize, ElapsedText};

    #[test]
    fn sizes_use_the_largest_fitting_unit() {
        assert_eq!(ByteSize(0).to_string(), "0 B");
        assert_eq!(ByteSize(1023).to_string(), "1023 B");
        assert_eq!(ByteSize(1536).to_string(), "1.5 KB");
        assert_eq!(ByteSize(5 * 1024 * 1024).to_string(), "5.00 MB");
    }

    #[test]
    fn durations_switch_from_milliseconds_to_seconds() {
        assert_eq!(ElapsedText(Duration::from_millis(12)).to_string(), "12 ms");
        assert_eq!(
            ElapsedText(Duration::from_millis(999)).to_string(),
            "999 ms"
        );
        assert_eq!(
            ElapsedText(Duration::from_millis(1250)).to_string(),
            "1.25 s"
        );
    }
}
