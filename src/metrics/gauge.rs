use crate::client::Client;
use crate::metrics::Metric;
use crate::types::Tags;
use crate::utils::serialize_name_and_tags;
use std::borrow::Cow;

/// Represents an operation to be performed on a gauge.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GaugeOperation {
    /// Sets the gauge to an absolute value.
    Set(i64),
    /// Removes the gauge.
    Remove,
    /// Increments the gauge by a relative value.
    Increment(i64),
    /// Decrements the gauge by a relative value.
    Decrement(i64),
}

/// A gauge metric.
///
/// Gauges are used to track a value that can go up and down.
///
/// # Examples
///
/// ```rust
/// use metco_client::{Gauge, GaugeOperation, Tags};
/// # use metco_client::Metric;
///
/// let gauge = Gauge::new("active_sessions", GaugeOperation::Set(100));
///
/// let mut buf = String::new();
/// gauge.serialize(&Tags::default(), &mut buf);
/// ```
pub struct Gauge<'a> {
    name: Cow<'a, str>,
    operation: GaugeOperation,
    tags: Option<Tags<'a>>,
}

impl<'a> Gauge<'a> {
    /// Creates a new gauge with the given name and operation.
    pub fn new<T: Into<Cow<'a, str>>>(name: T, operation: GaugeOperation) -> Self {
        Self {
            name: name.into(),
            operation,
            tags: None,
        }
    }

    tags_support!();
}

impl<'a> Metric for Gauge<'a> {
    fn serialize(&self, tags: &Tags, buf: &mut String) {
        serialize_name_and_tags(buf, &self.name, tags, self.tags.as_ref());

        buf.push_str("|g|");

        match self.operation {
            GaugeOperation::Set(value) => buf.push_str(&value.to_string()),
            GaugeOperation::Remove => buf.push('x'),
            GaugeOperation::Increment(value) => {
                buf.push_str("+=");
                buf.push_str(&value.to_string());
            }
            GaugeOperation::Decrement(value) => {
                buf.push_str("-=");
                buf.push_str(&value.to_string());
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn serialize_to_string(gauge: Gauge, tags: &Tags) -> String {
        let mut buf = String::new();
        gauge.serialize(tags, &mut buf);
        buf
    }

    #[test]
    fn test_gauge_serialization() {
        let tags = Tags::default();

        assert_eq!(
            "test|g|123",
            serialize_to_string(Gauge::new("test", GaugeOperation::Set(123)), &tags)
        );
        assert_eq!(
            "test|g|-123",
            serialize_to_string(Gauge::new("test", GaugeOperation::Set(-123)), &tags)
        );
        assert_eq!(
            "test|g|x",
            serialize_to_string(Gauge::new("test", GaugeOperation::Remove), &tags)
        );
        assert_eq!(
            "test|g|+=10",
            serialize_to_string(Gauge::new("test", GaugeOperation::Increment(10)), &tags)
        );
        assert_eq!(
            "test|g|-=10",
            serialize_to_string(Gauge::new("test", GaugeOperation::Decrement(10)), &tags)
        );
    }

    #[test]
    fn test_gauge_handle_name() {
        use crate::ClientBuilder;
        let client = ClientBuilder::default()
            .connect(([127, 0, 0, 1], 0))
            .unwrap();
        let handle = client.gauge("test_gauge");
        assert_eq!(handle.name, "test_gauge");
    }
}

/// A handle for performing operations on a gauge.
///
/// Created via [`Client::gauge`](crate::Client::gauge).
pub struct GaugeHandle<'a> {
    pub(crate) client: &'a Client,
    pub(crate) name: Cow<'a, str>,
}

impl<'a> GaugeHandle<'a> {
    /// Sets the gauge to the given value.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use metco_client::ClientBuilder;
    /// # let client = ClientBuilder::default().connect(([127, 0, 0, 1], 3232)).unwrap();
    /// let gauge = client.gauge("temperature");
    /// gauge.set(25);
    /// ```
    pub fn set(&self, value: i64) {
        self.client
            .send(Gauge::new(self.name.clone(), GaugeOperation::Set(value)));
    }

    /// Sets the gauge to the given value with additional tags.
    pub fn set_with_tags(&self, value: i64, tags: Tags<'a>) {
        self.client
            .send(Gauge::new(self.name.clone(), GaugeOperation::Set(value)).with_tags(tags));
    }

    /// Removes the gauge.
    pub fn remove(&self) {
        self.client
            .send(Gauge::new(self.name.clone(), GaugeOperation::Remove));
    }

    /// Removes the gauge with additional tags.
    pub fn remove_with_tags(&self, tags: Tags<'a>) {
        self.client
            .send(Gauge::new(self.name.clone(), GaugeOperation::Remove).with_tags(tags));
    }

    /// Increments the gauge by the given value.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use metco_client::ClientBuilder;
    /// # let client = ClientBuilder::default().connect(([127, 0, 0, 1], 3232)).unwrap();
    /// let gauge = client.gauge("tasks");
    /// gauge.increment(2);
    /// ```
    pub fn increment(&self, value: i64) {
        self.client.send(Gauge::new(
            self.name.clone(),
            GaugeOperation::Increment(value),
        ));
    }

    /// Increments the gauge by the given value with additional tags.
    pub fn increment_with_tags(&self, value: i64, tags: Tags<'a>) {
        self.client
            .send(Gauge::new(self.name.clone(), GaugeOperation::Increment(value)).with_tags(tags));
    }

    /// Decrements the gauge by the given value.
    pub fn decrement(&self, value: i64) {
        self.client.send(Gauge::new(
            self.name.clone(),
            GaugeOperation::Decrement(value),
        ));
    }

    /// Decrements the gauge by the given value with additional tags.
    pub fn decrement_with_tags(&self, value: i64, tags: Tags<'a>) {
        self.client
            .send(Gauge::new(self.name.clone(), GaugeOperation::Decrement(value)).with_tags(tags));
    }
}
