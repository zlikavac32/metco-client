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
    let mut buf = [0; 65_536];
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
    client.timer(non_empty!("test")).finish();
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
        .send(Counter::new(non_empty!("test"), 12))
        .send(Histogram::new(non_empty!("test2"), 123));

    assert_eq!("test|c|12", receive(&server));
    assert_eq!("test2|h|123", receive(&server));
}

#[test]
fn test_client_sends_over_ipv6() {
    let (server, client) =
        server_and_client_on(ClientBuilder::default(), Ipv6Addr::LOCALHOST.into());

    client.send(Counter::new(non_empty!("test"), 1));

    assert_eq!("test|c|1", receive(&server));
}

#[test]
fn test_client_with_prefix_is_sent() {
    let (server, client) = server_and_client(ClientBuilder::default().with_prefix("app."));

    client.send(Counter::new(non_empty!("test"), 1));

    assert_eq!("app.test|c|1", receive(&server));
}

#[test]
fn test_client_with_escaped_prefix_is_sent() {
    let (server, client) = server_and_client(ClientBuilder::default().with_prefix("app|test."));

    client.send(Counter::new(non_empty!("m1"), 1));

    assert_eq!("app\\|test.m1|c|1", receive(&server));
}

#[test]
fn counter_is_properly_serialized() {
    assert_eq!(
        "test|c|124",
        serialize(Counter::new(non_empty!("test"), 124))
    );
    assert_eq!(
        "test;t1=v1|c|124",
        serialize(
            Counter::new(non_empty!("test"), 124).with_tag(non_empty!("t1"), non_empty!("v1"))
        )
    );
}

#[test]
fn histogram_is_properly_serialized() {
    assert_eq!(
        "test|h|124",
        serialize(Histogram::new(non_empty!("test"), 124))
    );
}

#[test]
fn timing_is_properly_serialized() {
    assert_eq!(
        "test|t|124|us",
        serialize(Timing::new(
            non_empty!("test"),
            124,
            TimingResolution::MicroSeconds
        ))
    );
}

#[test]
fn test_tag_escaping_in_serialization() {
    let tags = Tags::from([(non_empty!("k=1"), non_empty!("v;1"))]);

    assert_eq!(
        "test;k\\=1=v\\;1|c|1",
        serialize(Counter::new(non_empty!("test"), 1).with_tags(tags))
    );
}

#[test]
fn gauge_is_properly_serialized() {
    assert_eq!(
        "test|g|123",
        serialize(Gauge::new(non_empty!("test"), GaugeOperation::Set(123)))
    );
}

#[test]
fn test_full_metric_is_sent_over_udp() {
    let (server, client) = server_and_client(
        ClientBuilder::default()
            .with_prefix("app.")
            .with_tag(non_empty!("env"), non_empty!("prod"))
            .with_tag(non_empty!("host"), non_empty!("a")),
    );

    client.send(
        Counter::new(non_empty!("req|total"), 3)
            .with_tag(non_empty!("host"), non_empty!("b"))
            .with_tag(non_empty!("k=v"), non_empty!("x;y")),
    );

    assert_eq!(
        r"app.req\|total;env=prod;host=b;k\=v=x\;y|c|3",
        receive(&server)
    );
}

#[test]
fn test_unicode_is_sent_over_udp() {
    let (server, client) = server_and_client(
        ClientBuilder::default().with_tag(non_empty!("grad"), non_empty!("Čakovec")),
    );

    client.send(Gauge::new(
        non_empty!("temperatura_°C"),
        GaugeOperation::Set(-5),
    ));

    assert_eq!("temperatura_°C;grad=Čakovec|g|-5", receive(&server));
}

#[test]
fn test_large_metric_is_sent_over_udp() {
    let (server, client) = server_and_client(ClientBuilder::default());
    let name = "n".repeat(10_000);

    client.send(Counter::new(NonEmptyStr::new(name.as_str()).unwrap(), 1));

    assert_eq!(format!("{name}|c|1"), receive(&server));
}

#[test]
fn test_gauge_and_timer_handles_send_over_udp() {
    let (server, client) = server_and_client(ClientBuilder::default());

    let gauge = client.gauge(non_empty!("g"));
    gauge.set(1);
    gauge.increment(2);
    gauge.decrement(-3);
    gauge.remove();
    {
        let _timer = client.timer(non_empty!("t"));
    }

    assert_eq!("g|g|1", receive(&server));
    assert_eq!("g|g|+=2", receive(&server));
    assert_eq!("g|g|+=3", receive(&server));
    assert_eq!("g|g|x", receive(&server));
    assert!(receive(&server).starts_with("t|t|"));
}

#[test]
fn test_client_can_be_shared_between_threads() {
    let (server, client) = server_and_client(ClientBuilder::default());

    std::thread::scope(|scope| {
        for i in 0..4 {
            let client = &client;
            scope.spawn(move || {
                client.send(Counter::new(
                    NonEmptyStr::new(format!("thread{i}")).unwrap(),
                    1,
                ))
            });
        }
    });

    let mut received: Vec<_> = (0..4).map(|_| receive(&server)).collect();
    received.sort();

    assert_eq!(
        received,
        ["thread0|c|1", "thread1|c|1", "thread2|c|1", "thread3|c|1"]
    );
}
