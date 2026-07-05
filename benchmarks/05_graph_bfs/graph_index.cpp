// Benchmark 05 — Graph BFS with integer index adjacency list  (C++)
// This is the equivalent of graph.vani: std::vector<int> indices,
// NO shared_ptr / weak_ptr.  Nodes sit in one contiguous buffer.
//
// g++ -O3 -march=native -std=c++17 -o graph_idx_cpp graph_index.cpp && ./graph_idx_cpp
#include <cinttypes>
#include <cstdint>
#include <cstdio>
#include <vector>

static const int N      = 1000;
static const int DEGREE = 6;
static const int RUNS   = 1000;

// Fixed offsets — same as graph.vani and graph_index.c
static const int OFFSETS[DEGREE] = {1, 3, 7, 13, 29, 61};

struct Node {
    int id;
    std::vector<int> neighbors; // integer indices — no ref-counting
    int parent;                  // -1 = no parent
};

static int bfs(const std::vector<Node>& nodes, int start) {
    std::vector<bool> visited(N, false);
    std::vector<int>  queue;
    queue.reserve(N);
    queue.push_back(start);
    visited[start] = true;
    int head = 0, count = 0;
    while (head < (int)queue.size()) {
        int curr = queue[head++];
        count++;
        for (int nb : nodes[curr].neighbors) {
            if (!visited[nb]) {
                visited[nb] = true;
                queue.push_back(nb);
            }
        }
    }
    return count;
}

int main() {
    std::vector<Node> nodes(N);
    for (int v = 0; v < N; v++) {
        nodes[v].id     = v;
        nodes[v].parent = (v == 0) ? -1 : (v - 1);
        for (int e = 0; e < DEGREE; e++)
            nodes[v].neighbors.push_back((v + OFFSETS[e]) % N);
    }

    int64_t total = 0;
    for (int run = 0; run < RUNS; run++)
        total += bfs(nodes, run % N);

    std::printf("%" PRId64 "\n", total);
}
