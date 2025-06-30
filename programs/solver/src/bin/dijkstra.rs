use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashMap};

#[derive(Debug, Clone)]
struct SwapEdge {
    to: String,
    slippage: u64,
}

type SwapGraph = HashMap<String, Vec<SwapEdge>>;

#[derive(Debug, Eq)]
struct State {
    token: String,
    cost: u64,
    path: Vec<String>,
}

impl Ord for State {
    fn cmp(&self, other: &Self) -> Ordering {
        // Notice: Reverse comparison to make min-heap
        other.cost.cmp(&self.cost)
    }
}

impl PartialOrd for State {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for State {
    fn eq(&self, other: &Self) -> bool {
        self.token == other.token && self.cost == other.cost
    }
}

#[derive(Debug, Default)]
struct ShortestPath {
    paths: Vec<String>,
    cost: u64,
}

impl ShortestPath {
    fn new(paths: Vec<String>, cost: u64) -> Self {
        Self { paths, cost }
    }
}

fn build_bidirectional_graph(edges: &[(String, String, u64)]) -> SwapGraph {
    let mut graph = SwapGraph::new();
    for (from, to, slippage) in edges {
        graph.entry(from.clone()).or_default().push(SwapEdge {
            to: to.clone(),
            slippage: *slippage,
        });
        graph.entry(to.clone()).or_default().push(SwapEdge {
            to: from.clone(),
            slippage: *slippage,
        });
    }
    graph
}

fn best_path(graph: &SwapGraph, start: &str, end: &str) -> ShortestPath {
    let mut heap = BinaryHeap::new();
    let mut best_cost = HashMap::new();

    heap.push(State {
        token: start.to_string(),
        cost: 0,
        path: vec![start.to_string()],
    });

    while let Some(State { token, cost, path }) = heap.pop() {
        if token == end {
            return ShortestPath::new(path, cost);
        }

        if cost > *best_cost.get(&token).unwrap_or(&u64::MAX) {
            continue;
        }

        if let Some(neighbors) = graph.get(&token) {
            for edge in neighbors {
                let new_cost = cost + edge.slippage;
                if new_cost < *best_cost.get(&edge.to).unwrap_or(&u64::MAX) {
                    let mut new_path = path.clone();
                    new_path.push(edge.to.clone());

                    best_cost.insert(edge.to.clone(), new_cost);

                    heap.push(State {
                        token: edge.to.clone(),
                        cost: new_cost,
                        path: new_path,
                    });
                }
            }
        }
    }

    ShortestPath::default()
}

fn main() {
    // Define your edges as (from, to, slippage)
    let edges = vec![
        ("ETH".to_string(), "USDC".to_string(), 15),
        ("ETH".to_string(), "DAI".to_string(), 2),
        ("USDC".to_string(), "WBTC".to_string(), 3),
        ("DAI".to_string(), "WBTC".to_string(), 6),
    ];

    let graph = build_bidirectional_graph(&edges);

    let best_output = best_path(&graph, "ETH", "WBTC");
    println!(
        "Best path: {:?}, cost: {}",
        best_output.paths, best_output.cost
    );
}
