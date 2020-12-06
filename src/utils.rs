use pnet::datalink::NetworkInterface;

use crate::error::*;

pub fn get_interface(name: &str) -> Result<NetworkInterface> {
    let interfaces = pnet::datalink::interfaces();

    if interfaces.is_empty() {
        Err(Error::NoSuchInterface)
    } else if name.is_empty() {
        // TODO: use more reasonable interface
        Ok(interfaces[0].clone())
    } else {
        interfaces
            .into_iter()
            .filter(|ni| ni.name == name)
            .next()
            .ok_or(Error::NoSuchInterface)
    }
}
