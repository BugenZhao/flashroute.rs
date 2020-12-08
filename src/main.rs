#![allow(dead_code)]

#[macro_use]
extern crate lazy_static;

mod dcb;
mod error;
mod network;
mod opt;
mod prober;
mod tracerouter;
mod utils;

use std::sync::Arc;

use opt::Opt;
pub use structopt::StructOpt;
use tracerouter::Tracerouter;

lazy_static! {
    static ref OPT: Opt = if cfg!(test) {
        opt::get_test_opt()
    } else {
        opt::get_opt()
    };
}

#[tokio::main]
async fn main() {
    env_logger::builder()
        .filter_level(log::LevelFilter::Info)
        .parse_default_env()
        .init();

    log::debug!("{:#?}", *OPT);

    let tr = Arc::new(Tracerouter::new().unwrap());
    let r = tr.clone();

    tokio::spawn(async move {
        tokio::signal::ctrl_c().await.unwrap();
        r.stop();
    });

    tr.run().await.unwrap();

    #[cfg(windows)]
    std::process::exit(0);
}
