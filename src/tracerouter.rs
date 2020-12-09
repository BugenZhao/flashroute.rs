use std::{
    collections::{HashMap, HashSet},
    net::Ipv4Addr,
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        Arc,
    },
    time::{Duration, SystemTime},
};

use ipnet::IpAdd;
use rand::{rngs::StdRng, Rng, SeedableRng};
use tokio::sync::{mpsc, oneshot};
use Ordering::SeqCst;

use crate::{
    dcb::DstCtrlBlock,
    error::*,
    network::NetworkManager,
    prober::ProbePhase,
    prober::ProbeResult,
    prober::Prober,
    topo::{Topo, TopoGraph, TopoReq},
    OPT,
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
    sent_preprobes: AtomicU64,
    sent_probes: AtomicU64,
    recv_responses: AtomicU64,
}

impl Tracerouter {
    pub fn new() -> Result<Self> {
        if OPT.grain > (OPT.target.max_prefix_len() - OPT.target.prefix_len()) {
            return Err(Error::BadGrainOrNet(OPT.grain, OPT.target));
        }

        let mut targets = DcbMap::new();
        targets.reserve(Self::targets_count());
        for addr in Self::random_targets() {
            targets.insert(
                Self::addr_to_key(addr),
                DstCtrlBlock::new(addr, OPT.default_ttl),
            );
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

        subnets.map(move |net| {
            if OPT.grain == 0 {
                net.addr()
            } else {
                net.addr().saturating_add(rng.gen_range(1, 1 << OPT.grain))
            }
        })
    }
}

impl Tracerouter {
    pub async fn run(&self) -> Result<TopoGraph> {
        let _ = self.run_preprobing_task().await?;
        let topo = self.run_probing_task().await?;
        self.summary();
        Ok(topo)
    }

    pub fn stop(&self) {
        self.stopped.store(true, SeqCst);
    }

    pub fn summary(&self) {
        log::info!(
            "sent preprobes: {:?}, sent probes: {:?}, received responses: {:?}",
            self.sent_preprobes,
            self.sent_probes,
            self.recv_responses
        );
    }

    fn stopped(&self) -> bool {
        self.stopped.load(SeqCst)
    }
}

impl Tracerouter {
    async fn run_preprobing_task(&self) -> Result<()> {
        let prober = Prober::new(ProbePhase::Pre, true);
        let (recv_tx, mut recv_rx) = mpsc::unbounded_channel();
        let mut nm = NetworkManager::new(prober, recv_tx)?;
        let (stop_tx, mut stop_rx) = oneshot::channel::<()>();

        let targets = self.targets.clone();
        tokio::spawn(async move {
            loop {
                tokio::select! {
                    Some(result) = recv_rx.recv() => {
                        Self::preprobing_callback(&targets, result);
                    }
                    _ = &mut stop_rx => {
                        return;
                    }
                };
            }
        });

        // WORKER BEGIN
        for target in self.targets.values() {
            if self.stopped() {
                break;
            }
            nm.schedule_probe((target.addr, OPT.preprobing_ttl));
        }
        // WORKER END

        if !self.stopped() {
            log::info!("[Pre] Waiting...");
            tokio::time::sleep(Duration::from_secs(3)).await;
        }
        nm.stop();
        let _ = stop_tx.send(());

        self.sent_preprobes.fetch_add(nm.sent_packets(), SeqCst);
        self.recv_responses.fetch_add(nm.recv_packets(), SeqCst);

        Ok(())
    }

    fn preprobing_callback(targets: &DcbMap, result: ProbeResult) {
        if !result.from_destination {
            return;
        }
        log::debug!("[Pre] CALLBACK: {}", result.destination);

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
    async fn run_probing_task(&self) -> Result<TopoGraph> {
        let prober = Prober::new(ProbePhase::Main, true);
        let (recv_tx, mut recv_rx) = mpsc::unbounded_channel();
        let mut nm = NetworkManager::new(prober, recv_tx)?;
        let (stop_tx, mut stop_rx) = oneshot::channel::<()>();

        let targets = self.targets.clone();
        let mut backward_stop_set = HashSet::<Ipv4Addr>::new();
        let mut forward_discovery_set = HashSet::<Ipv4Addr>::new();

        let (topo_tx, topo_rx) = mpsc::unbounded_channel();
        let cb_topo_tx = topo_tx.clone();

        let topo_task = tokio::spawn(async move { Topo::new(topo_rx).run() });

        tokio::spawn(async move {
            loop {
                tokio::select! {
                    Some(result) = recv_rx.recv() => {
                        Self::probing_callback(&targets, &mut backward_stop_set, &mut forward_discovery_set, &result);
                        let _ = cb_topo_tx.send(TopoReq::Result(result));
                    }
                    _ = &mut stop_rx => {
                        let _ = cb_topo_tx.send(TopoReq::Stop);
                        return;
                    }
                };
            }
        });

        // WORKER BEGIN
        let mut keys: Vec<_> = self.targets.keys().cloned().collect();
        let mut last_seen = SystemTime::now();
        let one_sec = Duration::from_secs(1);

        while !keys.is_empty() {
            let mut new_keys = Vec::new();
            new_keys.reserve(keys.len());

            log::debug!("[Main] loop");
            for key in keys.into_iter() {
                if self.stopped() {
                    break;
                }
                let dcb = self.targets.get(&key).unwrap();
                match (dcb.pull_forward_task(), dcb.pull_backward_task()) {
                    (None, None) => {
                        log::debug!("{} is done!", dcb.addr);
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
            let min_round_duration = one_sec.min(Duration::from_millis(keys.len() as u64 * 20));
            if duration < min_round_duration {
                tokio::time::sleep(min_round_duration - duration).await;
            }
            last_seen = SystemTime::now();
        }
        // WORKER END

        if !self.stopped() {
            log::info!("[Main] Waiting...");
            tokio::time::sleep(Duration::from_secs(5)).await;
        }
        nm.stop();
        let _ = stop_tx.send(());

        self.sent_probes.fetch_add(nm.sent_packets(), SeqCst);
        self.recv_responses.fetch_add(nm.recv_packets(), SeqCst);

        Ok(topo_task.await.unwrap().await)
    }

    fn probing_callback(
        targets: &DcbMap,
        backward_stop_set: &mut HashSet<Ipv4Addr>,
        forward_discovery_set: &mut HashSet<Ipv4Addr>,
        result: &ProbeResult,
    ) {
        log::debug!("[Main] CALLBACK: {}", result.destination);

        let key = Self::addr_to_key(result.destination);
        if let Some(dcb) = targets.get(&key) {
            if !result.from_destination {
                // hosts on the path
                if result.distance > dcb.initial_ttl() {
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
                        log::debug!("STOP for {}", dcb.addr);
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
