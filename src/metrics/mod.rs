use crate::types::Tags;

/// Trait for all metrics that can be sent via the [`Client`](crate::Client).
pub trait Metric {
    /// Serializes the metric into a buffer suitable for MetCo.
    ///
    /// The `tags` parameter contains default tags from the client that should be
    /// included in the serialized output.
    fn serialize(&self, tags: &Tags, buf: &mut String);
}

/// Macro to provide common tag methods for metric structs.
macro_rules! tags_support {
    () => {
        /// Adds multiple tags to the metric.
        pub fn with_tags(mut self, tags: crate::types::Tags<'a>) -> Self {
            self.tags = Some(match self.tags {
                None => tags,
                Some(mut current_tags) => {
                    current_tags.extend(tags);

                    current_tags
                }
            });

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
        pub fn with_tag<K: Into<std::borrow::Cow<'a, str>>, V: Into<std::borrow::Cow<'a, str>>>(
            mut self,
            name: K,
            value: V,
        ) -> Self {
            self.tags = Some(match self.tags {
                None => crate::types::Tags::from([(name.into(), value.into())]),
                Some(mut current_tags) => {
                    current_tags.insert(name.into(), value.into());

                    current_tags
                }
            });

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
