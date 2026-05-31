#[cfg(test)]
mod tests {
    use crate::prelude::*;
    use crate::transport::UdpTransport;
    use std::net::UdpSocket;
    use std::time::{Duration, Instant};

    #[test]
    fn test_timer_is_sent() {
        let server = UdpSocket::bind("127.0.0.32:3231").unwrap();
        server
            .set_read_timeout(Some(Duration::from_nanos(1)))
            .unwrap();

        let client =
            ClientBuilder::default().build(UdpTransport::connect(([127, 0, 0, 32], 3231)).unwrap());

        let now = Instant::now();
        client.timer("test").finish();
        let duration = now.elapsed();

        let mut buff = [0; 256];

        let (size, _) = server.recv_from(&mut buff).unwrap();

        let payload = std::str::from_utf8(&buff[..size]).unwrap();

        let split = payload.split('|').collect::<Vec<&str>>();

        assert_eq!(4, split.len(),);

        assert_eq!("test", split[0]);
        assert_eq!("t", split[1]);

        assert!(duration.as_nanos() as u64 > split[2].parse().unwrap());

        assert_eq!("ns", split[3]);
    }

    #[test]
    fn test_client_and_send_data() {
        let server = UdpSocket::bind("127.0.0.32:3232").unwrap();
        server
            .set_read_timeout(Some(Duration::from_nanos(1)))
            .unwrap();

        let client =
            ClientBuilder::default().build(UdpTransport::connect(([127, 0, 0, 32], 3232)).unwrap());

        client
            .send(Counter::new("test", 12))
            .send(Histogram::new("test2", 123));

        let mut buff = [0; 256];

        let (size, _) = server.recv_from(&mut buff).unwrap();

        assert_eq!(b"test|c|12", &buff[..size]);

        let (size, _) = server.recv_from(&mut buff).unwrap();

        assert_eq!(b"test2|h|123", &buff[..size]);
    }

    #[test]
    fn test_client_with_prefix_is_sent() {
        let server = UdpSocket::bind("127.0.0.32:3233").unwrap();
        server
            .set_read_timeout(Some(Duration::from_millis(100)))
            .unwrap();

        let client = ClientBuilder::default()
            .with_prefix("app.")
            .build(UdpTransport::connect(([127, 0, 0, 32], 3233)).unwrap());

        client.send(Counter::new("test", 1));

        let mut buff = [0; 256];
        let (size, _) = server.recv_from(&mut buff).unwrap();
        assert_eq!(b"app.test|c|1", &buff[..size]);
    }

    #[test]
    fn test_client_with_escaped_prefix_is_sent() {
        let server = UdpSocket::bind("127.0.0.32:3234").unwrap();
        server
            .set_read_timeout(Some(Duration::from_millis(100)))
            .unwrap();

        let client = ClientBuilder::default()
            .with_prefix("app|test.")
            .build(UdpTransport::connect(([127, 0, 0, 32], 3234)).unwrap());

        client.send(Counter::new("m1", 1));

        let mut buff = [0; 256];
        let (size, _) = server.recv_from(&mut buff).unwrap();
        assert_eq!(b"app\\|test.m1|c|1", &buff[..size]);
    }

    #[test]
    fn counter_is_properly_serialized() {
        let mut buf = String::new();
        Counter::new("test", 124).serialize(&Tags::default(), &mut buf);
        assert_eq!("test|c|124", buf);

        let mut buf = String::new();
        Counter::new("test", 124)
            .with_tag("t1", "v1")
            .serialize(&Tags::default(), &mut buf);
        assert_eq!("test;t1=v1|c|124", buf);
    }

    #[test]
    fn histogram_is_properly_serialized() {
        let mut buf = String::new();
        Histogram::new("test", 124).serialize(&Tags::default(), &mut buf);
        assert_eq!("test|h|124", buf);
    }

    #[test]
    fn timing_is_properly_serialized() {
        let mut buf = String::new();
        Timing::new("test", 124, TimingResolution::MicroSeconds)
            .serialize(&Tags::default(), &mut buf);
        assert_eq!("test|t|124|us", buf);
    }

    #[test]
    fn test_tag_escaping_in_serialization() {
        let mut tags = Tags::new();
        tags.insert("k=1".into(), "v;1".into());
        let mut buf = String::new();
        Counter::new("test", 1)
            .with_tags(tags)
            .serialize(&Tags::default(), &mut buf);
        assert_eq!("test;k\\=1=v\\;1|c|1", buf);
    }

    #[test]
    fn gauge_is_properly_serialized() {
        let mut buf = String::new();
        Gauge::new("test", GaugeOperation::Set(123)).serialize(&Tags::default(), &mut buf);
        assert_eq!("test|g|123", buf);
    }
}
