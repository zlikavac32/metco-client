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
    use crate::metrics::{Counter, Histogram};
    use crate::test_support::{RecordingTransport, tags};
    use crate::transport::{BlackHoleTransport, UdpTransport};

    #[test]
    fn test_client_builder_prefix() {
        let builder = ClientBuilder::default().with_prefix("app.");

        assert_eq!(builder.prefix.as_deref(), Some("app."));
    }

    #[test]
    fn test_client_builder_last_prefix_wins() {
        let builder = ClientBuilder::default()
            .with_prefix("first.")
            .with_prefix(String::from("second."));

        assert_eq!(builder.prefix.as_deref(), Some("second."));
    }

    #[test]
    fn test_prefix_escaping() {
        let client = ClientBuilder::default()
            .with_prefix("a|b;c\\d.")
            .build(BlackHoleTransport);

        assert_eq!(client.prefix.as_deref(), Some(r"a\|b\;c\\d."));
    }

    #[test]
    fn test_client_builder_tags() {
        let builder = ClientBuilder::default()
            .with_tag("t1", "v1")
            .with_tags(tags(&[("t2", "v2")]));

        assert_eq!(builder.tags.len(), 2);
        assert_eq!(builder.tags.get("t1").unwrap(), "v1");
        assert_eq!(builder.tags.get("t2").unwrap(), "v2");
    }

    #[test]
    fn test_client_builder_later_tags_override_earlier() {
        let builder = ClientBuilder::default()
            .with_tag("a", "1")
            .with_tag("b", "1")
            .with_tags(tags(&[("a", "2")]))
            .with_tag("b", String::from("2"));

        assert_eq!(builder.tags, tags(&[("a", "2"), ("b", "2")]));
    }

    #[test]
    fn test_client_builder_clone_is_independent() {
        let base = ClientBuilder::default().with_tag("a", "1");
        let extended = base.clone().with_tag("b", "2").with_prefix("p.");

        assert_eq!(base.tags.len(), 1);
        assert_eq!(base.prefix, None);
        assert_eq!(extended.tags.len(), 2);
    }

    #[test]
    fn test_send_without_prefix_or_tags() {
        let transport = RecordingTransport::default();
        let client = ClientBuilder::default().build(&transport);

        client.send(Counter::new("c", 1));

        assert_eq!(transport.sent(), ["c|c|1"]);
    }

    #[test]
    fn test_send_applies_client_tags() {
        let transport = RecordingTransport::default();
        let client = ClientBuilder::default()
            .with_tag("env", "prod")
            .build(&transport);

        client.send(Counter::new("c", 1));

        assert_eq!(transport.sent(), ["c;env=prod|c|1"]);
    }

    #[test]
    fn test_metric_tags_override_client_tags() {
        let transport = RecordingTransport::default();
        let client = ClientBuilder::default()
            .with_tag("env", "prod")
            .with_tag("host", "a")
            .build(&transport);

        client.send(Counter::new("c", 1).with_tag("env", "dev"));

        assert_eq!(transport.sent(), ["c;env=dev;host=a|c|1"]);
    }

    #[test]
    fn test_send_with_tags_precedence() {
        let transport = RecordingTransport::default();
        let client = ClientBuilder::default()
            .with_tag("a", "client")
            .with_tag("b", "client")
            .with_tag("c", "client")
            .build(&transport);

        client.send_with_tags(
            Counter::new("m", 1).with_tag("c", "metric"),
            tags(&[("b", "send"), ("c", "send"), ("d", "send")]),
        );

        assert_eq!(transport.sent(), ["m;a=client;b=send;c=metric;d=send|c|1"]);
    }

    #[test]
    fn test_send_with_empty_tags() {
        let transport = RecordingTransport::default();
        let client = ClientBuilder::default()
            .with_tag("a", "1")
            .build(&transport);

        client.send_with_tags(Counter::new("m", 1), Tags::new());

        assert_eq!(transport.sent(), ["m;a=1|c|1"]);
    }

    #[test]
    fn test_send_with_tags_does_not_modify_client_tags() {
        let transport = RecordingTransport::default();
        let client = ClientBuilder::default()
            .with_tag("a", "1")
            .build(&transport);

        client
            .send_with_tags(Counter::new("m", 1), tags(&[("a", "2"), ("b", "3")]))
            .send(Counter::new("m", 1));

        assert_eq!(transport.sent(), ["m;a=2;b=3|c|1", "m;a=1|c|1"]);
    }

    #[test]
    fn test_prefix_applies_to_name_only() {
        let transport = RecordingTransport::default();
        let client = ClientBuilder::default()
            .with_prefix("app.")
            .with_tag("a", "1")
            .build(&transport);

        client.send(Counter::new("m", 1).with_tag("b", "2"));

        assert_eq!(transport.sent(), ["app.m;a=1;b=2|c|1"]);
    }

    #[test]
    fn test_empty_prefix() {
        let transport = RecordingTransport::default();
        let client = ClientBuilder::default().with_prefix("").build(&transport);

        client.send(Counter::new("m", 1));

        assert_eq!(transport.sent(), ["m|c|1"]);
    }

    #[test]
    fn test_escaped_prefix_and_name_are_concatenated() {
        let transport = RecordingTransport::default();
        let client = ClientBuilder::default().with_prefix("a|").build(&transport);

        client.send(Counter::new("|b", 1));

        assert_eq!(transport.sent(), [r"a\|\|b|c|1"]);
    }

    #[test]
    fn test_send_chaining_preserves_order() {
        let transport = RecordingTransport::default();
        let client = ClientBuilder::default().build(&transport);

        client
            .send(Counter::new("a", 1))
            .send_with_tags(Histogram::new("b", 2), tags(&[("t", "v")]))
            .send(Counter::new("c", 3));

        assert_eq!(transport.sent(), ["a|c|1", "b;t=v|h|2", "c|c|3"]);
    }

    #[test]
    fn test_send_accepts_long_metrics() {
        let transport = RecordingTransport::default();
        let client = ClientBuilder::default().build(&transport);
        let name = "n".repeat(1000);

        client.send(Counter::new(name.as_str(), 1));

        assert_eq!(transport.sent(), [format!("{name}|c|1")]);
    }

    #[test]
    fn test_client_with_boxed_transport() {
        let transport = RecordingTransport::default();
        let boxed: Box<dyn Transport + '_> = Box::new(&transport);
        let client = ClientBuilder::default().build(boxed);

        client.send(Counter::new("m", 1));

        assert_eq!(transport.sent(), ["m|c|1"]);
    }

    #[test]
    fn test_client_is_send_and_sync_with_udp_transport() {
        fn assert_send_sync<T: Send + Sync>() {}

        assert_send_sync::<Client<UdpTransport>>();
        assert_send_sync::<Client<BlackHoleTransport>>();
        assert_send_sync::<ClientBuilder>();
    }
}
