use crate::types::Tags;

/// Trait for all metrics that can be sent via the [`Client`](crate::Client).
pub trait Metric {
    /// Serializes the metric into a buffer suitable for MetCo.
    ///
    /// The `tags` parameter contains default tags from the client that should be
    /// included in the serialized output.
    fn serialize(&self, tags: &Tags<'_>, buf: &mut String);
}

/// Macro to provide common tag methods for metric structs.
///
/// Expects the struct to have a lifetime named `'a` and a
/// `tags: Option<Tags<'a>>` field.
macro_rules! tags_support {
    () => {
        /// Adds multiple tags to the metric.
        pub fn with_tags(mut self, tags: $crate::types::Tags<'a>) -> Self {
            match &mut self.tags {
                Some(current_tags) => current_tags.extend(tags),
                None => self.tags = Some(tags),
            }

            self
        }

        /// Adds a single tag to the metric.
        ///
        /// # Examples
        ///
        /// ```rust
        /// # use metco_client::Counter;
        /// let counter = Counter::new("test", 1)
        ///     .with_tag("version", "1.0");
        /// ```
        pub fn with_tag<K, V>(mut self, name: K, value: V) -> Self
        where
            K: Into<std::borrow::Cow<'a, str>>,
            V: Into<std::borrow::Cow<'a, str>>,
        {
            self.tags
                .get_or_insert_with($crate::types::Tags::new)
                .insert(name.into(), value.into());

            self
        }
    };
}

mod counter;
mod gauge;
mod histogram;
mod timing;

pub use counter::Counter;
pub use gauge::{Gauge, GaugeHandle, GaugeOperation};
pub use histogram::Histogram;
pub use timing::{Timer, Timing, TimingResolution};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{serialize, tags};

    #[test]
    fn test_with_tag_overrides_same_name() {
        let counter = Counter::new("test", 1)
            .with_tag("a", "1")
            .with_tag("a", "2");

        assert_eq!("test;a=2|c|1", serialize(&counter));
    }

    #[test]
    fn test_with_tags_merges_with_existing_tags() {
        let counter = Counter::new("test", 1)
            .with_tag("a", "1")
            .with_tag("b", "2")
            .with_tags(tags(&[("b", "3"), ("c", "4")]));

        assert_eq!("test;a=1;b=3;c=4|c|1", serialize(&counter));
    }

    #[test]
    fn test_with_tag_after_with_tags() {
        let counter = Counter::new("test", 1)
            .with_tags(tags(&[("a", "1"), ("b", "2")]))
            .with_tag("a", "3");

        assert_eq!("test;a=3;b=2|c|1", serialize(&counter));
    }

    #[test]
    fn test_with_empty_tags() {
        assert_eq!(
            "test|c|1",
            serialize(&Counter::new("test", 1).with_tags(Tags::new()))
        );
        assert_eq!(
            "test;a=1|c|1",
            serialize(
                &Counter::new("test", 1)
                    .with_tag("a", "1")
                    .with_tags(Tags::new())
            )
        );
    }

    #[test]
    fn test_with_tag_accepts_owned_values() {
        let counter = Counter::new("test", 1).with_tag(String::from("a"), String::from("1"));

        assert_eq!("test;a=1|c|1", serialize(&counter));
    }

    #[test]
    fn test_all_metrics_support_tags() {
        assert_eq!(
            "c;t=v|c|1",
            serialize(&Counter::new("c", 1).with_tag("t", "v"))
        );
        assert_eq!(
            "h;t=v|h|1",
            serialize(&Histogram::new("h", 1).with_tag("t", "v"))
        );
        assert_eq!(
            "g;t=v|g|1",
            serialize(&Gauge::new("g", GaugeOperation::Set(1)).with_tag("t", "v"))
        );
        assert_eq!(
            "t;t=v|t|1|s",
            serialize(&Timing::new("t", 1, TimingResolution::Seconds).with_tag("t", "v"))
        );
    }
}
