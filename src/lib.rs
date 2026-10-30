//! # MetCo Client
//!
//! Client library for [MetCo](https://github.com/zlikavac32/metco).
//!
//! ## Usage
//!
//! ```toml
//! [dependencies]
//! metco-client = "0.2"
//! ```
//!
//! ```rust
//! use metco_client::{ClientBuilder, Counter, non_empty, transport::UdpTransport};
//!
//! let client = ClientBuilder::default().build(UdpTransport::connect(([127, 0, 0, 1], 0)).unwrap());
//! client.send(Counter::new(non_empty!("test"), 12));
//! ```

pub mod client;
pub mod metrics;
pub mod transport;
pub mod types;
pub mod utils;

#[cfg(test)]
mod integration_tests;
#[cfg(test)]
mod test_support;

pub use client::{Client, ClientBuilder};
pub use metrics::{
    Counter, Gauge, GaugeHandle, GaugeOperation, Histogram, Metric, Timer, Timing, TimingResolution,
};
pub use types::{EmptyStrError, NonEmptyStr, Tags};

/// A module containing all the types and traits needed to use the library.
pub mod prelude {
    pub use crate::client::{Client, ClientBuilder};
    pub use crate::metrics::{
        Counter, Gauge, GaugeHandle, GaugeOperation, Histogram, Metric, Timer, Timing,
        TimingResolution,
    };
    pub use crate::non_empty;
    pub use crate::transport::{Transport, UdpTransport};
    pub use crate::types::{EmptyStrError, NonEmptyStr, Tags};
}
