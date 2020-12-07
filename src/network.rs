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
}

impl NetworkManager {
    pub fn new(prober: Prober, recv_tx: MpscTx<ProbeResult>) -> Option<Self> {
        let (send_tx, send_rx) = mpsc::unbounded_channel();

        let prober = Arc::new(prober);
        let sent_packets = Arc::new(AtomicU64::new(0));
        let recv_packets = Arc::new(AtomicU64::new(0));
        let stopped = Arc::new(AtomicBool::new(false));

        Self::start_sending_task(
            prober.clone(),
            send_rx,
            stopped.clone(),
            sent_packets.clone(),
        );
        Self::start_recving_task(
            prober.clone(),
            stopped.clone(),
            recv_packets.clone(),
            recv_tx,
        );

        Some(Self {
            prober,
            sent_packets,
            recv_packets,
            send_tx,
            stopped,
        })
    }

    const RECV_BUF_SIZE: usize = 400 * 1024;

    fn start_sending_task(
        prober: Arc<Prober>,
        mut rx: MpscRx<ProbeUnit>,
        stopped: Arc<AtomicBool>,
        sent_packets: Arc<AtomicU64>,
    ) {
        tokio::spawn(async move {
            log::info!("sending task started");

            let protocol = Layer3(Udp);
            let (mut sender, _) = transport_channel(0, protocol).unwrap();
            let local_ip = OPT.local_addr;
            let dummy_addr = IpAddr::V4("0.0.0.0".parse().unwrap());

            let mut sent_this_sec = 0u64;
            let mut last_seen = SystemTime::now();

            let one_sec = Duration::new(1, 0);
            let timeout = Duration::new(0, 100_000_000);

            loop {
                if let Ok(Some(dst_unit)) = tokio::time::timeout(timeout, rx.recv()).await {
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
                    let _ = sender.send_to(packet, dummy_addr);

                    sent_packets.fetch_add(1, Ordering::SeqCst);
                    sent_this_sec += 1;
                }

                if stopped.load(Ordering::Acquire) {
                    break;
                }
            }

            log::info!("sending task stopped");
        });
    }

    fn start_recving_task(
        prober: Arc<Prober>,
        stopped: Arc<AtomicBool>,
        recv_packets: Arc<AtomicU64>,
        recv_tx: MpscTx<ProbeResult>,
    ) {
        tokio::spawn(async move {
            log::info!("receiving task started");

            let protocol = Layer3(Icmp);
            let (_, mut receiver) = transport_channel(Self::RECV_BUF_SIZE, protocol).unwrap();
            let mut iter = pnet::transport::ipv4_packet_iter(&mut receiver);

            let timeout = Duration::new(0, 100_000_000);

            loop {
                if let Ok(Some((ip_packet, _addr))) = iter.next_with_timeout(timeout) {
                    match prober.parse(ip_packet.packet(), false) {
                        Ok(result) => {
                            let _ = recv_tx.send(result);
                            recv_packets.fetch_add(1, Ordering::SeqCst);
                        }
                        Err(e) => {
                            log::warn!("error occurred while parsing: {}", e);
                        }
                    }
                }

                if stopped.load(Ordering::Acquire) {
                    break;
                }
            }

            log::info!("receiving task stopped");
        });
    }

    pub fn schedule_probe(&self, unit: ProbeUnit) {
        let _ = self.send_tx.send(unit);
    }

    pub fn stop(&mut self) {
        self.stopped.store(true, Ordering::SeqCst);
    }
}
