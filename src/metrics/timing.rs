use crate::client::Client;
use crate::metrics::Metric;
use crate::transport::Transport;
use crate::types::{NonEmptyStr, Tags};
use crate::utils::serialize_name_and_tags;
use std::fmt::Write;
use std::time::Instant;

/// Resolution for timing metrics.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TimingResolution {
    /// Seconds resolution.
    Seconds,
    /// Milliseconds resolution.
    MilliSeconds,
    /// Microseconds resolution.
    MicroSeconds,
    /// Nanoseconds resolution.
    NanoSeconds,
}

impl TimingResolution {
    /// Encodes the resolution into its string representation.
    #[must_use]
    pub fn encode(&self) -> &'static str {
        match self {
            Self::Seconds => "s",
            Self::MilliSeconds => "ms",
            Self::MicroSeconds => "us",
            Self::NanoSeconds => "ns",
        }
    }
}

/// A timing metric.
///
/// Timings are used to track the duration of events.
///
/// # Examples
///
/// ```rust
/// # use metco_client::non_empty;
/// use metco_client::{Timing, TimingResolution, Tags};
/// # use metco_client::Metric;
///
/// let timing = Timing::new(non_empty!("query_time"), 150, TimingResolution::MilliSeconds);
///
/// let mut buf = String::new();
/// timing.serialize(&Tags::default(), &mut buf);
/// ```
#[derive(Debug, Clone)]
#[must_use]
pub struct Timing<'a> {
    name: NonEmptyStr<'a>,
    duration: u64,
    resolution: TimingResolution,
    tags: Option<Tags<'a>>,
}

impl<'a> Timing<'a> {
    /// Creates a new timing metric with the given name, duration, and resolution.
    pub fn new(name: NonEmptyStr<'a>, duration: u64, resolution: TimingResolution) -> Self {
        Self {
            name,
            duration,
            resolution,
            tags: None,
        }
    }

    tags_support!();
}

impl Metric for Timing<'_> {
    fn serialize(&self, tags: &Tags<'_>, buf: &mut String) {
        serialize_name_and_tags(buf, &self.name, tags, self.tags.as_ref());

        let _ = write!(buf, "|t|{}|{}", self.duration, self.resolution.encode());
    }
}

/// A timer for measuring durations.
///
/// Created via [`Client::timer`](crate::Client::timer).
/// The duration is sent to the client exactly once, either when [`Timer::finish`] or
/// [`Timer::finish_with_tags`] is called, or when the timer is dropped (including
/// during unwinding).
///
/// # Examples
///
/// ```rust
/// # use metco_client::non_empty;
/// # use metco_client::ClientBuilder;
/// # use metco_client::transport::UdpTransport;
/// # let client = ClientBuilder::default().build(UdpTransport::connect(([127, 0, 0, 1], 3232)).unwrap());
/// let timer = client.timer(non_empty!("calculation"));
/// // ... complex calculation ...
/// timer.finish();
///
/// {
///     let _timer = client.timer(non_empty!("scoped_calculation"));
///     // ... complex calculation ...
/// } // duration is sent here
/// ```
#[derive(Debug)]
#[must_use = "dropping the timer immediately sends a near-zero duration"]
pub struct Timer<'a, T: Transport> {
    client: &'a Client<T>,
    start: Instant,
    name: NonEmptyStr<'a>,
    tags: Option<Tags<'a>>,
}

impl<'a, T: Transport> Timer<'a, T> {
    pub(crate) fn new(client: &'a Client<T>, name: NonEmptyStr<'a>) -> Self {
        Self {
            client,
            start: Instant::now(),
            name,
            tags: None,
        }
    }

    /// Finishes the timer and sends the elapsed time to the client.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use metco_client::non_empty;
    /// # use metco_client::ClientBuilder;
    /// # use metco_client::transport::UdpTransport;
    /// # let client = ClientBuilder::default().build(UdpTransport::connect(([127, 0, 0, 1], 3232)).unwrap());
    /// let timer = client.timer(non_empty!("long_op"));
    /// timer.finish();
    /// ```
    pub fn finish(self) {
        drop(self);
    }

    /// Finishes the timer and sends the elapsed time to the client with additional tags.
    pub fn finish_with_tags(self, tags: Tags<'a>) {
        drop(self.with_tags(tags));
    }

    tags_support!();
}

impl<T: Transport> Drop for Timer<'_, T> {
    fn drop(&mut self) {
        let elapsed = u64::try_from(self.start.elapsed().as_nanos()).unwrap_or(u64::MAX);
        let timing = Timing::new(self.name.borrowed(), elapsed, TimingResolution::NanoSeconds);

        match self.tags.take() {
            Some(tags) => self.client.send(timing.with_tags(tags)),
            None => self.client.send(timing),
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ClientBuilder;
    use crate::non_empty;
    use crate::test_support::{RecordingTransport, serialize, tags};
    use std::panic::{AssertUnwindSafe, catch_unwind};
    use std::time::Duration;

    /// Splits a sent timing into its name part and duration in nanoseconds.
    ///
    /// Splits from the right, so escaped `|` in the name are left intact.
    fn split_timing(metric: &str) -> (&str, u64) {
        let parts: Vec<_> = metric.rsplitn(4, '|').collect();

        assert_eq!(parts.len(), 4, "{metric}");
        assert_eq!(parts[0], "ns");
        assert_eq!(parts[2], "t");

        (parts[3], parts[1].parse().unwrap())
    }

    fn sent_names(transport: &RecordingTransport) -> Vec<String> {
        transport
            .sent()
            .iter()
            .map(|m| split_timing(m).0.to_owned())
            .collect()
    }

    #[test]
    fn test_timing_resolution_encoding() {
        assert_eq!(TimingResolution::Seconds.encode(), "s");
        assert_eq!(TimingResolution::MilliSeconds.encode(), "ms");
        assert_eq!(TimingResolution::MicroSeconds.encode(), "us");
        assert_eq!(TimingResolution::NanoSeconds.encode(), "ns");
    }

    #[test]
    fn test_timing_serialization() {
        assert_eq!(
            "test|t|100|ms",
            serialize(&Timing::new(
                non_empty!("test"),
                100,
                TimingResolution::MilliSeconds
            ))
        );
    }

    #[test]
    fn test_timing_serialization_all_resolutions() {
        let cases = [
            (TimingResolution::Seconds, "test|t|7|s"),
            (TimingResolution::MilliSeconds, "test|t|7|ms"),
            (TimingResolution::MicroSeconds, "test|t|7|us"),
            (TimingResolution::NanoSeconds, "test|t|7|ns"),
        ];

        for (resolution, expected) in cases {
            assert_eq!(
                expected,
                serialize(&Timing::new(non_empty!("test"), 7, resolution))
            );
        }
    }

    #[test]
    fn test_timing_value_bounds() {
        assert_eq!(
            "test|t|0|ns",
            serialize(&Timing::new(
                non_empty!("test"),
                0,
                TimingResolution::NanoSeconds
            ))
        );
        assert_eq!(
            "test|t|18446744073709551615|ns",
            serialize(&Timing::new(
                non_empty!("test"),
                u64::MAX,
                TimingResolution::NanoSeconds
            ))
        );
    }

    #[test]
    fn test_timer_with_tags() {
        let client = ClientBuilder::default().build(RecordingTransport::default());
        let timer = client
            .timer(non_empty!("test"))
            .with_tag(non_empty!("t1"), non_empty!("v1"));

        assert_eq!(timer.name, "test");
        assert_eq!(timer.tags.as_ref().unwrap().get("t1").unwrap(), "v1");
    }

    #[test]
    fn test_timer_is_sent_once_on_finish() {
        let transport = RecordingTransport::default();
        let client = ClientBuilder::default().build(&transport);

        client.timer(non_empty!("plain")).finish();
        client
            .timer(non_empty!("tagged"))
            .with_tag(non_empty!("t1"), non_empty!("v1"))
            .finish_with_tags(tags(&[("t2", "v2")]));

        assert_eq!(sent_names(&transport), ["plain", "tagged;t1=v1;t2=v2"]);
    }

    #[test]
    fn test_timer_is_sent_on_drop() {
        let transport = RecordingTransport::default();
        let client = ClientBuilder::default().build(&transport);

        {
            let _timer = client.timer(non_empty!("scoped"));
            assert!(transport.sent().is_empty());
        }

        assert_eq!(sent_names(&transport), ["scoped"]);
    }

    #[test]
    fn test_timer_is_sent_when_unwinding() {
        let transport = RecordingTransport::default();
        let client = ClientBuilder::default().build(&transport);

        let result = catch_unwind(AssertUnwindSafe(|| {
            let _timer = client.timer(non_empty!("panicking"));
            panic!("expected panic");
        }));

        assert!(result.is_err());
        assert_eq!(sent_names(&transport), ["panicking"]);
    }

    #[test]
    fn test_timer_measures_elapsed_time() {
        let transport = RecordingTransport::default();
        let client = ClientBuilder::default().build(&transport);

        let timer = client.timer(non_empty!("sleep"));
        std::thread::sleep(Duration::from_millis(5));
        timer.finish();

        let sent = transport.sent();
        let (_, nanos) = split_timing(&sent[0]);
        assert!(nanos >= 5_000_000, "{nanos}");
    }

    #[test]
    fn test_finish_with_tags_overrides_timer_tags() {
        let transport = RecordingTransport::default();
        let client = ClientBuilder::default().build(&transport);

        client
            .timer(non_empty!("t"))
            .with_tag(non_empty!("a"), non_empty!("timer"))
            .with_tag(non_empty!("b"), non_empty!("timer"))
            .finish_with_tags(tags(&[("b", "finish")]));

        assert_eq!(sent_names(&transport), ["t;a=timer;b=finish"]);
    }

    #[test]
    fn test_finish_with_tags_without_timer_tags() {
        let transport = RecordingTransport::default();
        let client = ClientBuilder::default().build(&transport);

        client
            .timer(non_empty!("t"))
            .finish_with_tags(tags(&[("a", "1")]));
        client.timer(non_empty!("t")).finish_with_tags(Tags::new());

        assert_eq!(sent_names(&transport), ["t;a=1", "t"]);
    }

    #[test]
    fn test_timer_uses_client_prefix_and_tags() {
        let transport = RecordingTransport::default();
        let client = ClientBuilder::default()
            .with_prefix("app.")
            .with_tag(non_empty!("env"), non_empty!("prod"))
            .with_tag(non_empty!("host"), non_empty!("a"))
            .build(&transport);

        client.timer(non_empty!("t")).finish();
        client
            .timer(non_empty!("t"))
            .with_tag(non_empty!("host"), non_empty!("b"))
            .finish();
        client
            .timer(non_empty!("t"))
            .finish_with_tags(tags(&[("host", "c")]));

        assert_eq!(
            sent_names(&transport),
            [
                "app.t;env=prod;host=a",
                "app.t;env=prod;host=b",
                "app.t;env=prod;host=c"
            ]
        );
    }

    #[test]
    fn test_timer_escapes_name() {
        let transport = RecordingTransport::default();
        let client = ClientBuilder::default().build(&transport);

        client.timer(non_empty!("a|b")).finish();

        assert_eq!(sent_names(&transport), [r"a\|b"]);
    }
}
