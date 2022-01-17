use anyhow::{Context, Result};
use clap::Parser;
use nix::time::{clock_gettime, ClockId};
use std::{
    sync::Arc,
    net::{Ipv4Addr, SocketAddrV4}
};
use tokio::{
    net::UdpSocket,
    time::{sleep, Duration}
};

const PAYLOAD_SIZE: usize = 16;
const REFLECTED_PAYLOAD_SIZE: usize = 32;

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

fn time_realtime() -> Result<(i64, i64)> {
    let time = clock_gettime(ClockId::CLOCK_REALTIME).context("clock_gettime() call failed")?;
    Ok((time.tv_sec(), time.tv_nsec()))
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    if let Some(remote_ip) = args.remote_ip {
        measure(SocketAddrV4::new(remote_ip.parse()?, args.port), args.interval).await
    } else {
        reflect(args.port).await
    }
}

async fn reflect(port: u16) -> Result<()> {
    let sockaddr = SocketAddrV4::new(Ipv4Addr::new(0, 0, 0, 0), port);
    eprintln!("Reflecting packets on {}...", sockaddr);

    let socket = UdpSocket::bind(sockaddr).await?;
    let mut buf = [0; 2048];  // should be enough for MTU 1500

    loop {
        let (len, addr) = socket.recv_from(&mut buf).await?;
        if len != PAYLOAD_SIZE {
            eprintln!("Invalid packet discarded: payload size {} != {}", len, PAYLOAD_SIZE);
            continue;
        }

        let (sec, nsec) = time_realtime()?;
        let reply = [&buf[..PAYLOAD_SIZE], &sec.to_le_bytes(), &nsec.to_le_bytes()].concat();
        socket.send_to(&reply, &addr).await?;
    }
}

async fn measure(remote: SocketAddrV4, interval: f64) -> Result<()> {
    eprintln!("Sending timestamps to {} every {} seconds...", remote, interval);

    let socket = UdpSocket::bind("0.0.0.0:0").await?;
    socket.connect(remote).await?;
    let socket_receive = Arc::new(socket);
    let socket_send = socket_receive.clone();

    tokio::spawn(async move {
        receive(socket_receive).await
    });

    send(socket_send, Duration::from_secs_f64(interval)).await
}

async fn receive(socket: Arc<UdpSocket>) -> Result<()> {
    println!("t1, tau2, t3, offset_min, offset_max, offset");

    let mut buf = [0; 2048];

    loop {
        let len = socket.recv(&mut buf).await?;
        if len != REFLECTED_PAYLOAD_SIZE {
            eprintln!(
                "Invalid packet discarded: payload size {} != {}",
                len,
                REFLECTED_PAYLOAD_SIZE
            );
            continue;
        }

        let (sec3, nsec3) = time_realtime()?;
        let sec1 = i64::from_le_bytes(buf[..8].try_into()?);
        let nsec1 = i64::from_le_bytes(buf[8..16].try_into()?);
        let sec2 = i64::from_le_bytes(buf[16..24].try_into()?);
        let nsec2 = i64::from_le_bytes(buf[24..32].try_into()?);

        let t1 = total_nsec(sec1, nsec1);
        let tau2 = total_nsec(sec2, nsec2);  // reference time
        let t3 = total_nsec(sec3, nsec3);

        let offset_min = t1 - tau2;
        let offset_max = t3 - tau2;
        let offset = (t1 + t3) / 2 - tau2;

        println!(
            "{}.{:09}, {}.{:09}, {}.{:09}, {:.9}, {:.9}, {:.9}",
            sec1, nsec1,
            sec2, nsec2,
            sec3, nsec3,
            nsec_to_sec(offset_min),
            nsec_to_sec(offset_max),
            nsec_to_sec(offset)
        );
    }
}

fn total_nsec(sec: i64, nsec: i64) -> i128 {
    const NANOSECONDS_IN_SECOND: i128 = 1000000000;

    sec as i128 * NANOSECONDS_IN_SECOND + nsec as i128
}

fn nsec_to_sec(nsec: i128) -> f64 {
    nsec as f64 * 1e-9
}

async fn send(socket: Arc<UdpSocket>, interval: Duration) -> Result<()> {
    loop {
        let (sec, nsec) = time_realtime()?;
        let payload = [sec.to_le_bytes(), nsec.to_le_bytes()].concat();
        assert_eq!(payload.len(), PAYLOAD_SIZE);

        socket.send(&payload[..]).await?;

        sleep(interval).await;
    }
}
