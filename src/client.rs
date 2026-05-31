use crate::metrics::{GaugeHandle, Metric, Timer};
use crate::transport::Transport;
use crate::types::Tags;
use crate::utils::write_escaped_value;
use std::borrow::Cow;
use std::time::Instant;

/// MetCo client for sending metrics over UDP.
pub struct Client<T> {
    transport: T,
    tags: Tags<'static>,
    prefix: Option<String>,
}

impl<T: Transport> Client<T> {
    /// Sends a metric to the MetCo server.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use metco_client::{ClientBuilder, Counter, transport::UdpTransport};
    /// # let client = ClientBuilder::default().build(UdpTransport::connect(([127, 0, 0, 1], 3232)).unwrap());
    /// client.send(Counter::new("requests", 1));
    /// ```
    pub fn send<M: Metric>(&self, metric: M) -> &Self {
        let mut buf = String::with_capacity(128);

        if let Some(prefix) = &self.prefix {
            buf.push_str(prefix);
        }

        metric.serialize(&self.tags, &mut buf);
        self.transport.send(&buf);

        self
    }

    /// Sends a metric to the MetCo server with additional tags.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use metco_client::{ClientBuilder, Counter, Tags, transport::UdpTransport};
    /// # let client = ClientBuilder::default().build(UdpTransport::connect(([127, 0, 0, 1], 3232)).unwrap());
    /// let mut tags = Tags::new();
    /// tags.insert("env".into(), "prod".into());
    /// client.send_with_tags(Counter::new("requests", 1), tags);
    /// ```
    pub fn send_with_tags<'a, M: Metric>(&self, metric: M, mut tags: Tags<'a>) -> &Self {
        for (k, v) in &self.tags {
            tags.entry(k.clone()).or_insert_with(|| v.clone());
        }

        let mut buf = String::with_capacity(128);

        if let Some(prefix) = &self.prefix {
            buf.push_str(prefix);
        }

        metric.serialize(&tags, &mut buf);
        self.transport.send(&buf);

        self
    }

    /// Creates a new timer for measuring durations.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use metco_client::{ClientBuilder, transport::UdpTransport};
    /// # let client = ClientBuilder::default().build(UdpTransport::connect(([127, 0, 0, 1], 3232)).unwrap());
    /// let timer = client.timer("db_query");
    /// // ... perform operation ...
    /// timer.finish();
    /// ```
    pub fn timer<'a, N: Into<Cow<'a, str>>>(&'a self, name: N) -> Timer<'a, T> {
        Timer {
            client: self,
            now: Instant::now(),
            name: name.into(),
            tags: None,
        }
    }

    /// Creates a handle for performing operations on a gauge.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use metco_client::{ClientBuilder, transport::UdpTransport};
    /// # let client = ClientBuilder::default().build(UdpTransport::connect(([127, 0, 0, 1], 3232)).unwrap());
    /// let gauge = client.gauge("active_users");
    /// gauge.set(42);
    /// gauge.increment(1);
    /// ```
    pub fn gauge<'a, N: Into<Cow<'a, str>>>(&'a self, name: N) -> GaugeHandle<'a, T> {
        GaugeHandle {
            client: self,
            name: name.into(),
        }
    }
}

/// Builder for creating a [`Client`].
#[derive(Default)]
pub struct ClientBuilder {
    tags: Tags<'static>,
    prefix: Option<String>,
}

impl ClientBuilder {
    /// Adds a default tag to all metrics sent by the client.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use metco_client::ClientBuilder;
    /// let builder = ClientBuilder::default()
    ///     .with_tag("app", "api");
    /// ```
    pub fn with_tag<K: Into<Cow<'static, str>>, V: Into<Cow<'static, str>>>(
        mut self,
        name: K,
        value: V,
    ) -> Self {
        self.tags.insert(name.into(), value.into());

        self
    }

    pub fn with_prefix<P: Into<String>>(mut self, prefix: P) -> Self {
        self.prefix = Some(prefix.into());
        self
    }

    /// Adds multiple default tags to all metrics sent by the client.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use metco_client::{ClientBuilder, Tags};
    /// let mut tags = Tags::new();
    /// tags.insert("source".into(), "api".into());
    ///
    /// let builder = ClientBuilder::default()
    ///     .with_tags(tags);
    /// ```
    pub fn with_tags(mut self, tags: Tags<'static>) -> Self {
        self.tags.extend(tags);

        self
    }

    /// Connects to the MetCo server at the given address.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use metco_client::transport::BlackHoleTransport;
    /// use metco_client::ClientBuilder;
    ///
    /// let client = ClientBuilder::default()
    ///     .build(BlackHoleTransport::default());
    /// ```
    pub fn build<T: Transport>(self, transport: T) -> Client<T> {
        let prefix = self.prefix.map(|p| {
            let mut escaped = String::with_capacity(p.len());

            write_escaped_value(&mut escaped, &p);

            escaped
        });

        Client {
            transport,
            tags: self.tags,
            prefix,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transport::BlackHoleTransport;

    #[test]
    fn test_client_builder_prefix() {
        let builder = ClientBuilder::default().with_prefix("app.");

        assert_eq!(builder.prefix.as_deref(), Some("app."));
    }

    #[test]
    fn test_prefix_escaping() {
        let client = ClientBuilder::default()
            .with_prefix("app|test.")
            .build(BlackHoleTransport::default());

        assert_eq!(client.prefix.as_deref(), Some("app\\|test."));
    }

    #[test]
    fn test_client_builder_tags() {
        let builder = ClientBuilder::default()
            .with_tag("t1", "v1")
            .with_tags(Tags::from([("t2".into(), "v2".into())]));

        assert_eq!(builder.tags.len(), 2);
        assert_eq!(builder.tags.get("t1").unwrap(), "v1");
        assert_eq!(builder.tags.get("t2").unwrap(), "v2");
    }
}
