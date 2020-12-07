use std::{
    collections::{HashMap, HashSet},
    net::Ipv4Addr,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::{Duration, SystemTime},
};

use ipnet::IpAdd;
use rand::{rngs::StdRng, Rng, SeedableRng};
use tokio::sync::{mpsc, oneshot};
use Ordering::SeqCst;

use crate::{
    dcb::DstCtrlBlock, error::*, network::NetworkManager, prober::ProbePhase, prober::ProbeResult,
    prober::Prober, OPT,
};

type MpscTx<T> = mpsc::UnboundedSender<T>;
type MpscRx<T> = mpsc::UnboundedReceiver<T>;

type AddrKey = i64;
type DcbMap = HashMap<AddrKey, DstCtrlBlock>;

#[derive(Debug, Default)]
pub struct Tracerouter {
    targets: Arc<DcbMap>,
    stopped: Arc<AtomicBool>,

    // stats
    // preprobe_update_count: Arc<AtomicU64>,
    sent_preprobes: u64,
    sent_probes: u64,
    recv_responses: u64,
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
            targets: Arc::new(targets),
            ..Self::default()
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

    fn stopped(&self) -> bool {
        self.stopped.load(SeqCst)
    }
}

impl Tracerouter {
    fn start_preprobing_task(&mut self) {
        let prober = Prober::new(ProbePhase::Pre, true, 0);
        let (recv_tx, mut recv_rx) = mpsc::unbounded_channel();
        let nm = NetworkManager::new(prober, recv_tx);
        let (stop_tx, stop_rx) = oneshot::channel::<()>();

        let targets = self.targets.clone();
        tokio::spawn(async move {
            tokio::select! {
                Some(result) = recv_rx.recv() => {
                    Self::preprobing_callback(&targets, result);
                }
                _ = stop_rx => {
                    return;
                }
            };
        });

        // WORKER BEGIN
        for target in self.targets.values() {
            if self.stopped() {
                break;
            }
            nm.schedule_probe((target.addr, OPT.preprobing_ttl));
        }
        // WORKER END

        if !self.stopped.load(SeqCst) {
            std::thread::sleep(Duration::from_secs(3));
        }
        nm.stop();
        let _ = stop_tx.send(());

        self.sent_preprobes += nm.sent_packets();
        self.recv_responses += nm.recv_packets();
    }

    fn preprobing_callback(targets: &DcbMap, result: ProbeResult) {
        if !result.from_destination {
            return;
        }

        let key = Self::addr_to_key(result.destination);
        if let Some(dcb) = targets.get(&key) {
            dcb.update_split_ttl(result.distance, true);

            // proximity
            let lo = 0.max(key - OPT.proximity_span as AddrKey);
            let hi = key + OPT.proximity_span as AddrKey;
            for n_key in lo..hi {
                if n_key == key {
                    continue;
                }
                if let Some(dcb) = targets.get(&n_key) {
                    dcb.update_split_ttl(result.distance, false);
                }
            }
        }
    }
}

impl Tracerouter {
    fn start_probing_task(&mut self) {
        let prober = Prober::new(ProbePhase::Main, true, 0);
        let (recv_tx, mut recv_rx) = mpsc::unbounded_channel();
        let nm = NetworkManager::new(prober, recv_tx);
        let (stop_tx, stop_rx) = oneshot::channel::<()>();

        let targets = self.targets.clone();
        let mut backward_stop_set = HashSet::<Ipv4Addr>::new();
        let mut forward_discovery_set = HashSet::<Ipv4Addr>::new();

        tokio::spawn(async move {
            tokio::select! {
                Some(result) = recv_rx.recv() => {
                    Self::probing_callback(&targets, &mut backward_stop_set, &mut forward_discovery_set, result);
                }
                _ = stop_rx => {
                    return;
                }
            };
        });

        // WORKER BEGIN
        let mut keys: Vec<_> = self.targets.keys().cloned().collect();
        let mut last_seen = SystemTime::now();
        let one_sec = Duration::from_secs(1);

        while !keys.is_empty() {
            let mut new_keys = Vec::new();
            new_keys.reserve(keys.len());
            for key in keys.into_iter() {
                if self.stopped() {
                    break;
                }
                let dcb = self.targets.get(&key).unwrap();
                match (dcb.pull_forward_task(), dcb.pull_backward_task()) {
                    (None, None) => {
                        continue;
                    }
                    (None, Some(t2)) => {
                        nm.schedule_probe((dcb.addr, t2));
                    }
                    (Some(t1), None) => {
                        nm.schedule_probe((dcb.addr, t1));
                    }
                    (Some(t1), Some(t2)) => {
                        nm.schedule_probe((dcb.addr, t1));
                        nm.schedule_probe((dcb.addr, t2));
                    }
                }
                new_keys.push(key);
            }
            keys = new_keys;

            let duration = SystemTime::now().duration_since(last_seen).unwrap();
            if duration < one_sec {
                std::thread::sleep(one_sec - duration);
            }
            last_seen = SystemTime::now();
        }
        // WORKER END

        nm.stop();
        let _ = stop_tx.send(());

        self.sent_probes += nm.sent_packets();
        self.recv_responses += nm.recv_packets();
    }

    fn probing_callback(
        targets: &DcbMap,
        backward_stop_set: &mut HashSet<Ipv4Addr>,
        forward_discovery_set: &mut HashSet<Ipv4Addr>,
        result: ProbeResult,
    ) {
        let key = Self::addr_to_key(result.destination);
        if let Some(dcb) = targets.get(&key) {
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
            tr.targets.len(),
            1 << (32 - OPT.target.prefix_len() - OPT.grain)
        );
        assert!(tr
            .targets
            .values()
            .all(|dcb| OPT.target.contains(&dcb.addr)));
    }
}
