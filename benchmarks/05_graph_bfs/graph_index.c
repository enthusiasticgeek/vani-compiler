/* Benchmark 05 — Graph BFS with integer index adjacency list  (C)
   Equivalent to graph.vani: no pointers, no ref-counting.
   gcc -O2 -o graph_c graph_index.c && ./graph_c               */
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#define N         1000
#define DEGREE    6
#define RUNS      1000

/* Adjacency list in CSR format (fixed degree). */
static int adj[N * DEGREE];   /* adj[v*DEGREE + e] = neighbour index */

static void build_graph(void) {
    int offsets[] = {1, 3, 7, 13, 29, 61};
    for (int v = 0; v < N; v++)
        for (int e = 0; e < DEGREE; e++)
            adj[v * DEGREE + e] = (v + offsets[e]) % N;
}

static int bfs(int start) {
    static char visited[N];
    static int  queue[N];
    memset(visited, 0, sizeof visited);
    int head = 0, tail = 0, count = 0;
    queue[tail++] = start;
    visited[start] = 1;
    while (head < tail) {
        int curr = queue[head++];
        count++;
        for (int e = 0; e < DEGREE; e++) {
            int nb = adj[curr * DEGREE + e];
            if (!visited[nb]) {
                visited[nb] = 1;
                queue[tail++] = nb;
            }
        }
    }
    return count;
}

int main(void) {
    build_graph();
    long total = 0;
    for (int run = 0; run < RUNS; run++)
        total += bfs(run % N);
    printf("%ld\n", total);
    return 0;
}
