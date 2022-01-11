use clap::Parser;

/// UDP-based naive clock offset measurement tool
#[derive(Parser, Debug)]
struct Args {
    /// Stream timestamps to (host:port)
    address: Option<String>,

    /// Port to listen for incoming timestamps on
    #[clap(short, long)]
    port: Option<u16>,

    /// Timestamp sending rate
    #[clap(short, long, default_value_t = 1.0)]
    rate: f64
}

fn main() {
    let args = Args::parse();

    if let Some(ref address) = args.address {
        println!("Peer address: {}", address);
    }

    if let Some(port) = args.port {
        println!("Port to listen on: {}", port);
    }
}
