# MetCo Client

[![Crates.io](https://img.shields.io/crates/v/metco-client.svg)](https://crates.io/crates/metco-client)
[![Documentation](https://docs.rs/metco-client/badge.svg)](https://docs.rs/metco-client)

A lightweight Rust client library for [MetCo](https://github.com/zlikavac32/metco), a metric collection server.

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
metco-client = "0.2"
```

## Quick Start

```rust
use metco_client::prelude::*;

fn main() -> std::io::Result<()> {
    // Create a client with default tags
    let client = ClientBuilder::default()
        .with_tag(non_empty!("env"), non_empty!("production"))
        .with_tag(non_empty!("service"), non_empty!("api"))
        .build(UdpTransport::connect(([127, 0, 0, 1], 3232))?);

    // Send a counter
    client.send(Counter::new(non_empty!("requests_total"), 1));

    // Send a counter with extra tags
    client.send(
        Counter::new(non_empty!("errors_total"), 1).with_tag(non_empty!("type"), non_empty!("auth")),
    );

    Ok(())
}
```

## Names and Tag Values

MetCo rejects metrics whose name, tag name or tag value is empty, so these are
passed as `NonEmptyStr`. String literals are checked at compile time with the
`non_empty!` macro, and other strings are checked at runtime:

```rust
let literal = non_empty!("requests_total");
let dynamic = NonEmptyStr::new(format!("requests_{}", region))?;
```

## Metric Types

### Counters
Used to track occurrences of events. They are monotonic and can only be incremented.

```rust
client.send(Counter::new(non_empty!("button_clicks"), 1));
```

### Gauges
Gauges represent a value that can go up and down.

```rust
let gauge = client.gauge(non_empty!("active_users"));

// Set absolute value
gauge.set(42);

// Increment/Decrement
gauge.increment(2);
gauge.decrement(1);

// Remove the gauge
gauge.remove();
```

### Histograms
Used to track the distribution of values (e.g., request sizes).

```rust
client.send(Histogram::new(non_empty!("response_size_bytes"), 1024));
```

### Timings & Timers
Used to measure the duration of events.

#### Manual Timing
```rust
client.send(Timing::new(non_empty!("query_time"), 150, TimingResolution::MilliSeconds));
```

#### Automatic Timer
The `Timer` measures elapsed time from its creation until `finish()` is called.

```rust
let timer = client.timer(non_empty!("db_lookup"));
// ... perform operation ...
timer.finish();
```

## Tagging

MetCo supports tagging for multidimensional metrics.

- **Default Tags**: Set on the `Client` via `ClientBuilder`. These are added to every metric sent.
- **Metric Tags**: Set on individual metrics using `.with_tag(key, value)` or `.with_tags(map)`.

Metric-specific tags will override default tags if they share the same key.

```rust
let client = ClientBuilder::default()
    .with_tag(non_empty!("source"), non_empty!("web"))
    .build(UdpTransport::connect(([127, 0, 0, 1], 3232))?);

// Resulting tags: source=web, method=GET
client.send(Counter::new(non_empty!("req"), 1).with_tag(non_empty!("method"), non_empty!("GET")));

// Overriding: Resulting tags: source=cli
client.send(Counter::new(non_empty!("req"), 1).with_tag(non_empty!("source"), non_empty!("cli")));
```

## Character Escaping

The client automatically escapes special characters (`|`, `;`, `=`, `\`) to ensure the MetCo protocol remains intact. You don't need to worry about sanitizing your metric names or tag values.
