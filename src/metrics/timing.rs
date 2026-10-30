use crate::client::Client;
use crate::metrics::Metric;
use crate::transport::Transport;
use crate::types::Tags;
use crate::utils::serialize_name_and_tags;
use std::borrow::Cow;
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
/// use metco_client::{Timing, TimingResolution, Tags};
/// # use metco_client::Metric;
///
/// let timing = Timing::new("query_time", 150, TimingResolution::MilliSeconds);
///
/// let mut buf = String::new();
/// timing.serialize(&Tags::default(), &mut buf);
/// ```
#[derive(Debug, Clone)]
#[must_use]
pub struct Timing<'a> {
    name: Cow<'a, str>,
    duration: u64,
    resolution: TimingResolution,
    tags: Option<Tags<'a>>,
}

impl<'a> Timing<'a> {
    /// Creates a new timing metric with the given name, duration, and resolution.
    pub fn new<T: Into<Cow<'a, str>>>(
        name: T,
        duration: u64,
        resolution: TimingResolution,
    ) -> Self {
        Self {
            name: name.into(),
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
/// # use metco_client::ClientBuilder;
/// # use metco_client::transport::UdpTransport;
/// # let client = ClientBuilder::default().build(UdpTransport::connect(([127, 0, 0, 1], 3232)).unwrap());
/// let timer = client.timer("calculation");
/// // ... complex calculation ...
/// timer.finish();
///
/// {
///     let _timer = client.timer("scoped_calculation");
///     // ... complex calculation ...
/// } // duration is sent here
/// ```
#[derive(Debug)]
#[must_use = "dropping the timer immediately sends a near-zero duration"]
pub struct Timer<'a, T: Transport> {
    client: &'a Client<T>,
    start: Instant,
    name: Cow<'a, str>,
    tags: Option<Tags<'a>>,
}

impl<'a, T: Transport> Timer<'a, T> {
    pub(crate) fn new(client: &'a Client<T>, name: Cow<'a, str>) -> Self {
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
    /// # use metco_client::ClientBuilder;
    /// # use metco_client::transport::UdpTransport;
    /// # let client = ClientBuilder::default().build(UdpTransport::connect(([127, 0, 0, 1], 3232)).unwrap());
    /// let timer = client.timer("long_op");
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
        let timing = Timing::new(self.name.as_ref(), elapsed, TimingResolution::NanoSeconds);

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
    use crate::transport::BlackHoleTransport;
    use std::cell::RefCell;

    #[derive(Default)]
    struct RecordingTransport(RefCell<Vec<String>>);

    impl Transport for RecordingTransport {
        fn send(&self, metric: &str) {
            self.0.borrow_mut().push(metric.to_owned());
        }
    }

    fn recorded_names(transport: &RecordingTransport) -> Vec<String> {
        transport
            .0
            .borrow()
            .iter()
            .map(|m| m.split('|').next().unwrap().to_owned())
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
        let mut buf = String::new();
        Timing::new("test", 100, TimingResolution::MilliSeconds)
            .serialize(&Tags::default(), &mut buf);
        assert_eq!("test|t|100|ms", buf);
    }

    #[test]
    fn test_timer_with_tags() {
        let client = ClientBuilder::default().build(BlackHoleTransport);
        let timer = client.timer("test").with_tag("t1", "v1");

        assert_eq!(timer.name, "test");
        assert_eq!(timer.tags.as_ref().unwrap().get("t1").unwrap(), "v1");
    }

    #[test]
    fn test_timer_is_sent_once_on_finish() {
        let transport = RecordingTransport::default();
        let client = ClientBuilder::default().build(&transport);

        client.timer("plain").finish();
        client
            .timer("tagged")
            .with_tag("t1", "v1")
            .finish_with_tags(Tags::from([("t2".into(), "v2".into())]));

        let names = recorded_names(&transport);
        assert_eq!(names.len(), 2);
        assert_eq!(names[0], "plain");
        assert!(names[1] == "tagged;t1=v1;t2=v2" || names[1] == "tagged;t2=v2;t1=v1");
    }

    #[test]
    fn test_timer_is_sent_on_drop() {
        let transport = RecordingTransport::default();
        let client = ClientBuilder::default().build(&transport);

        {
            let _timer = client.timer("scoped");
        }

        assert_eq!(recorded_names(&transport), ["scoped"]);
    }
}
