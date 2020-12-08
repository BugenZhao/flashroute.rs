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

    // Connection
    #[structopt(long, parse(try_from_str = utils::get_interface), default_value = "")]
    pub interface: pnet::datalink::NetworkInterface,
    #[structopt(long, default_value = "33434")]
    pub dst_port: u16,
    #[structopt(long, default_value = "53")]
    pub src_port: u16,
    #[structopt(long, default_value = "How are you?")]
    pub payload_message: String,

    // Output
    #[structopt(short, long, default_value = "fr.out")]
    pub output: PathBuf,

    // Misc
    #[structopt(long, default_value = "114514")]
    pub seed: u64,
    #[structopt(long, default_value = "0")]
    pub salt: u16,

    // Target
    #[structopt(default_value = "115.159.0.0/16")]
    pub target: ipnet::Ipv4Net,
    #[structopt(long, default_value = "8")]
    pub grain: u8,

    // Generated
    #[structopt(skip = ("0.0.0.0".parse::<std::net::Ipv4Addr>().unwrap()))]
    pub local_addr: std::net::Ipv4Addr,
}

pub fn get_opt() -> Opt {
    let mut opt: Opt = Opt::from_args();
    opt.local_addr = crate::utils::get_interface_ipv4_addr(&opt.interface).unwrap();
    opt
}

pub fn get_test_opt() -> Opt {
    let args: Vec<String> = vec![];
    let mut opt: Opt = Opt::from_iter(args);
    opt.local_addr = crate::utils::get_interface_ipv4_addr(&opt.interface).unwrap();
    opt
}
