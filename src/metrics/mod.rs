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
        /// # use metco_client::{Counter, non_empty};
        /// let counter = Counter::new(non_empty!("test"), 1)
        ///     .with_tag(non_empty!("version"), non_empty!("1.0"));
        /// ```
        pub fn with_tag(
            mut self,
            name: $crate::types::NonEmptyStr<'a>,
            value: $crate::types::NonEmptyStr<'a>,
        ) -> Self {
            self.tags
                .get_or_insert_with($crate::types::Tags::new)
                .insert(name, value);

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
    use crate::non_empty;
    use crate::test_support::{serialize, tags};
    use crate::types::NonEmptyStr;

    #[test]
    fn test_with_tag_overrides_same_name() {
        let counter = Counter::new(non_empty!("test"), 1)
            .with_tag(non_empty!("a"), non_empty!("1"))
            .with_tag(non_empty!("a"), non_empty!("2"));

        assert_eq!("test;a=2|c|1", serialize(&counter));
    }

    #[test]
    fn test_with_tags_merges_with_existing_tags() {
        let counter = Counter::new(non_empty!("test"), 1)
            .with_tag(non_empty!("a"), non_empty!("1"))
            .with_tag(non_empty!("b"), non_empty!("2"))
            .with_tags(tags(&[("b", "3"), ("c", "4")]));

        assert_eq!("test;a=1;b=3;c=4|c|1", serialize(&counter));
    }

    #[test]
    fn test_with_tag_after_with_tags() {
        let counter = Counter::new(non_empty!("test"), 1)
            .with_tags(tags(&[("a", "1"), ("b", "2")]))
            .with_tag(non_empty!("a"), non_empty!("3"));

        assert_eq!("test;a=3;b=2|c|1", serialize(&counter));
    }

    #[test]
    fn test_with_empty_tags() {
        assert_eq!(
            "test|c|1",
            serialize(&Counter::new(non_empty!("test"), 1).with_tags(Tags::new()))
        );
        assert_eq!(
            "test;a=1|c|1",
            serialize(
                &Counter::new(non_empty!("test"), 1)
                    .with_tag(non_empty!("a"), non_empty!("1"))
                    .with_tags(Tags::new())
            )
        );
    }

    #[test]
    fn test_with_tag_accepts_owned_values() {
        let counter = Counter::new(non_empty!("test"), 1).with_tag(
            NonEmptyStr::new(String::from("a")).unwrap(),
            NonEmptyStr::new(String::from("1")).unwrap(),
        );

        assert_eq!("test;a=1|c|1", serialize(&counter));
    }

    #[test]
    fn test_all_metrics_support_tags() {
        assert_eq!(
            "c;t=v|c|1",
            serialize(&Counter::new(non_empty!("c"), 1).with_tag(non_empty!("t"), non_empty!("v")))
        );
        assert_eq!(
            "h;t=v|h|1",
            serialize(
                &Histogram::new(non_empty!("h"), 1).with_tag(non_empty!("t"), non_empty!("v"))
            )
        );
        assert_eq!(
            "g;t=v|g|1",
            serialize(
                &Gauge::new(non_empty!("g"), GaugeOperation::Set(1))
                    .with_tag(non_empty!("t"), non_empty!("v"))
            )
        );
        assert_eq!(
            "t;t=v|t|1|s",
            serialize(
                &Timing::new(non_empty!("t"), 1, TimingResolution::Seconds)
                    .with_tag(non_empty!("t"), non_empty!("v"))
            )
        );
    }
}
