use crate::metrics::{GaugeHandle, Metric, Timer};
use crate::transport::Transport;
use crate::types::Tags;
use crate::utils::write_escaped_value;
use std::borrow::Cow;

/// MetCo client for sending metrics through a [`Transport`].
#[derive(Debug)]
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
        self.dispatch(&metric, &self.tags)
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
    pub fn send_with_tags<M: Metric>(&self, metric: M, mut tags: Tags<'_>) -> &Self {
        for (k, v) in &self.tags {
            tags.entry(k.clone()).or_insert_with(|| v.clone());
        }

        self.dispatch(&metric, &tags)
    }

    fn dispatch<M: Metric>(&self, metric: &M, tags: &Tags<'_>) -> &Self {
        let mut buf = String::with_capacity(128);

        if let Some(prefix) = &self.prefix {
            buf.push_str(prefix);
        }

        metric.serialize(tags, &mut buf);
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
        Timer::new(self, name.into())
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
        GaugeHandle::new(self, name.into())
    }
}

/// Builder for creating a [`Client`].
#[derive(Debug, Default, Clone)]
#[must_use]
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
    pub fn with_tag<K, V>(mut self, name: K, value: V) -> Self
    where
        K: Into<Cow<'static, str>>,
        V: Into<Cow<'static, str>>,
    {
        self.tags.insert(name.into(), value.into());

        self
    }

    /// Sets a prefix prepended to the name of every metric sent by the client.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use metco_client::ClientBuilder;
    /// let builder = ClientBuilder::default()
    ///     .with_prefix("app.");
    /// ```
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

    /// Builds a client that sends metrics through the given transport.
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
            let mut escaped = String::new();
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
            .build(BlackHoleTransport);

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
