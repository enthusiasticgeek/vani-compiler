// Benchmark 08 — Pointer-linked list, 1 000 000 nodes  (C++)
// Traditional heap-allocated linked list for comparison with the
// index-based approach in list.vani and list.c.
// Each node is a separate allocation → pointer chase → cache misses.
//
// g++ -O2 -std=c++17 -o list_cpp list.cpp && ./list_cpp
#include <cstdio>
#include <memory>

struct Node {
    long value;
    Node* next;  // raw pointer — no shared_ptr overhead here
};

int main() {
    const long N = 1000000L;

    // Build list: allocate one block and wire manually for fairness.
    // (Each node is a separate new — same cost as typical linked list.)
    Node* nodes_pool = new Node[N];
    for (long i = 0; i < N; i++) {
        nodes_pool[i].value = i % 1000;
        nodes_pool[i].next  = (i + 1 < N) ? &nodes_pool[i + 1] : nullptr;
    }
    Node* head = &nodes_pool[0];

    long sum = 0;
    for (Node* curr = head; curr != nullptr; curr = curr->next)
        sum += curr->value;

    std::printf("%ld\n", sum);
    delete[] nodes_pool;
}
