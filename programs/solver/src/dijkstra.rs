use super::*;

#[derive(Debug, Clone)]
struct SwapEdge {
    to: String,
    slippage: U256,
}

type SwapGraph = HashMap<String, Vec<SwapEdge>>;

#[derive(Debug, Eq)]
struct State {
    token: String,
    cost: U256,
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
    cost: U256,
}

impl ShortestPath {
    fn new(paths: Vec<String>, cost: U256) -> Self {
        Self { paths, cost }
    }
}

fn build_bidirectional_graph(edges: &[(String, String, U256)]) -> SwapGraph {
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
        cost: U256::ZERO,
        path: vec![start.to_string()],
    });

    while let Some(State { token, cost, path }) = heap.pop() {
        if token == end {
            return ShortestPath::new(path, cost);
        }

        if cost > *best_cost.get(&token).unwrap_or(&U256::MAX) {
            continue;
        }

        if let Some(neighbors) = graph.get(&token) {
            for edge in neighbors {
                let new_cost = cost + edge.slippage;
                if new_cost < *best_cost.get(&edge.to).unwrap_or(&U256::MAX) {
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

#[cfg(test)]
mod tests {
    use super::*;

    fn create_graph() -> SwapGraph {
        let mut graph = HashMap::new();
        graph.insert(
            "A".to_string(),
            vec![
                SwapEdge {
                    to: "B".to_string(),
                    slippage: U256::from(10),
                },
                SwapEdge {
                    to: "C".to_string(),
                    slippage: U256::from(20),
                },
            ],
        );
        graph.insert(
            "B".to_string(),
            vec![SwapEdge {
                to: "D".to_string(),
                slippage: U256::from(5),
            }],
        );
        graph.insert(
            "C".to_string(),
            vec![SwapEdge {
                to: "D".to_string(),
                slippage: U256::from(10),
            }],
        );
        graph
    }

    #[test]
    fn test_single_depth_path() {
        let graph = create_graph();
        let path = best_path(&graph, "A", "B");
        assert_eq!(path.paths, vec!["A".to_string(), "B".to_string()]);
        assert_eq!(path.cost, U256::from(10));
    }

    #[test]
    fn test_multi_depth_path() {
        let graph = create_graph();
        let path = best_path(&graph, "A", "D");
        assert_eq!(
            path.paths,
            vec!["A".to_string(), "B".to_string(), "D".to_string()]
        );
        assert_eq!(path.cost, U256::from(15));
    }

    #[test]
    fn test_no_path() {
        let graph = create_graph();
        let path = best_path(&graph, "A", "E");
        assert!(path.paths.is_empty());
        assert_eq!(path.cost, U256::ZERO);
    }

    #[test]
    fn test_complex_path() {
        let mut graph = create_graph();
        graph.insert(
            "A".to_string(),
            vec![
                SwapEdge {
                    to: "B".to_string(),
                    slippage: U256::from(10),
                },
                SwapEdge {
                    to: "C".to_string(),
                    slippage: U256::from(5),
                },
            ],
        );
        graph.insert(
            "C".to_string(),
            vec![SwapEdge {
                to: "D".to_string(),
                slippage: U256::from(5),
            }],
        );
        let path = best_path(&graph, "A", "D");
        assert_eq!(
            path.paths,
            vec!["A".to_string(), "C".to_string(), "D".to_string()]
        );
        assert_eq!(path.cost, U256::from(10));
    }

    #[test]
    fn test_bidirectional_path() {
        let edges = vec![
            ("A".to_string(), "B".to_string(), U256::from(10)),
            ("A".to_string(), "C".to_string(), U256::from(20)),
            ("B".to_string(), "D".to_string(), U256::from(5)),
            ("C".to_string(), "D".to_string(), U256::from(10)),
        ];
        let graph = build_bidirectional_graph(&edges);

        // Test forward path
        let path_forward = best_path(&graph, "A", "D");
        assert_eq!(
            path_forward.paths,
            vec!["A".to_string(), "B".to_string(), "D".to_string()]
        );
        assert_eq!(path_forward.cost, U256::from(15));

        // Test reverse path
        let path_reverse = best_path(&graph, "D", "A");
        assert_eq!(
            path_reverse.paths,
            vec!["D".to_string(), "B".to_string(), "A".to_string()]
        );
        assert_eq!(path_reverse.cost, U256::from(15));
    }

    #[test]
    fn test_bidirectional_complex_path() {
        let edges = vec![
            ("A".to_string(), "B".to_string(), U256::from(10)),
            ("A".to_string(), "C".to_string(), U256::from(5)), // A-C is cheaper
            ("B".to_string(), "D".to_string(), U256::from(5)),
            ("C".to_string(), "D".to_string(), U256::from(5)), // C-D is cheaper
            ("B".to_string(), "E".to_string(), U256::from(2)),
            ("D".to_string(), "E".to_string(), U256::from(8)),
        ];
        let graph = build_bidirectional_graph(&edges);

        // Test forward path A -> E
        // A -> C -> D -> E : 5 + 5 + 8 = 18
        // A -> B -> E : 10 + 2 = 12
        // A -> B -> D -> E : 10 + 5 + 8 = 23
        let path_forward = best_path(&graph, "A", "E");
        assert_eq!(
            path_forward.paths,
            vec!["A".to_string(), "B".to_string(), "E".to_string()]
        );
        assert_eq!(path_forward.cost, U256::from(12));

        // Test reverse path E -> A
        // E -> B -> A : 2 + 10 = 12
        // E -> D -> C -> A : 8 + 5 + 5 = 18
        let path_reverse = best_path(&graph, "E", "A");
        assert_eq!(
            path_reverse.paths,
            vec!["E".to_string(), "B".to_string(), "A".to_string()]
        );
        assert_eq!(path_reverse.cost, U256::from(12));
    }
}
