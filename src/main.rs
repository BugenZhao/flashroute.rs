#![allow(dead_code)]

mod error;
mod opt;
mod utils;
mod dcb;

use opt::Opt;
pub use structopt::StructOpt;

fn main() {
    let opt: Opt = Opt::from_args();
    println!("{:#?}", opt);
}
