mod error;
mod opt;
mod utils;

use opt::Opt;
pub use structopt::StructOpt;

fn main() {
    let opt: Opt = Opt::from_args();
    println!("{:#?}", opt);
}
