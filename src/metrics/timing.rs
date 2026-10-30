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
/// The duration is sent to the client when the timer is dropped or when [`Timer::finish`] is called.
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
/// ```
#[derive(Debug)]
#[must_use = "the duration is only sent when `finish` or `finish_with_tags` is called"]
pub struct Timer<'a, T> {
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
        let client = self.client;
        client.send(self.into_timing());
    }

    /// Finishes the timer and sends the elapsed time to the client with additional tags.
    pub fn finish_with_tags(self, tags: Tags<'a>) {
        let client = self.client;
        client.send(self.into_timing().with_tags(tags));
    }

    fn into_timing(self) -> Timing<'a> {
        let elapsed = u64::try_from(self.start.elapsed().as_nanos()).unwrap_or(u64::MAX);
        let timing = Timing::new(self.name, elapsed, TimingResolution::NanoSeconds);

        match self.tags {
            Some(tags) => timing.with_tags(tags),
            None => timing,
        }
    }

    tags_support!();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ClientBuilder;
    use crate::transport::BlackHoleTransport;

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
}
