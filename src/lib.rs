//! # MetCo Client
//!
//! Client library for [MetCo](https://github.com/zlikavac32/metco).
//!
//! ## Usage
//!
//! ```yaml
//! [dependencies]
//! metco-client = "0.1.0"
//! ```
//!
//! ```rust
//! use metco_client::{ClientBuilder, Counter, transport::UdpTransport};
//!
//! let client = ClientBuilder::default().build(UdpTransport::connect(([127, 0, 0, 1], 0)).unwrap());
//! client.send(Counter::new("test", 12));
//! ```

pub mod client;
pub mod metrics;
pub mod transport;
pub mod types;
pub mod utils;

#[cfg(test)]
mod integration_tests;

pub use client::{Client, ClientBuilder};
pub use metrics::{
    Counter, Gauge, GaugeHandle, GaugeOperation, Histogram, Metric, Timer, Timing, TimingResolution,
};
pub use types::Tags;

/// A module containing all the types and traits needed to use the library.
pub mod prelude {
    pub use crate::client::{Client, ClientBuilder};
    pub use crate::metrics::{
        Counter, Gauge, GaugeHandle, GaugeOperation, Histogram, Metric, Timer, Timing,
        TimingResolution,
    };
    pub use crate::types::Tags;
}
