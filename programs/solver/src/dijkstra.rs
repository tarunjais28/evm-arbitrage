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
    amount: U256,
}

impl Ord for State {
    fn cmp(&self, other: &Self) -> Ordering {
        self.amount.cmp(&other.amount)
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
    cost: U256,
}

impl ShortestPath {
    fn new(paths: Vec<String>, cost: U256) -> Self {
        Self { paths, cost }
    }
}

fn best_path(graph: &SwapGraph, start: &str, end: &str) -> ShortestPath {
    let mut heap = BinaryHeap::new();
    let mut best = HashMap::new();
    let mut predecessor: HashMap<String, String> = HashMap::new();

    heap.push(State {
        token: start.to_string(),
        amount: U256::ZERO,
    });
    best.insert(start.to_string(), U256::ZERO);

    while let Some(State { token, amount }) = heap.pop() {
        if amount < *best.get(&token).unwrap_or(&U256::ZERO) {
            continue;
        }

        if token == end {
            let mut path = vec![end.to_string()];
            let mut current = end.to_string();
            while let Some(prev) = predecessor.get(&current) {
                path.push(prev.clone());
                current = prev.clone();
            }
            path.reverse();
            return ShortestPath::new(path, amount);
        }

        if let Some(neighbors) = graph.get(&token) {
            for edge in neighbors {
                let new_amount = amount + edge.slippage;
                if new_amount > *best.get(&edge.to).unwrap_or(&U256::ZERO) {
                    best.insert(edge.to.clone(), new_amount);
                    predecessor.insert(edge.to.clone(), token.clone());
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
            vec!["A".to_string(), "C".to_string(), "D".to_string()]
        );
        assert_eq!(path.cost, U256::from(30));
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
            vec!["A".to_string(), "B".to_string(), "D".to_string()]
        );
        assert_eq!(path.cost, U256::from(15));
    }
}
