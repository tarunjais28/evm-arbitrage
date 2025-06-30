use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashMap};

#[derive(Debug, Clone)]
struct SwapEdge {
    to: String,
    slippage: u64, // e.g., tokenA → tokenB exchange rate
}

type SwapGraph = HashMap<String, Vec<SwapEdge>>;

#[derive(Debug, Eq)]
struct State {
    token: String,
    amount: u64,
}

impl Ord for State {
    fn cmp(&self, other: &Self) -> Ordering {
        other.amount.partial_cmp(&self.amount).unwrap()
    }
}

impl PartialOrd for State {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for State {
    fn eq(&self, other: &Self) -> bool {
        self.token == other.token && self.amount == other.amount
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

fn best_path(graph: &SwapGraph, start: &str, end: &str) -> ShortestPath {
    let mut heap = BinaryHeap::new();
    let mut best = HashMap::new();
    let mut paths = Vec::with_capacity(graph.len());

    heap.push(State {
        token: start.to_string(),
        amount: 0,
    });

    while let Some(State { token, amount }) = heap.pop() {
        paths.push(token.to_string());

        if token == end {
            return ShortestPath::new(paths, amount);
        }

        if let Some(neighbors) = graph.get(&token) {
            for edge in neighbors {
                let new_amount = amount + edge.slippage;
                if new_amount > *best.get(&edge.to).unwrap_or(&0) {
                    best.insert(edge.to.clone(), new_amount);
                    heap.push(State {
                        token: edge.to.clone(),
                        amount: new_amount,
                    });
                }
            }
        }
    }

    ShortestPath::default()
}

fn main() {
    let mut graph = SwapGraph::new();
    graph.insert(
        "ETH".to_string(),
        vec![
            SwapEdge {
                to: "USDC".to_string(),
                slippage: 15,
            },
            SwapEdge {
                to: "DAI".to_string(),
                slippage: 2,
            },
        ],
    );
    graph.insert(
        "USDC".to_string(),
        vec![SwapEdge {
            to: "WBTC".to_string(),
            slippage: 3,
        }],
    );
    graph.insert(
        "DAI".to_string(),
        vec![SwapEdge {
            to: "WBTC".to_string(),
            slippage: 6,
        }],
    );

    let best_output = best_path(&graph, "ETH", "WBTC");
    println!("{:?} -> {}", best_output.paths, best_output.cost);
}
