mod opt;

use opt::Opt;
pub use structopt::StructOpt;

fn main() {
    println!("{:#?}", Opt::from_args());
}
