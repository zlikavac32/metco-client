use crate::client::Client;
use crate::metrics::Metric;
use crate::types::Tags;
use crate::utils::serialize_name_and_tags;
use std::borrow::Cow;
use std::time::Instant;

/// Resolution for timing metrics.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
    pub fn encode(&self) -> &'static str {
        match self {
            TimingResolution::Seconds => "s",
            TimingResolution::MilliSeconds => "ms",
            TimingResolution::MicroSeconds => "us",
            TimingResolution::NanoSeconds => "ns",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ClientBuilder;

    #[test]
    fn test_timing_resolution_encoding() {
        assert_eq!(TimingResolution::Seconds.encode(), "s");
        assert_eq!(TimingResolution::MilliSeconds.encode(), "ms");
        assert_eq!(TimingResolution::MicroSeconds.encode(), "us");
        assert_eq!(TimingResolution::NanoSeconds.encode(), "ns");
    }

    #[test]
    fn test_timing_serialization() {
        let tags = Tags::default();
        let mut buf = String::new();
        Timing::new("test", 100, TimingResolution::MilliSeconds).serialize(&tags, &mut buf);
        assert_eq!("test|t|100|ms", buf);
    }

    #[test]
    fn test_timer_with_tags() {
        let client = ClientBuilder::default()
            .connect(([127, 0, 0, 1], 0))
            .unwrap();
        let timer = client.timer("test").with_tag("t1", "v1");

        assert_eq!(timer.name, "test");
        assert_eq!(timer.tags.as_ref().unwrap().get("t1").unwrap(), "v1");
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

impl<'a> Metric for Timing<'a> {
    fn serialize(&self, tags: &Tags, buf: &mut String) {
        serialize_name_and_tags(buf, &self.name, tags, self.tags.as_ref());

        buf.push_str("|t|");
        buf.push_str(&self.duration.to_string());
        buf.push('|');
        buf.push_str(self.resolution.encode());
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
/// # let client = ClientBuilder::default().connect(([127, 0, 0, 1], 3232)).unwrap();
/// let timer = client.timer("calculation");
/// // ... complex calculation ...
/// timer.finish();
/// ```
pub struct Timer<'a> {
    pub(crate) client: &'a Client,
    pub(crate) now: Instant,
    pub(crate) name: Cow<'a, str>,
    pub(crate) tags: Option<Tags<'a>>,
}

impl<'a> Timer<'a> {
    /// Finishes the timer and sends the elapsed time to the client.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use metco_client::ClientBuilder;
    /// # let client = ClientBuilder::default().connect(([127, 0, 0, 1], 3232)).unwrap();
    /// let timer = client.timer("long_op");
    /// timer.finish();
    /// ```
    pub fn finish(self) {
        match self.tags {
            None => self.client.send(Timing::new(
                self.name,
                self.now.elapsed().as_nanos() as u64,
                TimingResolution::NanoSeconds,
            )),
            Some(tags) => self.client.send_with_tags(
                Timing::new(
                    self.name,
                    self.now.elapsed().as_nanos() as u64,
                    TimingResolution::NanoSeconds,
                ),
                tags,
            ),
        };
    }

    /// Finishes the timer and sends the elapsed time to the client with additional tags.
    pub fn finish_with_tags(self, tags: Tags<'a>) {
        match self.tags {
            None => self.client.send_with_tags(
                Timing::new(
                    self.name,
                    self.now.elapsed().as_nanos() as u64,
                    TimingResolution::NanoSeconds,
                ),
                tags,
            ),
            Some(mut inner_tags) => {
                inner_tags.extend(tags);

                self.client.send_with_tags(
                    Timing::new(
                        self.name,
                        self.now.elapsed().as_nanos() as u64,
                        TimingResolution::NanoSeconds,
                    ),
                    inner_tags,
                )
            }
        };
    }

    tags_support!();
}
