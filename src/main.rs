#![allow(dead_code)]

#[macro_use]
extern crate lazy_static;

mod dcb;
mod error;
mod network;
mod opt;
mod prober;
mod utils;

use opt::Opt;
pub use structopt::StructOpt;

lazy_static! {
    static ref OPT: Opt = Opt::from_args();
    static ref LOCAL_IPV4_ADDR: std::net::Ipv4Addr =
        crate::utils::get_interface_ipv4_addr(&(*OPT).interface).unwrap();
}

fn main() {
    env_logger::builder()
        .filter_level(log::LevelFilter::Info)
        .init();

    log::info!("{:#?}", *OPT);
}
