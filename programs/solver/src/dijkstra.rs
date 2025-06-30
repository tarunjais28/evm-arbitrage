use super::*;

#[derive(Debug, Clone)]
pub struct SwapEdge {
    pub to: Address,
    pub pool: Address,
    pub slippage: U256,
}

impl SwapEdge {
    pub fn new(to: Address, pool: Address, slippage: U256) -> Self {
        Self { to, pool, slippage }
    }
}

pub type SwapGraph = HashMap<Address, Vec<SwapEdge>>;

#[derive(Debug, Eq)]
pub struct State {
    pub token: Address,
    pub cost: U256,
    pub paths: Vec<Address>,
    pub pools: Vec<Address>,
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
    pub cost: U256,
}

impl ShortestPath {
    pub fn new(paths: Vec<Address>, pools: Vec<Address>, cost: U256) -> Self {
        Self { paths, pools, cost }
    }
}

pub fn build_bidirectional_graph(edges: &[(Address, Address, Address, U256)]) -> SwapGraph {
    let mut graph = SwapGraph::new();
    for (from, to, pool, slippage) in edges {
        graph.entry(from.clone()).or_default().push(SwapEdge::new(
            to.clone(),
            pool.clone(),
            *slippage,
        ));
        graph.entry(to.clone()).or_default().push(SwapEdge::new(
            from.clone(),
            pool.clone(),
            *slippage,
        ));
    }
    graph
}

pub fn best_path(graph: &SwapGraph, start: &Address, end: &Address) -> ShortestPath {
    let mut heap = BinaryHeap::new();
    let mut best_cost = HashMap::new();

    heap.push(State {
        token: start.clone(),
        cost: U256::ZERO,
        paths: vec![start.clone()],
        pools: vec![],
    });

    while let Some(State {
        token,
        cost,
        paths,
        pools,
    }) = heap.pop()
    {
        if &token == end {
            return ShortestPath::new(paths, pools, cost);
        }

        if cost > *best_cost.get(&token).unwrap_or(&U256::MAX) {
            continue;
        }

        if let Some(neighbors) = graph.get(&token) {
            for edge in neighbors {
                let new_cost = cost + edge.slippage;
                if new_cost < *best_cost.get(&edge.to).unwrap_or(&U256::MAX) {
                    let mut new_paths = paths.clone();
                    new_paths.push(edge.to.clone());

                    let mut new_pools = pools.clone();
                    new_pools.push(edge.pool.clone());

                    best_cost.insert(edge.to.clone(), new_cost);

                    heap.push(State {
                        token: edge.to.clone(),
                        cost: new_cost,
                        paths: new_paths,
                        pools: new_pools,
                    });
                }
            }
        }
    }

    ShortestPath::default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloy::primitives::address;

    pub fn create_graph() -> SwapGraph {
        let mut graph = HashMap::new();
        let a = address!("000000000000000000000000000000000000000A");
        let b = address!("000000000000000000000000000000000000000B");
        let c = address!("000000000000000000000000000000000000000C");
        let d = address!("000000000000000000000000000000000000000D");
        let p_a_b = address!("00000000000000000000000000000000000000AB");
        let p_a_c = address!("00000000000000000000000000000000000000AC");
        let p_b_d = address!("00000000000000000000000000000000000000BD");
        let p_c_d = address!("00000000000000000000000000000000000000CD");

        graph.insert(
            a,
            vec![
                SwapEdge::new(b, p_a_b, U256::from(10)),
                SwapEdge::new(c, p_a_c, U256::from(20)),
            ],
        );
        graph.insert(b, vec![SwapEdge::new(d, p_b_d, U256::from(5))]);
        graph.insert(c, vec![SwapEdge::new(d, p_c_d, U256::from(10))]);
        graph
    }

    #[test]
    pub fn test_single_depth_path() {
        let graph = create_graph();
        let a = address!("000000000000000000000000000000000000000A");
        let b = address!("000000000000000000000000000000000000000B");
        let path = best_path(&graph, &a, &b);
        assert_eq!(path.paths, vec![a, b]);
        assert_eq!(path.cost, U256::from(10));
    }

    #[test]
    pub fn test_multi_depth_path() {
        let graph = create_graph();
        let a = address!("000000000000000000000000000000000000000A");
        let b = address!("000000000000000000000000000000000000000B");
        let d = address!("000000000000000000000000000000000000000D");
        let path = best_path(&graph, &a, &d);
        assert_eq!(path.paths, vec![a, b, d]);
        assert_eq!(path.cost, U256::from(15));
    }

    #[test]
    pub fn test_no_path() {
        let graph = create_graph();
        let a = address!("000000000000000000000000000000000000000A");
        let e = address!("000000000000000000000000000000000000000E");
        let path = best_path(&graph, &a, &e);
        assert!(path.paths.is_empty());
        assert_eq!(path.cost, U256::ZERO);
    }

    #[test]
    pub fn test_complex_path() {
        let mut graph = create_graph();
        let a = address!("000000000000000000000000000000000000000A");
        let b = address!("000000000000000000000000000000000000000B");
        let c = address!("000000000000000000000000000000000000000C");
        let d = address!("000000000000000000000000000000000000000D");
        let p_a_b = address!("00000000000000000000000000000000000000AB");
        let p_a_c = address!("00000000000000000000000000000000000000AC");
        let p_c_d = address!("00000000000000000000000000000000000000CD");
        graph.insert(
            a,
            vec![
                SwapEdge::new(b, p_a_b, U256::from(10)),
                SwapEdge::new(c, p_a_c, U256::from(5)),
            ],
        );
        graph.insert(c, vec![SwapEdge::new(d, p_c_d, U256::from(5))]);
        let path = best_path(&graph, &a, &d);
        assert_eq!(path.paths, vec![a, c, d]);
        assert_eq!(path.cost, U256::from(10));
    }

    #[test]
    pub fn test_bidirectional_path() {
        let a = address!("000000000000000000000000000000000000000A");
        let b = address!("000000000000000000000000000000000000000B");
        let c = address!("000000000000000000000000000000000000000C");
        let d = address!("000000000000000000000000000000000000000D");
        let p_a_b = address!("00000000000000000000000000000000000000AB");
        let p_a_c = address!("00000000000000000000000000000000000000AC");
        let p_b_d = address!("00000000000000000000000000000000000000BD");
        let p_c_d = address!("00000000000000000000000000000000000000CD");
        let edges = vec![
            (a, b, p_a_b, U256::from(10)),
            (a, c, p_a_c, U256::from(20)),
            (b, d, p_b_d, U256::from(5)),
            (c, d, p_c_d, U256::from(10)),
        ];
        let graph = build_bidirectional_graph(&edges);

        // Test forward path
        let path_forward = best_path(&graph, &a, &d);
        assert_eq!(path_forward.paths, vec![a, b, d]);
        assert_eq!(path_forward.cost, U256::from(15));

        // Test reverse path
        let path_reverse = best_path(&graph, &d, &a);
        assert_eq!(path_reverse.paths, vec![d, b, a]);
        assert_eq!(path_reverse.cost, U256::from(15));
    }

    #[test]
    pub fn test_bidirectional_complex_path() {
        let a = address!("000000000000000000000000000000000000000A");
        let b = address!("000000000000000000000000000000000000000B");
        let c = address!("000000000000000000000000000000000000000C");
        let d = address!("000000000000000000000000000000000000000D");
        let e = address!("000000000000000000000000000000000000000E");
        let p_a_b = address!("00000000000000000000000000000000000000AB");
        let p_a_c = address!("00000000000000000000000000000000000000AC");
        let p_b_d = address!("00000000000000000000000000000000000000BD");
        let p_c_d = address!("00000000000000000000000000000000000000CD");
        let p_b_e = address!("00000000000000000000000000000000000000BE");
        let p_d_e = address!("00000000000000000000000000000000000000DE");
        let edges = vec![
            (a, b, p_a_b, U256::from(10)),
            (a, c, p_a_c, U256::from(5)), // A-C is cheaper
            (b, d, p_b_d, U256::from(5)),
            (c, d, p_c_d, U256::from(5)), // C-D is cheaper
            (b, e, p_b_e, U256::from(2)),
            (d, e, p_d_e, U256::from(8)),
        ];
        let graph = build_bidirectional_graph(&edges);

        // Test forward path A -> E
        // A -> C -> D -> E : 5 + 5 + 8 = 18
        // A -> B -> E : 10 + 2 = 12
        // A -> B -> D -> E : 10 + 5 + 8 = 23
        let path_forward = best_path(&graph, &a, &e);
        assert_eq!(path_forward.paths, vec![a, b, e]);
        assert_eq!(path_forward.cost, U256::from(12));

        // Test reverse path E -> A
        // E -> B -> A : 2 + 10 = 12
        // E -> D -> C -> A : 8 + 5 + 5 = 18
        let path_reverse = best_path(&graph, &e, &a);
        assert_eq!(path_reverse.paths, vec![e, b, a]);
        assert_eq!(path_reverse.cost, U256::from(12));
    }
}
