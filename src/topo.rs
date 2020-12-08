use std::{collections::HashMap, net::Ipv4Addr};

use petgraph::graphmap::UnGraphMap;
use tokio::sync::mpsc;

use crate::{prober::ProbeDebugResult, prober::ProbeResult, OPT};

type MpscTx<T> = mpsc::UnboundedSender<T>;
type MpscRx<T> = mpsc::UnboundedReceiver<T>;

pub type TopoGraph = UnGraphMap<Ipv4Addr, u8>;

pub enum TopoReq {
    Result(ProbeResult),
    Complete(Ipv4Addr),
    Stop,
}

pub struct Topo {
    req_rx: MpscRx<TopoReq>,
    results_buf: HashMap<Ipv4Addr, Vec<ProbeResult>>,
    graph: TopoGraph,
}

impl Topo {
    pub fn new(req_rx: MpscRx<TopoReq>) -> Self {
        let mut graph = UnGraphMap::new();
        graph.add_node(OPT.local_addr);
        Self {
            req_rx,
            results_buf: HashMap::new(),
            graph,
        }
    }

    pub async fn run(mut self) -> TopoGraph {
        let dummy = [ProbeResult {
            destination: OPT.local_addr,
            responder: OPT.local_addr,
            distance: 0,
            from_destination: true,
            debug: ProbeDebugResult::default(),
        }];

        let process = |mut results: Vec<ProbeResult>, graph: &mut TopoGraph| {
            results.sort_by_key(|r| r.distance);
            let it = dummy.iter().chain(results.iter());
            for (a, b) in it.clone().zip(it.skip(1)) {
                let dist = b.distance - a.distance;
                if dist > 8 {
                    continue;
                }
                graph.add_node(b.responder);
                graph.add_edge(a.responder, b.responder, dist);
            }
        };

        while let Some(req) = self.req_rx.recv().await {
            match req {
                TopoReq::Result(result) => {
                    self.results_buf
                        .entry(result.destination)
                        .or_insert(Vec::new())
                        .push(result);
                }
                TopoReq::Complete(addr) => {
                    if let Some(results) = self.results_buf.remove(&addr) {
                        process(results, &mut self.graph);
                    }
                }
                TopoReq::Stop => {
                    for (_, results) in self.results_buf.drain() {
                        process(results, &mut self.graph);
                    }
                    break;
                }
            }
        }

        self.graph
    }
}
