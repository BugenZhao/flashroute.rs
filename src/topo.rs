use std::{collections::HashMap, net::Ipv4Addr};

use petgraph::{dot::Dot, graphmap::UnGraphMap};
use tokio::io::AsyncWriteExt;
use tokio::sync::mpsc;

use crate::error::*;
use crate::{prober::ProbeDebugResult, prober::ProbeResult, OPT};

type MpscTx<T> = mpsc::UnboundedSender<T>;
type MpscRx<T> = mpsc::UnboundedReceiver<T>;

pub type TopoGraph = UnGraphMap<Ipv4Addr, u8>;

pub enum TopoReq {
    Result(ProbeResult),
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
        let local = ProbeResult {
            destination: OPT.local_addr,
            responder: OPT.local_addr,
            distance: 0,
            from_destination: true,
            debug: ProbeDebugResult::default(),
        };

        let process = |mut results: Vec<ProbeResult>, graph: &mut TopoGraph| {
            results.sort_by_key(|r| r.distance);
            if let Some(first) = results.first() {
                let dist = first.distance;
                graph.add_node(first.responder);
                if dist <= 1 {
                    graph.add_edge(local.responder, first.responder, dist);
                }
            }
            for (a, b) in results.iter().zip(results.iter().skip(1)) {
                if a.responder != b.responder {
                    let dist = b.distance - a.distance;
                    graph.add_node(b.responder);
                    graph.add_edge(a.responder, b.responder, dist);
                }
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
                TopoReq::Stop => {
                    for (_, results) in self.results_buf {
                        process(results, &mut self.graph);
                    }
                    break;
                }
            }
        }

        self.graph
    }

    pub async fn process_graph(topo_graph: TopoGraph) -> Result<()> {
        log::info!("[Summary] Total probed hosts: {}", topo_graph.node_count());

        if OPT.dot {
            let dot_content =
                Dot::with_config(&topo_graph, &[petgraph::dot::Config::GraphContentOnly]);

            let dot_path = OPT.output_dot.to_str().unwrap();
            let viz_path = OPT.output_viz.to_str().unwrap();
            let mut dot_file = tokio::fs::File::create(dot_path).await?;

            macro_rules! write {
                ($str:expr) => {
                    dot_file.write($str.as_bytes()).await?;
                };
            }

            log::info!("Saving topology to {}...", dot_path);
            write!("graph {\n    overlap = false;\n");
            if OPT.spline {
                write!("    splines = true;\n");
            }
            for s in format!("{}", dot_content).lines() {
                write!(s);
                write!("\n");
            }
            write!("}\n");

            if OPT.plot {
                log::info!("Plotting to {}...", viz_path);
                tokio::process::Command::new("dot")
                    .arg("-K")
                    .arg(OPT.layout.as_str())
                    .arg("-Tpng")
                    .arg(dot_path)
                    .arg("-o")
                    .arg(viz_path)
                    .spawn()?
                    .wait()
                    .await?;
            }
        }

        Ok(())
    }
}
