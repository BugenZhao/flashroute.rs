use std::{
    collections::{HashMap, HashSet},
    net::Ipv4Addr,
    sync::atomic::AtomicU64,
    sync::{atomic::Ordering, Arc, RwLock},
};

use ipnet::IpAdd;
use rand::{rngs::StdRng, Rng, SeedableRng};
use tokio::sync::mpsc;
use Ordering::SeqCst;

use crate::{
    dcb::DstCtrlBlock, error::*, network::NetworkManager, prober::ProbePhase, prober::ProbeResult,
    prober::Prober, OPT,
};

type MpscTx<T> = mpsc::UnboundedSender<T>;
type MpscRx<T> = mpsc::UnboundedReceiver<T>;

type AddrKey = i64;
type DcbMap = HashMap<AddrKey, DstCtrlBlock>;

#[derive(Debug)]
pub struct Tracerouter {
    targets: Arc<RwLock<DcbMap>>,

    // stats
    preprobe_update_count: Arc<AtomicU64>,
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
            targets: Arc::new(RwLock::new(targets)),
            preprobe_update_count: Arc::new(AtomicU64::new(0)),
        })
    }

    fn addr_to_key(addr: Ipv4Addr) -> AddrKey {
        let u: u32 = addr.into();
        (u >> (OPT.grain)) as AddrKey
    }

    fn targets_count() -> usize {
        1 << ((OPT.target.max_prefix_len() - OPT.target.prefix_len()) - OPT.grain)
    }

    fn random_targets() -> impl Iterator<Item = Ipv4Addr> {
        let mut rng = StdRng::seed_from_u64(OPT.seed);
        let subnets = OPT
            .target
            .subnets(OPT.target.max_prefix_len() - OPT.grain)
            .unwrap();

        subnets.map(move |net| net.addr().saturating_add(rng.gen_range(1, 1 << OPT.grain)))
    }
}

impl Tracerouter {
    fn start_preprobing_task(&mut self) {
        let prober = Prober::new(ProbePhase::Pre, true, 0);
        let (recv_tx, recv_rx) = mpsc::unbounded_channel();
        let nm = NetworkManager::new(prober, recv_tx);

        // TODO:
    }

    fn preprobing_callback(targets: Arc<RwLock<DcbMap>>, result: ProbeResult) {
        if !result.from_destination {
            return;
        }

        let key = Self::addr_to_key(result.destination);
        if let Some(dcb) = targets.read().unwrap().get(&key) {
            dcb.update_split_ttl(result.distance, true);

            // proximity
            let lo = 0.max(key - OPT.proximity_span as AddrKey);
            let hi = key + OPT.proximity_span as AddrKey;
            for n_key in lo..hi {
                if n_key == key {
                    continue;
                }
                if let Some(dcb) = targets.read().unwrap().get(&n_key) {
                    dcb.update_split_ttl(result.distance, false);
                }
            }
        }
    }

    fn probing_callback(
        targets: Arc<RwLock<DcbMap>>,
        backward_stop_set: &mut HashSet<Ipv4Addr>,
        forward_discovery_set: &mut HashSet<Ipv4Addr>,
        result: ProbeResult,
    ) {
        let key = Self::addr_to_key(result.destination);
        if let Some(dcb) = targets.write().unwrap().get_mut(&key) {
            if !result.from_destination {
                // hosts on the path
                if result.distance > dcb.initial_ttl.load(SeqCst) {
                    // o-o-o-S-o-X-o-D
                    forward_discovery_set.insert(result.responder);
                    if result.distance <= dcb.last_forward_task() {
                        // reasonable distance, update horizon
                        dcb.set_forward_horizon((result.distance + OPT.gap).min(OPT.max_ttl));
                    }
                } else {
                    // o-X-o-S-o-o-o-D
                    let new = backward_stop_set.insert(result.responder);
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
            tr.targets.read().unwrap().len(),
            1 << (32 - OPT.target.prefix_len() - OPT.grain)
        );
        assert!(tr
            .targets
            .read()
            .unwrap()
            .values()
            .all(|dcb| OPT.target.contains(&dcb.addr)));
    }
}
