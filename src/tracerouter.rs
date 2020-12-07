use std::{
    collections::{HashMap, HashSet},
    net::Ipv4Addr,
};

use ipnet::IpAdd;
use rand::{rngs::StdRng, Rng, SeedableRng};

use crate::{dcb::DstCtrlBlock, error::*, prober::ProbeResult, OPT};

type AddrKey = i64;
type DcbMap = HashMap<AddrKey, DstCtrlBlock>;

#[derive(Default, Debug)]
pub struct Tracerouter {
    targets: DcbMap,
    backward_stop_set: HashSet<Ipv4Addr>,
    forward_discovery_set: HashSet<Ipv4Addr>,

    // stat
    preprobe_update_count: u64,
}

impl Tracerouter {
    pub fn new() -> Result<Self> {
        if OPT.grain > (OPT.target.max_prefix_len() - OPT.target.prefix_len()) {
            return Err(Error::BadGrainOrNet(OPT.grain, OPT.target));
        }

        let mut targets = DcbMap::new();
        targets.reserve(Self::targets_count());
        for addr in Self::random_targets() {
            targets.insert(Self::addr_to_key(addr), DstCtrlBlock::new(addr, 8));
        }

        Ok(Self {
            targets,
            ..Self::default()
        })
    }

    fn addr_to_key(addr: Ipv4Addr) -> AddrKey {
        let u: u32 = addr.into();
        (u >> (OPT.grain)) as AddrKey
    }

    pub fn targets_count() -> usize {
        1 << ((OPT.target.max_prefix_len() - OPT.target.prefix_len()) - OPT.grain)
    }

    pub fn random_targets() -> impl Iterator<Item = Ipv4Addr> {
        let mut rng = StdRng::seed_from_u64(OPT.seed);
        let subnets = OPT
            .target
            .subnets(OPT.target.max_prefix_len() - OPT.grain)
            .unwrap();

        subnets.map(move |net| net.addr().saturating_add(rng.gen_range(1, 1 << OPT.grain)))
    }

    fn preprobing_callback(&mut self, result: ProbeResult) {
        if !result.from_destination {
            return;
        }

        let key = Self::addr_to_key(result.destination);
        if let Some(dcb) = self.targets.get_mut(&key) {
            dcb.update_split_ttl(result.distance, true);
            self.preprobe_update_count += 1;
            // proximity
            let lo = 0.max(key - OPT.proximity_span as AddrKey);
            let hi = key + OPT.proximity_span as AddrKey;
            for n_key in lo..hi {
                if n_key == key {
                    continue;
                }
                if let Some(dcb) = self.targets.get_mut(&n_key) {
                    dcb.update_split_ttl(result.distance, false);
                    self.preprobe_update_count += 1;
                }
            }
        }
    }

    fn probing_callback(&mut self, result: ProbeResult) {
        let key = Self::addr_to_key(result.destination);
        if let Some(dcb) = self.targets.get_mut(&key) {
            if !result.from_destination {
                // hosts on the path
                if result.distance > dcb.initial_ttl {
                    // o-o-o-S-o-X-o-D
                    self.forward_discovery_set.insert(result.responder);
                    if result.distance <= dcb.last_forward_task() {
                        // reasonable distance, update horizon
                        dcb.set_forward_horizon((result.distance + OPT.gap).min(OPT.max_ttl));
                    }
                } else {
                    // o-X-o-S-o-o-o-D
                    let new = self.backward_stop_set.insert(result.responder);
                    if !new {
                        dcb.stop_backward();
                    }
                }
            } else {
                // from destination
                dcb.stop_forward();
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_generation() {
        let tr = Tracerouter::new().unwrap();
        assert_eq!(
            tr.targets.len(),
            1 << (32 - OPT.target.prefix_len() - OPT.grain)
        );
        assert!(tr
            .targets
            .values()
            .all(|dcb| OPT.target.contains(&dcb.addr)));
    }
}
