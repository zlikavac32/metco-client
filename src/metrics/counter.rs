use crate::metrics::Metric;
use crate::types::{NonEmptyStr, Tags};
use crate::utils::serialize_name_and_tags;
use std::fmt::Write;

/// A counter metric.
///
/// Counters are used to track occurrences of events. They are monotonic
/// and can only be incremented.
///
/// # Examples
///
/// ```rust
/// # use metco_client::non_empty;
/// use metco_client::{Counter, Tags};
/// # use metco_client::Metric;
///
/// let counter = Counter::new(non_empty!("requests"), 1)
///     .with_tag(non_empty!("method"), non_empty!("GET"));
///
/// let mut buf = String::new();
/// counter.serialize(&Tags::default(), &mut buf);
/// ```
#[derive(Debug, Clone)]
#[must_use]
pub struct Counter<'a> {
    name: NonEmptyStr<'a>,
    count: u64,
    tags: Option<Tags<'a>>,
}

impl<'a> Counter<'a> {
    /// Creates a new counter with the given name and initial count.
    pub fn new(name: NonEmptyStr<'a>, count: u64) -> Self {
        Self {
            name,
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
    use crate::non_empty;
    use crate::test_support::{serialize, serialize_with, tags};

    #[test]
    fn test_counter_serialization() {
        assert_eq!(
            "test|c|10",
            serialize(&Counter::new(non_empty!("test"), 10))
        );
    }

    #[test]
    fn test_counter_value_bounds() {
        assert_eq!("test|c|0", serialize(&Counter::new(non_empty!("test"), 0)));
        assert_eq!(
            "test|c|18446744073709551615",
            serialize(&Counter::new(non_empty!("test"), u64::MAX))
        );
    }

    #[test]
    fn test_counter_accepts_owned_and_borrowed_names() {
        let owned = String::from("owned");

        assert_eq!(
            "owned|c|1",
            serialize(&Counter::new(NonEmptyStr::new(owned).unwrap(), 1))
        );
        assert_eq!(
            "borrowed|c|1",
            serialize(&Counter::new(non_empty!("borrowed"), 1))
        );
        assert_eq!(
            "cow|c|1",
            serialize(&Counter::new(
                NonEmptyStr::new(std::borrow::Cow::Borrowed("cow")).unwrap(),
                1
            ))
        );
    }

    #[test]
    fn test_counter_with_tags_and_default_tags() {
        let counter =
            Counter::new(non_empty!("test"), 1).with_tag(non_empty!("b"), non_empty!("metric"));

        assert_eq!(
            "test;a=default;b=metric|c|1",
            serialize_with(&counter, &tags(&[("a", "default"), ("b", "default")]))
        );
    }

    #[test]
    fn test_counter_escapes_name() {
        assert_eq!(
            r"a\|b\;c|c|1",
            serialize(&Counter::new(non_empty!("a|b;c"), 1))
        );
    }

    #[test]
    fn test_counter_clone_is_independent() {
        let original =
            Counter::new(non_empty!("test"), 1).with_tag(non_empty!("a"), non_empty!("1"));
        let clone = original.clone().with_tag(non_empty!("b"), non_empty!("2"));

        assert_eq!("test;a=1|c|1", serialize(&original));
        assert_eq!("test;a=1;b=2|c|1", serialize(&clone));
    }
}
