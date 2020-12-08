use std::{
    net::{IpAddr, Ipv4Addr},
    time::{SystemTime, UNIX_EPOCH},
};

use petgraph::dot::Dot;
use pnet::datalink::NetworkInterface;
use tokio::io::AsyncWriteExt;

use crate::{error::*, topo::TopoGraph, OPT};

pub fn get_interface_ipv4_addr(ni: &NetworkInterface) -> Option<Ipv4Addr> {
    for ip in ni.ips.iter().map(|net| net.ip()) {
        if let IpAddr::V4(ipv4) = ip {
            return Some(ipv4);
        }
    }
    None
}

pub fn get_interface(name: &str) -> Result<NetworkInterface> {
    let interfaces = pnet::datalink::interfaces();

    if name.is_empty() {
        interfaces
            .into_iter()
            .filter(|ni| ni.is_up() && !ni.is_loopback() && !ni.ips.is_empty())
            .next()
            .ok_or(Error::NoSuchInterface(name.to_owned()))
    } else {
        interfaces
            .into_iter()
            .filter(|ni| ni.name == name)
            .next()
            .ok_or(Error::NoSuchInterface(name.to_owned()))
    }
}

pub fn timestamp_ms_u16() -> u16 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as u16
}

pub fn ip_checksum(addr: Ipv4Addr, salt: u16) -> u16 {
    pnet::util::checksum(&addr.octets(), 0) + salt
}

pub async fn process_topo(topo: TopoGraph) -> Result<()> {
    let dot = Dot::new(&topo);
    log::debug!("{}", dot);

    let dot_path = OPT.output_dot.to_str().unwrap();
    let viz_path = OPT.output_viz.to_str().unwrap();

    let mut dot_file = tokio::fs::File::create(dot_path).await?;
    dot_file.write_all(format!("{}", dot).as_bytes()).await?;

    tokio::process::Command::new("dot")
        .arg("-Tpng")
        .arg(dot_path)
        .arg("-o")
        .arg(viz_path)
        .spawn()?
        .wait()
        .await?;

    Ok(())
}
