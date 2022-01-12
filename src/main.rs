use anyhow::{Context, Result};
use clap::Parser;
use std::net::{Ipv4Addr, SocketAddrV4};
use tokio::{
    net::UdpSocket,
    time::{sleep, Duration}
};

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

    let payload = b"Hello!\n";

    loop {
        socket.send(&payload[..]).await?;
        sleep(Duration::from_secs_f64(interval)).await;
    }

    Ok(())
}

async fn receive(port: u16) -> Result<()> {
    eprintln!("Listening for timestamps on port {}...", port);

    let socket = UdpSocket::bind(SocketAddrV4::new(Ipv4Addr::new(0, 0, 0, 0), port)).await?;
    let mut buf = [0; 1024];
    loop {
        let (len, addr) = socket.recv_from(&mut buf).await?;
        println!("{}", String::from_utf8_lossy(&buf[..len]));
    }

    Ok(())
}
