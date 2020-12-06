use std::path::PathBuf;

use structopt::StructOpt;

#[derive(Debug, StructOpt)]
pub struct Opt {
    // Preprobing
    #[structopt(long, default_value = "32")]
    preprobing_ttl: u16,
    #[structopt(long, default_value = "5")]
    proximity_span: u32,

    // Probing
    #[structopt(long, default_value = "16")]
    split_ttl: u16,
    #[structopt(long, default_value = "400000")]
    probing_rate: u32,
    #[structopt(long, default_value = "How are you?")]
    payload_message: String,
    #[structopt(long, default_value = "33434")]
    dst_port: u16,
    #[structopt(long, default_value = "53")]
    src_port: u16,

    // Output
    #[structopt(short, long, default_value = "fr.out")]
    output: PathBuf,

    // Misc
    #[structopt(long, default_value = "114514")]
    seed: i32,

    // Target
    #[structopt(default_value = "115.159.1.64")]
    target: String,
}
