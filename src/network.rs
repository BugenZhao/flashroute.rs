use std::{
    net::IpAddr,
    sync::atomic::Ordering,
    sync::{
        atomic::{AtomicBool, AtomicU64},
        Arc,
    },
    time::Duration,
    time::SystemTime,
};

use crate::{
    error::*,
    prober::{ProbeResult, ProbeUnit, Prober},
    OPT,
};
use pnet::{
    packet::{
        ip::IpNextHeaderProtocols::{Icmp, Udp},
        Packet,
    },
    transport::{transport_channel, TransportChannelType::Layer3},
};
use tokio::sync::{mpsc, oneshot};
use Ordering::SeqCst;

type MpscTx<T> = mpsc::UnboundedSender<T>;
type MpscRx<T> = mpsc::UnboundedReceiver<T>;

type OneshotTx<T> = oneshot::Sender<T>;
type OneshotRx<T> = oneshot::Receiver<T>;

pub struct NetworkManager {
    prober: Arc<Prober>,
    sent_packets: Arc<AtomicU64>,
    recv_packets: Arc<AtomicU64>,
    send_tx: MpscTx<ProbeUnit>,
    stopped: Arc<AtomicBool>,
    stop_txs: Vec<OneshotTx<()>>,
}

impl NetworkManager {
    pub fn new(prober: Prober, recv_tx: MpscTx<ProbeResult>) -> Result<Self> {
        let (send_tx, send_rx) = mpsc::unbounded_channel();

        let prober = Arc::new(prober);
        let sent_packets = Arc::new(AtomicU64::new(0));
        let recv_packets = Arc::new(AtomicU64::new(0));
        let mut stop_txs = Vec::new();

        let (stop_tx, stop_rx) = oneshot::channel::<()>();
        stop_txs.push(stop_tx);
        Self::start_sending_task(prober.clone(), send_rx, stop_rx, sent_packets.clone())?;

        let stopped = Arc::new(AtomicBool::new(false));
        Self::start_recving_task(
            prober.clone(),
            stopped.clone(),
            recv_packets.clone(),
            recv_tx,
        )?;

        Ok(Self {
            prober,
            sent_packets,
            recv_packets,
            send_tx,
            stopped,
            stop_txs,
        })
    }

    const RECV_BUF_SIZE: usize = 400 * 1024;

    fn start_sending_task(
        prober: Arc<Prober>,
        mut rx: MpscRx<ProbeUnit>,
        mut stop_rx: OneshotRx<()>,
        sent_packets: Arc<AtomicU64>,
    ) -> Result<()> {
        let protocol = Layer3(Udp);
        let (mut sender, _) = transport_channel(0, protocol)?;
        let local_ip = OPT.local_addr;
        let _dummy_addr = IpAddr::V4("0.0.0.0".parse().unwrap());

        tokio::spawn(async move {
            log::info!("[{:?}] sending task started", prober.phase);

            let mut sent_this_sec = 0u64;
            let mut last_seen = SystemTime::now();

            let one_sec = Duration::from_secs(1);

            loop {
                tokio::select! {
                    _ = &mut stop_rx => {
                        break;
                    }
                    Some(dst_unit) = rx.recv() => {
                        // Probing rate control
                        let now = SystemTime::now();
                        let time_elapsed = now.duration_since(last_seen).unwrap();
                        if time_elapsed >= one_sec {
                            sent_this_sec = 0;
                            last_seen = now;
                        }
                        if sent_this_sec > (*OPT).probing_rate {
                            tokio::time::sleep(one_sec - time_elapsed).await;
                        }

                        let packet = prober.pack(dst_unit, local_ip);
                        let _ = sender.send_to(packet, IpAddr::V4(dst_unit.0));

                        log::debug!("PROBE: {:?}", dst_unit);

                        sent_packets.fetch_add(1, SeqCst);
                        sent_this_sec += 1;
                    }
                }
            }

            log::info!("[{:?}] sending task stopped", prober.phase);
        });

        Ok(())
    }

    fn start_recving_task(
        prober: Arc<Prober>,
        stopped: Arc<AtomicBool>,
        recv_packets: Arc<AtomicU64>,
        recv_tx: MpscTx<ProbeResult>,
    ) -> Result<()> {
        let protocol = Layer3(Icmp);
        let (_, mut receiver) = transport_channel(Self::RECV_BUF_SIZE, protocol)?;

        tokio::task::spawn_blocking(move || {
            // pnet io is synchronous, must be spawned with blocking
            log::info!("[{:?}] receiving task started", prober.phase);

            let io_timeout = Duration::from_millis(10);
            let mut iter = pnet::transport::ipv4_packet_iter(&mut receiver);

            loop {
                if stopped.load(SeqCst) {
                    break;
                }

                if let Ok(Some((ip_packet, _addr))) = iter.next_with_timeout(io_timeout) {
                    match prober.parse(ip_packet.packet(), false) {
                        Ok(result) => {
                            log::info!("[{:?}] RECV: {:?}", prober.phase, result);
                            let _ = recv_tx.send(result);
                            recv_packets.fetch_add(1, SeqCst);
                        }
                        Err(e) => {
                            log::warn!("error occurred while parsing: {}", e);
                        }
                    }
                }
            }

            log::info!("[{:?}] receiving task stopped", prober.phase);
        });

        Ok(())
    }

    pub fn schedule_probe(&self, unit: ProbeUnit) {
        match self.send_tx.send(unit) {
            Ok(_) => {
                log::debug!("SCHEDULE: {:?}", unit);
            }
            Err(e) => {
                log::error!("{:?}", e);
            }
        }
    }

    pub fn stop(&mut self) {
        self.stopped.store(true, SeqCst);
        for tx in self.stop_txs.drain(..) {
            let _ = tx.send(());
        }
    }

    pub fn sent_packets(&self) -> u64 {
        self.sent_packets.load(SeqCst)
    }

    pub fn recv_packets(&self) -> u64 {
        self.recv_packets.load(SeqCst)
    }
}
