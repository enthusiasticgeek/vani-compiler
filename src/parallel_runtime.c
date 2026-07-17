/* vani_parallel_runtime.c — thread pool for vāṇī parallel-for.
 *
 * Replaces per-invocation CreateThread (Windows) / GOMP_parallel (POSIX)
 * with persistent pthreads workers.  Workers sleep between tasks, so
 * parallel-for overhead drops from O(thread-create) to O(condvar-signal).
 *
 * The pool is initialized once (pthread_once) with INTENT_POOL_THREADS
 * workers (default 4).  intent_pool_run() submits a task to all workers
 * and blocks until every worker finishes.  The outlined parallel-for
 * function receives (ctx, tid, nth) directly as parameters — no struct
 * unpacking needed.
 */
#include <pthread.h>
#include <stdint.h>
#include <stdlib.h>

#ifndef INTENT_POOL_THREADS
#define INTENT_POOL_THREADS 4
#endif

typedef void (*intent_par_fn)(void *ctx, int64_t tid, int64_t nth);

typedef struct {
    pthread_t       thread;
    pthread_mutex_t mu;
    pthread_cond_t  cond;
    intent_par_fn   fn;
    void           *ctx;
    int64_t         tid;
    int64_t         nth;
    int             pending;
    int             stop;
} Worker;

static Worker          pool[INTENT_POOL_THREADS];
static pthread_once_t  pool_once = PTHREAD_ONCE_INIT;

static void *worker_main(void *arg) {
    Worker *w = (Worker *)arg;
    pthread_mutex_lock(&w->mu);
    for (;;) {
        while (!w->pending && !w->stop)
            pthread_cond_wait(&w->cond, &w->mu);
        if (w->stop) { pthread_mutex_unlock(&w->mu); return NULL; }
        intent_par_fn fn  = w->fn;
        void         *ctx = w->ctx;
        int64_t       tid = w->tid;
        int64_t       nth = w->nth;
        pthread_mutex_unlock(&w->mu);

        fn(ctx, tid, nth);

        pthread_mutex_lock(&w->mu);
        w->pending = 0;
        pthread_cond_signal(&w->cond);
    }
}

static void pool_init(void) {
    for (int i = 0; i < INTENT_POOL_THREADS; i++) {
        Worker *w = &pool[i];
        pthread_mutex_init(&w->mu, NULL);
        pthread_cond_init(&w->cond, NULL);
        w->fn      = NULL;
        w->ctx     = NULL;
        w->tid     = (int64_t)i;
        w->nth     = INTENT_POOL_THREADS;
        w->pending = 0;
        w->stop    = 0;
        pthread_create(&w->thread, NULL, worker_main, w);
    }
}

void intent_pool_run(intent_par_fn fn, void *ctx, int64_t nth) {
    if (nth <= 0) return;
    pthread_once(&pool_once, pool_init);

    int64_t n = nth < INTENT_POOL_THREADS ? nth : INTENT_POOL_THREADS;

    for (int64_t i = 0; i < n; i++) {
        Worker *w = &pool[i];
        pthread_mutex_lock(&w->mu);
        w->fn      = fn;
        w->ctx     = ctx;
        w->tid     = i;
        w->nth     = n;
        w->pending = 1;
        pthread_cond_signal(&w->cond);
        pthread_mutex_unlock(&w->mu);
    }

    for (int64_t i = 0; i < n; i++) {
        Worker *w = &pool[i];
        pthread_mutex_lock(&w->mu);
        while (w->pending)
            pthread_cond_wait(&w->cond, &w->mu);
        pthread_mutex_unlock(&w->mu);
    }
}
