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
use topo::Topo;
use tracerouter::Tracerouter;

lazy_static! {
    static ref OPT: Opt = if cfg!(test) {
        opt::get_test_opt()
    } else {
        opt::get_opt()
    };
}

fn init() {
    env_logger::builder()
        .filter_level(if OPT.debug {
            log::LevelFilter::Debug
        } else {
            log::LevelFilter::Info
        })
        .parse_default_env()
        .init();

    #[cfg(unix)]
    utils::ensure_su();

    log::info!("{:?}", *OPT);

    #[cfg(debug_assertions)]
    log::warn!(
        "{} is built in DEBUG mode, thus may perform quite poorly.",
        env!("CARGO_PKG_NAME")
    );
}

#[tokio::main]
async fn main() -> Result<()> {
    init();

    let tr = Arc::new(Tracerouter::new()?);
    let running = tr.clone();
    tokio::spawn(async move {
        tokio::signal::ctrl_c().await.unwrap();
        running.stop();
    });

    let topo = tr.run().await?;
    Topo::process_graph(topo).await?;

    #[cfg(windows)]
    std::process::exit(0);

    Ok(())
}
