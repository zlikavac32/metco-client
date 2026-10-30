use crate::metrics::Metric;
use crate::types::Tags;
use crate::utils::serialize_name_and_tags;
use std::borrow::Cow;
use std::fmt::Write;

/// A histogram metric.
///
/// Histograms are used to track the distribution of values.
///
/// # Examples
///
/// ```rust
/// use metco_client::{Histogram, Tags};
/// # use metco_client::Metric;
///
/// let hist = Histogram::new("response_size", 1024);
///
/// let mut buf = String::new();
/// hist.serialize(&Tags::default(), &mut buf);
/// ```
#[derive(Debug, Clone)]
#[must_use]
pub struct Histogram<'a> {
    name: Cow<'a, str>,
    count: u64,
    tags: Option<Tags<'a>>,
}

impl<'a> Histogram<'a> {
    /// Creates a new histogram with the given name and initial count.
    pub fn new<T: Into<Cow<'a, str>>>(name: T, count: u64) -> Self {
        Self {
            name: name.into(),
            count,
            tags: None,
        }
    }

    tags_support!();
}

impl Metric for Histogram<'_> {
    fn serialize(&self, tags: &Tags<'_>, buf: &mut String) {
        serialize_name_and_tags(buf, &self.name, tags, self.tags.as_ref());

        let _ = write!(buf, "|h|{}", self.count);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{serialize, serialize_with, tags};

    #[test]
    fn test_histogram_serialization() {
        assert_eq!("test|h|100", serialize(&Histogram::new("test", 100)));
    }

    #[test]
    fn test_histogram_value_bounds() {
        assert_eq!("test|h|0", serialize(&Histogram::new("test", 0)));
        assert_eq!(
            "test|h|18446744073709551615",
            serialize(&Histogram::new("test", u64::MAX))
        );
    }

    #[test]
    fn test_histogram_with_tags_and_default_tags() {
        let histogram = Histogram::new("test", 1).with_tag("b", "metric");

        assert_eq!(
            "test;a=default;b=metric|h|1",
            serialize_with(&histogram, &tags(&[("a", "default"), ("b", "default")]))
        );
    }

    #[test]
    fn test_histogram_escapes_name() {
        assert_eq!(r"a\|b|h|1", serialize(&Histogram::new("a|b", 1)));
    }
}
