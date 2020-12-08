#![allow(dead_code)]

#[macro_use]
extern crate lazy_static;

mod dcb;
mod error;
mod network;
mod opt;
mod prober;
mod topo;
mod tracerouter;
mod utils;

use std::sync::Arc;

use error::Result;
use opt::Opt;
use tracerouter::Tracerouter;
use utils::process_topo;

lazy_static! {
    static ref OPT: Opt = if cfg!(test) {
        opt::get_test_opt()
    } else {
        opt::get_opt()
    };
}

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::builder()
        .filter_level(log::LevelFilter::Info)
        .parse_default_env()
        .init();

    #[cfg(unix)]
    utils::ensure_su();

    log::debug!("{:#?}", *OPT);

    let tr = Arc::new(Tracerouter::new()?);
    let r = tr.clone();
    tokio::spawn(async move {
        tokio::signal::ctrl_c().await.unwrap();
        r.stop();
    });

    let topo = tr.run().await?;
    process_topo(topo).await?;

    #[cfg(windows)]
    std::process::exit(0);

    Ok(())
}
