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
