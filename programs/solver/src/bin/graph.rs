use alloy::primitives::{address, Address};
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashMap};
use std::fs::File;
use std::io::BufReader;

type SwapGraph = HashMap<Address, Vec<SwapEdge>>;

#[derive(Debug, Eq)]
pub struct State {
    pub token: Address,
    pub cost: i64,
    pub paths: Vec<Address>,
    pub pools: Vec<Address>,
    pub fees: Vec<u16>,
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
pub struct ShortestPath {
    pub paths: Vec<Address>,
    pub pools: Vec<Address>,
    pub fees: Vec<u16>,
    pub cost: i64,
}

impl ShortestPath {
    pub fn new(paths: Vec<Address>, pools: Vec<Address>, fees: Vec<u16>, cost: i64) -> Self {
        Self {
            paths,
            pools,
            fees,
            cost,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwapEdge {
    pub to: Address,
    pub pool: Address,
    pub slippage: i64,
    pub fee: u16,
}

pub fn best_path(graph: &SwapGraph, start: &Address, end: &Address) -> ShortestPath {
    let mut heap = BinaryHeap::new();
    let mut best_cost = HashMap::new();

    heap.push(State {
        token: *start,
        cost: 0,
        paths: vec![*start],
        pools: vec![],
        fees: vec![],
    });

    while let Some(State {
        token,
        cost,
        paths,
        pools,
        fees,
    }) = heap.pop()
    {
        if &token == end {
            return ShortestPath::new(paths, pools, fees, cost);
        }

        if cost > *best_cost.get(&token).unwrap_or(&i64::MAX) {
            continue;
        }

        if let Some(neighbors) = graph.get(&token) {
            for edge in neighbors {
                let new_cost = cost + edge.slippage;
                if new_cost < *best_cost.get(&edge.to).unwrap_or(&i64::MAX) {
                    let mut new_paths = paths.clone();
                    new_paths.push(edge.to);

                    let mut new_pools = pools.clone();
                    new_pools.push(edge.pool);

                    let mut new_fees = fees.clone();
                    new_fees.push(edge.fee);

                    best_cost.insert(edge.to, new_cost);

                    heap.push(State {
                        token: edge.to,
                        cost: new_cost,
                        paths: new_paths,
                        pools: new_pools,
                        fees: new_fees,
                    });
                }
            }
        }
    }

    ShortestPath::default()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Dummy {
    pub address: Address,
    pub swap_edge: Vec<SwapEdge>,
}

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    let file = File::open("resources/graph.json")?;
    let reader = BufReader::new(file);
    let dummy: Vec<Dummy> = serde_json::from_reader(reader)?;
    let mut graph: HashMap<Address, Vec<SwapEdge>> = dummy
        .iter()
        .map(|d| (d.address, d.swap_edge.clone()))
        .collect();

    graph.iter_mut().for_each(|(_, data)| {
        data.iter_mut().for_each(|d| {
            d.slippage = d.slippage.abs();
        })
    });
    println!("Successfully loaded graph with {} tokens", graph.len());
    println!("{:#?}", graph);

    let path = best_path(
        &graph,
        &address!("0xa0b86991c6218b36c1d19d4a2e9eb0ce3606eb48"),
        &address!("0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2"),
    );

    println!("{:#?}", path);

    Ok(())
}
