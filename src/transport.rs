use std::net::{SocketAddr, UdpSocket};

pub trait Transport {
    fn send(&self, metric: &str);
}

impl Transport for BlackHoleTransport {
    fn send(&self, _metric: &str) {}
}

impl Transport for UdpTransport {
    fn send(&self, metric: &str) {
        let _ = self.socket.send(metric.as_bytes());
    }
}

#[derive(Default)]
pub struct BlackHoleTransport {}

pub struct UdpTransport {
    socket: UdpSocket,
}

impl UdpTransport {
    pub fn connect<T: Into<SocketAddr>>(addr: T) -> std::io::Result<Self> {
        let socket = UdpSocket::bind("0.0.0.0:0")?;
        socket.connect(addr.into())?;

        Ok(Self { socket })
    }
}
