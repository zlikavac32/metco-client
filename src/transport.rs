use std::fmt;
use std::io;
use std::net::{Ipv4Addr, Ipv6Addr, SocketAddr, UdpSocket};
use std::rc::Rc;
use std::sync::Arc;

/// Delivers serialized metrics to a MetCo server.
pub trait Transport {
    /// Sends a single serialized metric.
    fn send(&self, metric: &str);
}

impl<T: Transport + ?Sized> Transport for &T {
    fn send(&self, metric: &str) {
        (**self).send(metric);
    }
}

impl<T: Transport + ?Sized> Transport for Box<T> {
    fn send(&self, metric: &str) {
        (**self).send(metric);
    }
}

impl<T: Transport + ?Sized> Transport for Rc<T> {
    fn send(&self, metric: &str) {
        (**self).send(metric);
    }
}

impl<T: Transport + ?Sized> Transport for Arc<T> {
    fn send(&self, metric: &str) {
        (**self).send(metric);
    }
}

/// A transport that discards every metric.
#[derive(Debug, Default, Clone, Copy)]
pub struct BlackHoleTransport;

impl Transport for BlackHoleTransport {
    fn send(&self, _metric: &str) {}
}

type ErrorHandler = Box<dyn Fn(&io::Error) + Send + Sync>;

/// A transport that sends each metric as a single UDP datagram.
///
/// Send errors are ignored unless an error handler is registered with
/// [`UdpTransport::with_error_handler`].
pub struct UdpTransport {
    socket: UdpSocket,
    on_error: Option<ErrorHandler>,
}

impl fmt::Debug for UdpTransport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("UdpTransport")
            .field("socket", &self.socket)
            .field("on_error", &self.on_error.is_some())
            .finish()
    }
}

impl UdpTransport {
    /// Binds an ephemeral local socket of the same address family as `addr`
    /// and connects it to `addr`.
    ///
    /// # Errors
    ///
    /// Returns an error if the local socket cannot be bound or connected.
    pub fn connect<A: Into<SocketAddr>>(addr: A) -> io::Result<Self> {
        let addr = addr.into();
        let local: SocketAddr = match addr {
            SocketAddr::V4(_) => (Ipv4Addr::UNSPECIFIED, 0).into(),
            SocketAddr::V6(_) => (Ipv6Addr::UNSPECIFIED, 0).into(),
        };

        let socket = UdpSocket::bind(local)?;
        socket.connect(addr)?;

        Ok(Self {
            socket,
            on_error: None,
        })
    }

    /// Registers a handler that is called whenever sending a metric fails.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use metco_client::transport::UdpTransport;
    /// let transport = UdpTransport::connect(([127, 0, 0, 1], 3232))
    ///     .unwrap()
    ///     .with_error_handler(|err| eprintln!("failed to send metric: {err}"));
    /// ```
    #[must_use]
    pub fn with_error_handler<F>(mut self, handler: F) -> Self
    where
        F: Fn(&io::Error) + Send + Sync + 'static,
    {
        self.on_error = Some(Box::new(handler));
        self
    }
}

impl Transport for UdpTransport {
    fn send(&self, metric: &str) {
        if let (Err(err), Some(on_error)) = (self.socket.send(metric.as_bytes()), &self.on_error) {
            on_error(&err);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ClientBuilder;
    use crate::test_support::RecordingTransport;
    use std::sync::atomic::{AtomicUsize, Ordering};

    /// Returns an error handler that counts calls, and the counter it updates.
    fn counting_handler() -> (
        Arc<AtomicUsize>,
        impl Fn(&io::Error) + Send + Sync + 'static,
    ) {
        let errors = Arc::new(AtomicUsize::new(0));
        let handler = {
            let errors = Arc::clone(&errors);
            move |_: &io::Error| {
                errors.fetch_add(1, Ordering::Relaxed);
            }
        };

        (errors, handler)
    }

    /// Larger than the maximum UDP payload, so sending always fails.
    fn oversized_metric() -> String {
        "x".repeat(70_000)
    }

    #[test]
    fn test_smart_pointers_are_transports() {
        let boxed: Box<dyn Transport> = Box::new(BlackHoleTransport);
        let boxed_send_sync: Box<dyn Transport + Send + Sync> = Box::new(BlackHoleTransport);
        let shared: Arc<dyn Transport> = Arc::new(BlackHoleTransport);

        ClientBuilder::default().build(boxed);
        ClientBuilder::default().build(boxed_send_sync);
        ClientBuilder::default().build(Rc::new(BlackHoleTransport));
        ClientBuilder::default().build(&*shared);
        ClientBuilder::default().build(shared);
    }

    #[test]
    fn test_smart_pointers_forward_to_inner_transport() {
        let transport = RecordingTransport::default();

        <&RecordingTransport as Transport>::send(&&transport, "ref");
        Box::new(&transport).send("box");
        Rc::new(&transport).send("rc");
        Arc::new(&transport).send("arc");
        (Box::new(&transport) as Box<dyn Transport + '_>).send("dyn");
        Box::new(Box::new(&transport)).send("nested");

        assert_eq!(
            transport.sent(),
            ["ref", "box", "rc", "arc", "dyn", "nested"]
        );
    }

    #[test]
    fn test_black_hole_transport_accepts_anything() {
        BlackHoleTransport.send("");
        BlackHoleTransport.send(&oversized_metric());
    }

    #[test]
    fn test_udp_connect_binds_matching_address_family() {
        let v4 = UdpTransport::connect(([127, 0, 0, 1], 3232)).unwrap();
        let v6 = UdpTransport::connect((Ipv6Addr::LOCALHOST, 3232)).unwrap();

        assert!(v4.socket.local_addr().unwrap().is_ipv4());
        assert!(v6.socket.local_addr().unwrap().is_ipv6());
    }

    #[test]
    fn test_udp_send_errors_are_reported() {
        let (errors, handler) = counting_handler();
        let transport = UdpTransport::connect(([127, 0, 0, 1], 3232))
            .unwrap()
            .with_error_handler(handler);

        transport.send(&oversized_metric());
        transport.send(&oversized_metric());

        assert_eq!(errors.load(Ordering::Relaxed), 2);
    }

    #[test]
    fn test_udp_error_handler_is_not_called_on_success() {
        let server = UdpSocket::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
        let (errors, handler) = counting_handler();
        let transport = UdpTransport::connect(server.local_addr().unwrap())
            .unwrap()
            .with_error_handler(handler);

        transport.send("m|c|1");

        assert_eq!(errors.load(Ordering::Relaxed), 0);
    }

    #[test]
    fn test_udp_last_error_handler_wins() {
        let (first, first_handler) = counting_handler();
        let (second, second_handler) = counting_handler();
        let transport = UdpTransport::connect(([127, 0, 0, 1], 3232))
            .unwrap()
            .with_error_handler(first_handler)
            .with_error_handler(second_handler);

        transport.send(&oversized_metric());

        assert_eq!(first.load(Ordering::Relaxed), 0);
        assert_eq!(second.load(Ordering::Relaxed), 1);
    }

    #[test]
    fn test_udp_send_errors_are_ignored_without_handler() {
        let transport = UdpTransport::connect(([127, 0, 0, 1], 3232)).unwrap();

        transport.send(&oversized_metric());
    }

    #[test]
    fn test_udp_transport_debug_shows_whether_handler_is_set() {
        let transport = UdpTransport::connect(([127, 0, 0, 1], 3232)).unwrap();
        assert!(format!("{transport:?}").contains("on_error: false"));

        let transport = transport.with_error_handler(|_| {});
        assert!(format!("{transport:?}").contains("on_error: true"));
    }

    #[test]
    fn test_udp_transport_is_send_and_sync() {
        fn assert_send_sync<T: Send + Sync>() {}

        assert_send_sync::<UdpTransport>();
        assert_send_sync::<BlackHoleTransport>();
    }
}
