use std::path::PathBuf;
use structopt::StructOpt;

use crate::utils;

#[derive(Debug, StructOpt)]
pub struct Opt {
    // Preprobing
    #[structopt(long, default_value = "32")]
    pub preprobing_ttl: u8,
    #[structopt(long, default_value = "5")]
    pub proximity_span: u32,

    // Probing
    #[structopt(long, default_value = "8")]
    pub default_ttl: u8,
    #[structopt(long, default_value = "16")]
    pub split_ttl: u8,
    #[structopt(long, default_value = "32")]
    pub max_ttl: u8,
    #[structopt(long, default_value = "5")]
    pub gap: u8,
    #[structopt(long, default_value = "400000")]
    pub probing_rate: u64,
    #[structopt(short = "2", help = "Send at least 2 probes to each target")]
    pub two: bool,

    // Connection
    #[structopt(long, parse(try_from_str = utils::get_interface), default_value = "")]
    pub interface: pnet::datalink::NetworkInterface,
    #[structopt(long, default_value = "33434")]
    pub dst_port: u16,
    #[structopt(long, default_value = "How are you?")]
    pub payload_message: String,

    // Output
    #[structopt(long = "no-plot", parse(from_flag = std::ops::Not::not))]
    pub plot: bool,
    #[structopt(long, default_value = "neato")]
    pub layout: String,
    #[structopt(short = "o", long, default_value = "fr.dot")]
    pub output_dot: PathBuf,
    #[structopt(short = "O", long, default_value = "fr.png")]
    pub output_viz: PathBuf,

    // Misc
    #[structopt(long, default_value = "114514")]
    pub seed: u64,
    #[structopt(long, default_value = "0")]
    pub salt: u16,
    #[structopt(long)]
    pub dry_run: bool,
    #[structopt(short = "D", long)]
    pub debug: bool,

    // Target
    #[structopt(short, long, default_value = "8")]
    pub grain: u8,
    #[structopt()]
    pub targets: ipnet::Ipv4Net,
    #[structopt(long)]
    pub global_only: bool,
    #[structopt(long)]
    pub allow_private: bool,

    // Generated
    #[structopt(skip = ("0.0.0.0".parse::<std::net::Ipv4Addr>().unwrap()))]
    pub local_addr: std::net::Ipv4Addr,
}

pub fn get_opt() -> Opt {
    let mut opt: Opt = Opt::from_args();
    opt.local_addr = crate::utils::get_interface_ipv4_addr(&opt.interface).unwrap();
    if opt.probing_rate == 0 {
        log::warn!("Probing rate is 0, rate limit will be turned off.");
        opt.probing_rate = u64::MAX;
    }
    opt
}

pub fn get_test_opt() -> Opt {
    let args = [env!("CARGO_PKG_NAME"), "192.168.1.1/24", "-g=8"];
    let mut opt: Opt = Opt::from_iter(args.iter());
    opt.local_addr = crate::utils::get_interface_ipv4_addr(&opt.interface).unwrap();
    opt
}
