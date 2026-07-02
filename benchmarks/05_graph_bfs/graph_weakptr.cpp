// Benchmark 05 — Graph BFS with shared_ptr children + weak_ptr parent  (C++)
//
// This is the IDIOMATIC C++ solution when you need a cyclic graph:
//   • children held via shared_ptr<Node>  (forward ownership)
//   • parent  held via weak_ptr<Node>     (back-edge, avoids ref-count cycle)
//
// Each node is a SEPARATE heap allocation; accessing a parent costs
// a weak_ptr::lock() — at least 2 atomic operations — plus a temporary
// shared_ptr that increments / decrements the ref-count on creation/destruction.
//
// Compare with graph_index.cpp (and graph.vani) to see the speedup from
// the index-handle approach.
//
// g++ -O2 -std=c++17 -o graph_weak_cpp graph_weakptr.cpp && ./graph_weak_cpp
#include <cstdio>
#include <memory>
#include <vector>
#include <queue>

static const int N      = 1000;
static const int DEGREE = 6;
static const int RUNS   = 1000;
static const int OFFSETS[DEGREE] = {1, 3, 7, 13, 29, 61};

struct Node {
    int id;
    std::vector<std::shared_ptr<Node>> children; // forward edges — shared ownership
    std::weak_ptr<Node>                parent;   // back-edge — weak to break cycle
};

static int bfs(const std::shared_ptr<Node>& root) {
    std::vector<bool> visited(N, false);
    std::queue<std::shared_ptr<Node>> q;
    q.push(root);
    visited[root->id] = true;
    int count = 0;
    while (!q.empty()) {
        auto node = q.front(); q.pop();
        count++;
        for (auto& child : node->children) {
            if (!visited[child->id]) {
                visited[child->id] = true;
                q.push(child);  // shared_ptr copy: atomic refcount++
            }
        }
        // Accessing back-edge costs a lock() — 2+ atomic ops.
        // auto par = node->parent.lock();
        // (not used in BFS, but the node carries it in memory)
    }
    return count;
}

int main() {
    // Allocate all nodes.
    std::vector<std::shared_ptr<Node>> nodes(N);
    for (int v = 0; v < N; v++) {
        nodes[v] = std::make_shared<Node>();
        nodes[v]->id = v;
    }
    // Wire edges and parents.
    for (int v = 0; v < N; v++) {
        for (int e = 0; e < DEGREE; e++) {
            int nb = (v + OFFSETS[e]) % N;
            nodes[v]->children.push_back(nodes[nb]); // shared_ptr copy
        }
        if (v > 0)
            nodes[v]->parent = nodes[v - 1]; // weak_ptr assignment
    }

    long total = 0;
    for (int run = 0; run < RUNS; run++)
        total += bfs(nodes[run % N]);

    std::printf("%ld\n", total);
    // Nodes are freed here; shared_ptr ref-counts drop to zero in cascade.
}
