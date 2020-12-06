#![allow(dead_code)]

mod dcb;
mod error;
mod opt;
mod prober;
mod utils;

use opt::Opt;
pub use structopt::StructOpt;

fn main() {
    env_logger::builder()
        .filter_level(log::LevelFilter::Info)
        .init();

    let opt: Opt = Opt::from_args();
    log::info!("{:#?}", opt);
}
