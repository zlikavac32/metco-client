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

/// A transport that sends each metric as a single UDP datagram.
///
/// Send errors are ignored.
#[derive(Debug)]
pub struct UdpTransport {
    socket: UdpSocket,
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

        Ok(Self { socket })
    }
}

impl Transport for UdpTransport {
    fn send(&self, metric: &str) {
        let _ = self.socket.send(metric.as_bytes());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ClientBuilder;

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
}
