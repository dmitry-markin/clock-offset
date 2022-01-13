use anyhow::Result;
use clap::Parser;
use nix::time::{clock_gettime, ClockId};
use std::net::{Ipv4Addr, SocketAddrV4};
use tokio::{
    net::UdpSocket,
    time::{sleep, Duration}
};

const PAYLOAD_SIZE: usize = 16;
const NANOSECONDS_IN_SECOND: i128 = 1000000000;

/// UDP-based naive clock offset measurement tool
#[derive(Parser, Debug)]
struct Args {
    /// Stream timestamps to host
    remote_ip: Option<String>,

    /// Port to listen for incoming timestamps on
    #[clap(short, long, default_value_t = 55555)]
    port: u16,

    /// Timestamp sending interval (seconds)
    #[clap(short, long, default_value_t = 1.0)]
    interval: f64
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    match args.remote_ip {
        Some(remote_ip) => send(remote_ip.parse()?, args.port, args.interval).await,
        None => receive(args.port).await
    }
}

async fn send(remote_ip: Ipv4Addr, port: u16, interval: f64) -> Result<()> {
    let sockaddr = SocketAddrV4::new(remote_ip, port);
    eprintln!("Sending timestamps to {} every {} seconds...", sockaddr, interval);

    let socket = UdpSocket::bind("0.0.0.0:0").await?;
    socket.connect(sockaddr).await?;

    loop {
        let time = clock_gettime(ClockId::CLOCK_REALTIME)?;
        let sec: i64 = time.tv_sec();
        let nsec: i64 = time.tv_nsec();
        let payload = [sec.to_le_bytes(), nsec.to_le_bytes()].concat();
        assert_eq!(payload.len(), PAYLOAD_SIZE);

        socket.send(&payload[..]).await?;

        sleep(Duration::from_secs_f64(interval)).await;
    }
}

async fn receive(port: u16) -> Result<()> {
    eprintln!("Listening for timestamps on port {}...", port);

    let socket = UdpSocket::bind(SocketAddrV4::new(Ipv4Addr::new(0, 0, 0, 0), port)).await?;
    let mut buf = [0; 1024];
    loop {
        let (len, _addr) = socket.recv_from(&mut buf).await?;
        if len != PAYLOAD_SIZE {
            eprintln!("Invalid packet: payload size {} != {}", len, PAYLOAD_SIZE);
            continue;
        }

        let received = clock_gettime(ClockId::CLOCK_REALTIME)?;
        let received_sec = received.tv_sec();
        let received_nsec = received.tv_nsec();
        let received_total = received_sec as i128 * NANOSECONDS_IN_SECOND + received_nsec as i128;

        let sent_sec = i64::from_le_bytes(buf[..8].try_into()?);
        let sent_nsec = i64::from_le_bytes(buf[8..16].try_into()?);
        let sent_total: i128 = sent_sec as i128 * NANOSECONDS_IN_SECOND + sent_nsec as i128;

        let offset = (received_total - sent_total) as f64 / 1e9;

        println!("{}.{:09}, {}.{:09}, {:.9}", sent_sec, sent_nsec, received_sec, received_nsec, offset);
    }
}
