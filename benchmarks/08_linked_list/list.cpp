// Benchmark 08 — Pointer-linked list, 1 000 000 nodes  (C++)
// Traditional heap-allocated linked list for comparison with the
// index-based approach in list.vani and list.c.
// Each node is a separate allocation → pointer chase → cache misses.
//
// g++ -O3 -march=native -std=c++17 -o list_cpp list.cpp && ./list_cpp
#include <cinttypes>
#include <cstdint>
#include <cstdio>
#include <memory>

struct Node {
    int64_t value;
    Node* next;
};

int main() {
    const int64_t N = 1000000LL;

    Node* nodes_pool = new Node[N];
    for (int64_t i = 0; i < N; i++) {
        nodes_pool[i].value = i % 1000;
        nodes_pool[i].next  = (i + 1 < N) ? &nodes_pool[i + 1] : nullptr;
    }
    Node* head = &nodes_pool[0];

    int64_t sum = 0;
    for (Node* curr = head; curr != nullptr; curr = curr->next)
        sum += curr->value;

    std::printf("%" PRId64 "\n", sum);
    delete[] nodes_pool;
}
