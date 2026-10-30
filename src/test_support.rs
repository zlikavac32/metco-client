use crate::metrics::Metric;
use crate::transport::Transport;
use crate::types::{NonEmptyStr, Tags};
use std::sync::Mutex;

/// A transport that records every metric sent through it.
#[derive(Debug, Default)]
pub struct RecordingTransport(Mutex<Vec<String>>);

impl RecordingTransport {
    /// Returns all metrics sent so far, in order.
    pub fn sent(&self) -> Vec<String> {
        self.0.lock().unwrap().clone()
    }
}

impl Transport for RecordingTransport {
    fn send(&self, metric: &str) {
        self.0.lock().unwrap().push(metric.to_owned());
    }
}

/// Serializes a metric with the given default tags.
pub fn serialize_with(metric: &impl Metric, default_tags: &Tags<'_>) -> String {
    let mut buf = String::new();
    metric.serialize(default_tags, &mut buf);
    buf
}

/// Serializes a metric without any default tags.
pub fn serialize(metric: &impl Metric) -> String {
    serialize_with(metric, &Tags::new())
}

/// Builds tags from `(name, value)` pairs.
///
/// # Panics
///
/// Panics if any name or value is empty.
pub fn tags<'a>(pairs: &[(&'a str, &'a str)]) -> Tags<'a> {
    pairs
        .iter()
        .map(|&(k, v)| (NonEmptyStr::new(k).unwrap(), NonEmptyStr::new(v).unwrap()))
        .collect()
}
