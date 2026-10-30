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

    #[test]
    fn test_counter_serialization() {
        let mut buf = String::new();
        Counter::new("test", 10).serialize(&Tags::default(), &mut buf);
        assert_eq!("test|c|10", buf);
    }
}
