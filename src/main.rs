#![allow(dead_code)]

#[macro_use]
extern crate lazy_static;

mod dcb;
mod error;
mod opt;
mod prober;
mod utils;

use opt::Opt;
pub use structopt::StructOpt;

lazy_static! {
    static ref OPT: Opt = Opt::from_args();
}

fn main() {
    env_logger::builder()
        .filter_level(log::LevelFilter::Info)
        .init();

    log::info!("{:#?}", *OPT);
}
