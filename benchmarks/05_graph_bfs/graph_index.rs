// Benchmark 05 — Graph BFS with usize index adjacency list  (Rust)
// Equivalent to graph.vani: uses usize indices instead of Rc<Weak<T>>.
// Rust's borrow checker naturally encourages this pattern for cyclic graphs.
//
// rustc -C opt-level=2 -o graph_idx_rs graph_index.rs && ./graph_idx_rs

const N: usize = 1000;
const DEGREE: usize = 6;
const RUNS: usize = 1000;
const OFFSETS: [usize; DEGREE] = [1, 3, 7, 13, 29, 61];

struct Graph {
    adj: Vec<Vec<usize>>,  // adj[v] = list of neighbour indices
    parent: Vec<Option<usize>>,
}

impl Graph {
    fn build() -> Self {
        let mut adj = vec![Vec::new(); N];
        let mut parent = vec![None; N];
        for v in 0..N {
            for &off in &OFFSETS {
                adj[v].push((v + off) % N);
            }
            if v > 0 { parent[v] = Some(v - 1); }
        }
        Graph { adj, parent }
    }

    fn bfs(&self, start: usize) -> usize {
        let mut visited = vec![false; N];
        let mut queue = Vec::with_capacity(N);
        queue.push(start);
        visited[start] = true;
        let mut head = 0;
        let mut count = 0;
        while head < queue.len() {
            let curr = queue[head]; head += 1; count += 1;
            for &nb in &self.adj[curr] {
                if !visited[nb] {
                    visited[nb] = true;
                    queue.push(nb);
                }
            }
        }
        count
    }
}

fn main() {
    let g = Graph::build();
    let total: usize = (0..RUNS).map(|r| g.bfs(r % N)).sum();
    println!("{}", total);
}
