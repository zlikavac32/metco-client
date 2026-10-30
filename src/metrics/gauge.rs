use crate::client::Client;
use crate::metrics::Metric;
use crate::transport::Transport;
use crate::types::Tags;
use crate::utils::serialize_name_and_tags;
use std::borrow::Cow;
use std::fmt::Write;

/// Represents an operation to be performed on a gauge.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GaugeOperation {
    /// Sets the gauge to an absolute value.
    Set(i64),
    /// Removes the gauge.
    Remove,
    /// Increments the gauge by a relative value.
    ///
    /// A negative value is sent as a decrement.
    Increment(i64),
    /// Decrements the gauge by a relative value.
    ///
    /// A negative value is sent as an increment.
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
#[derive(Debug, Clone)]
#[must_use]
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

impl Metric for Gauge<'_> {
    fn serialize(&self, tags: &Tags<'_>, buf: &mut String) {
        serialize_name_and_tags(buf, &self.name, tags, self.tags.as_ref());

        buf.push_str("|g|");

        let _ = match self.operation {
            GaugeOperation::Set(value) => write!(buf, "{value}"),
            GaugeOperation::Remove => buf.write_char('x'),
            GaugeOperation::Increment(delta) => write_delta(buf, delta < 0, delta.unsigned_abs()),
            GaugeOperation::Decrement(delta) => write_delta(buf, delta >= 0, delta.unsigned_abs()),
        };
    }
}

/// Writes a relative change as `+=N` or `-=N`, keeping `N` unsigned.
fn write_delta(buf: &mut String, negative: bool, magnitude: u64) -> std::fmt::Result {
    let sign = if negative { '-' } else { '+' };
    write!(buf, "{sign}={magnitude}")
}

/// A handle for performing operations on a gauge.
///
/// Created via [`Client::gauge`](crate::Client::gauge).
#[derive(Debug)]
pub struct GaugeHandle<'a, T> {
    client: &'a Client<T>,
    name: Cow<'a, str>,
}

impl<'a, T: Transport> GaugeHandle<'a, T> {
    pub(crate) fn new(client: &'a Client<T>, name: Cow<'a, str>) -> Self {
        Self { client, name }
    }

    fn apply(&self, operation: GaugeOperation, tags: Option<Tags<'_>>) {
        let gauge = Gauge::new(self.name.as_ref(), operation);

        match tags {
            Some(tags) => self.client.send(gauge.with_tags(tags)),
            None => self.client.send(gauge),
        };
    }

    /// Sets the gauge to the given value.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use metco_client::ClientBuilder;
    /// # use metco_client::transport::UdpTransport;
    /// # let client = ClientBuilder::default().build(UdpTransport::connect(([127, 0, 0, 1], 3232)).unwrap());
    /// let gauge = client.gauge("temperature");
    /// gauge.set(25);
    /// ```
    pub fn set(&self, value: i64) {
        self.apply(GaugeOperation::Set(value), None);
    }

    /// Sets the gauge to the given value with additional tags.
    pub fn set_with_tags(&self, value: i64, tags: Tags<'_>) {
        self.apply(GaugeOperation::Set(value), Some(tags));
    }

    /// Removes the gauge.
    pub fn remove(&self) {
        self.apply(GaugeOperation::Remove, None);
    }

    /// Removes the gauge with additional tags.
    pub fn remove_with_tags(&self, tags: Tags<'_>) {
        self.apply(GaugeOperation::Remove, Some(tags));
    }

    /// Increments the gauge by the given value.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use metco_client::ClientBuilder;
    /// # use metco_client::transport::UdpTransport;
    /// # let client = ClientBuilder::default().build(UdpTransport::connect(([127, 0, 0, 1], 3232)).unwrap());
    /// let gauge = client.gauge("tasks");
    /// gauge.increment(2);
    /// ```
    pub fn increment(&self, value: i64) {
        self.apply(GaugeOperation::Increment(value), None);
    }

    /// Increments the gauge by the given value with additional tags.
    pub fn increment_with_tags(&self, value: i64, tags: Tags<'_>) {
        self.apply(GaugeOperation::Increment(value), Some(tags));
    }

    /// Decrements the gauge by the given value.
    pub fn decrement(&self, value: i64) {
        self.apply(GaugeOperation::Decrement(value), None);
    }

    /// Decrements the gauge by the given value with additional tags.
    pub fn decrement_with_tags(&self, value: i64, tags: Tags<'_>) {
        self.apply(GaugeOperation::Decrement(value), Some(tags));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ClientBuilder;
    use crate::test_support::{RecordingTransport, serialize, tags};

    #[test]
    fn test_gauge_serialization() {
        let cases = [
            (GaugeOperation::Set(123), "test|g|123"),
            (GaugeOperation::Set(-123), "test|g|-123"),
            (GaugeOperation::Set(0), "test|g|0"),
            (GaugeOperation::Set(i64::MAX), "test|g|9223372036854775807"),
            (GaugeOperation::Set(i64::MIN), "test|g|-9223372036854775808"),
            (GaugeOperation::Remove, "test|g|x"),
            (GaugeOperation::Increment(10), "test|g|+=10"),
            (GaugeOperation::Decrement(10), "test|g|-=10"),
            (GaugeOperation::Increment(0), "test|g|+=0"),
            (GaugeOperation::Decrement(0), "test|g|-=0"),
            (GaugeOperation::Increment(-10), "test|g|-=10"),
            (GaugeOperation::Decrement(-10), "test|g|+=10"),
            (
                GaugeOperation::Increment(i64::MAX),
                "test|g|+=9223372036854775807",
            ),
            (
                GaugeOperation::Decrement(i64::MAX),
                "test|g|-=9223372036854775807",
            ),
            (
                GaugeOperation::Increment(i64::MIN),
                "test|g|-=9223372036854775808",
            ),
            (
                GaugeOperation::Decrement(i64::MIN),
                "test|g|+=9223372036854775808",
            ),
        ];

        for (operation, expected) in cases {
            assert_eq!(
                expected,
                serialize(&Gauge::new("test", operation)),
                "{operation:?}"
            );
        }
    }

    #[test]
    fn test_gauge_handle_name() {
        let client = ClientBuilder::default().build(RecordingTransport::default());
        let handle = client.gauge("test_gauge");
        assert_eq!(handle.name, "test_gauge");
    }

    #[test]
    fn test_gauge_handle_operations() {
        let transport = RecordingTransport::default();
        let client = ClientBuilder::default().build(&transport);
        let gauge = client.gauge("g");

        gauge.set(5);
        gauge.increment(2);
        gauge.decrement(1);
        gauge.increment(-3);
        gauge.remove();

        assert_eq!(
            transport.sent(),
            ["g|g|5", "g|g|+=2", "g|g|-=1", "g|g|-=3", "g|g|x"]
        );
    }

    #[test]
    fn test_gauge_handle_operations_with_tags() {
        let transport = RecordingTransport::default();
        let client = ClientBuilder::default().build(&transport);
        let gauge = client.gauge("g");

        gauge.set_with_tags(5, tags(&[("op", "set")]));
        gauge.increment_with_tags(2, tags(&[("op", "inc")]));
        gauge.decrement_with_tags(1, tags(&[("op", "dec")]));
        gauge.remove_with_tags(tags(&[("op", "rm")]));

        assert_eq!(
            transport.sent(),
            [
                "g;op=set|g|5",
                "g;op=inc|g|+=2",
                "g;op=dec|g|-=1",
                "g;op=rm|g|x"
            ]
        );
    }

    #[test]
    fn test_gauge_handle_uses_client_prefix_and_tags() {
        let transport = RecordingTransport::default();
        let client = ClientBuilder::default()
            .with_prefix("app.")
            .with_tag("env", "prod")
            .with_tag("host", "a")
            .build(&transport);
        let gauge = client.gauge("g");

        gauge.set(1);
        gauge.set_with_tags(2, tags(&[("host", "b")]));

        assert_eq!(
            transport.sent(),
            ["app.g;env=prod;host=a|g|1", "app.g;env=prod;host=b|g|2"]
        );
    }

    #[test]
    fn test_gauge_handle_is_reusable_with_owned_name() {
        let transport = RecordingTransport::default();
        let client = ClientBuilder::default().build(&transport);
        let gauge = client.gauge(format!("g{}", 1));

        gauge.set(1);
        gauge.set(2);

        assert_eq!(transport.sent(), ["g1|g|1", "g1|g|2"]);
    }

    #[test]
    fn test_gauge_handle_tags_can_be_short_lived() {
        let transport = RecordingTransport::default();
        let client = ClientBuilder::default().build(&transport);
        let gauge = client.gauge("g");

        for i in 0..2 {
            let value = i.to_string();
            gauge.set_with_tags(i, tags(&[("i", &value)]));
        }

        assert_eq!(transport.sent(), ["g;i=0|g|0", "g;i=1|g|1"]);
    }
}
