use crate::metrics::Metric;
use crate::types::Tags;
use crate::utils::serialize_name_and_tags;
use std::borrow::Cow;
use std::fmt::Write;

/// A counter metric.
///
/// Counters are used to track occurrences of events. They are monotonic
/// and can only be incremented.
///
/// # Examples
///
/// ```rust
/// use metco_client::{Counter, Tags};
/// # use metco_client::Metric;
///
/// let counter = Counter::new("requests", 1)
///     .with_tag("method", "GET");
///
/// let mut buf = String::new();
/// counter.serialize(&Tags::default(), &mut buf);
/// ```
#[derive(Debug, Clone)]
#[must_use]
pub struct Counter<'a> {
    name: Cow<'a, str>,
    count: u64,
    tags: Option<Tags<'a>>,
}

impl<'a> Counter<'a> {
    /// Creates a new counter with the given name and initial count.
    pub fn new<T: Into<Cow<'a, str>>>(name: T, count: u64) -> Self {
        Self {
            name: name.into(),
            count,
            tags: None,
        }
    }

    tags_support!();
}

impl Metric for Counter<'_> {
    fn serialize(&self, tags: &Tags<'_>, buf: &mut String) {
        serialize_name_and_tags(buf, &self.name, tags, self.tags.as_ref());

        let _ = write!(buf, "|c|{}", self.count);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{serialize, serialize_with, tags};

    #[test]
    fn test_counter_serialization() {
        assert_eq!("test|c|10", serialize(&Counter::new("test", 10)));
    }

    #[test]
    fn test_counter_value_bounds() {
        assert_eq!("test|c|0", serialize(&Counter::new("test", 0)));
        assert_eq!(
            "test|c|18446744073709551615",
            serialize(&Counter::new("test", u64::MAX))
        );
    }

    #[test]
    fn test_counter_accepts_owned_and_borrowed_names() {
        let owned = String::from("owned");

        assert_eq!("owned|c|1", serialize(&Counter::new(owned, 1)));
        assert_eq!("borrowed|c|1", serialize(&Counter::new("borrowed", 1)));
        assert_eq!("cow|c|1", serialize(&Counter::new(Cow::Borrowed("cow"), 1)));
    }

    #[test]
    fn test_counter_with_tags_and_default_tags() {
        let counter = Counter::new("test", 1).with_tag("b", "metric");

        assert_eq!(
            "test;a=default;b=metric|c|1",
            serialize_with(&counter, &tags(&[("a", "default"), ("b", "default")]))
        );
    }

    #[test]
    fn test_counter_escapes_name() {
        assert_eq!(r"a\|b\;c|c|1", serialize(&Counter::new("a|b;c", 1)));
    }

    #[test]
    fn test_counter_clone_is_independent() {
        let original = Counter::new("test", 1).with_tag("a", "1");
        let clone = original.clone().with_tag("b", "2");

        assert_eq!("test;a=1|c|1", serialize(&original));
        assert_eq!("test;a=1;b=2|c|1", serialize(&clone));
    }
}
