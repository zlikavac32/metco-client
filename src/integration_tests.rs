use crate::prelude::*;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, UdpSocket};
use std::time::{Duration, Instant};

/// Binds a server socket on an ephemeral port and returns it together with
/// a client connected to it.
fn server_and_client(builder: ClientBuilder) -> (UdpSocket, Client<UdpTransport>) {
    server_and_client_on(builder, Ipv4Addr::LOCALHOST.into())
}

fn server_and_client_on(builder: ClientBuilder, ip: IpAddr) -> (UdpSocket, Client<UdpTransport>) {
    let server = UdpSocket::bind((ip, 0)).unwrap();
    server
        .set_read_timeout(Some(Duration::from_secs(1)))
        .unwrap();

    let client = builder.build(UdpTransport::connect(server.local_addr().unwrap()).unwrap());

    (server, client)
}

fn receive(server: &UdpSocket) -> String {
    let mut buf = [0; 256];
    let (size, _) = server.recv_from(&mut buf).unwrap();

    String::from_utf8(buf[..size].to_vec()).unwrap()
}

fn serialize(metric: impl Metric) -> String {
    let mut buf = String::new();
    metric.serialize(&Tags::default(), &mut buf);
    buf
}

#[test]
fn test_timer_is_sent() {
    let (server, client) = server_and_client(ClientBuilder::default());

    let now = Instant::now();
    client.timer("test").finish();
    let duration = now.elapsed();

    let payload = receive(&server);
    let split = payload.split('|').collect::<Vec<_>>();

    assert_eq!(4, split.len());
    assert_eq!("test", split[0]);
    assert_eq!("t", split[1]);
    assert!(duration.as_nanos() >= split[2].parse().unwrap());
    assert_eq!("ns", split[3]);
}

#[test]
fn test_client_and_send_data() {
    let (server, client) = server_and_client(ClientBuilder::default());

    client
        .send(Counter::new("test", 12))
        .send(Histogram::new("test2", 123));

    assert_eq!("test|c|12", receive(&server));
    assert_eq!("test2|h|123", receive(&server));
}

#[test]
fn test_client_sends_over_ipv6() {
    let (server, client) =
        server_and_client_on(ClientBuilder::default(), Ipv6Addr::LOCALHOST.into());

    client.send(Counter::new("test", 1));

    assert_eq!("test|c|1", receive(&server));
}

#[test]
fn test_client_with_prefix_is_sent() {
    let (server, client) = server_and_client(ClientBuilder::default().with_prefix("app."));

    client.send(Counter::new("test", 1));

    assert_eq!("app.test|c|1", receive(&server));
}

#[test]
fn test_client_with_escaped_prefix_is_sent() {
    let (server, client) = server_and_client(ClientBuilder::default().with_prefix("app|test."));

    client.send(Counter::new("m1", 1));

    assert_eq!("app\\|test.m1|c|1", receive(&server));
}

#[test]
fn counter_is_properly_serialized() {
    assert_eq!("test|c|124", serialize(Counter::new("test", 124)));
    assert_eq!(
        "test;t1=v1|c|124",
        serialize(Counter::new("test", 124).with_tag("t1", "v1"))
    );
}

#[test]
fn histogram_is_properly_serialized() {
    assert_eq!("test|h|124", serialize(Histogram::new("test", 124)));
}

#[test]
fn timing_is_properly_serialized() {
    assert_eq!(
        "test|t|124|us",
        serialize(Timing::new("test", 124, TimingResolution::MicroSeconds))
    );
}

#[test]
fn test_tag_escaping_in_serialization() {
    let tags = Tags::from([("k=1".into(), "v;1".into())]);

    assert_eq!(
        "test;k\\=1=v\\;1|c|1",
        serialize(Counter::new("test", 1).with_tags(tags))
    );
}

#[test]
fn gauge_is_properly_serialized() {
    assert_eq!(
        "test|g|123",
        serialize(Gauge::new("test", GaugeOperation::Set(123)))
    );
}
