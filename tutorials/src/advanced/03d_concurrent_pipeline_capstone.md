# Advanced 3d -- Capstone: a concurrent sensor-dashboard pipeline

> **Learning goal**: see every advanced-features primitive from this
> section of the book working together in one realistic program --
> `Pool<T>`/`Handle<T>`, `Region`/`ArenaRef<T>`, `Mutex<T>`,
> `Barrier`, `task`+`join`, and `task`+`detach` -- rather than each in
> isolation. Reading order: [Advanced 3 --
> concurrency](03_concurrency.md), [Advanced 2b -- Barrier
> primer](02b_barrier_primer.md), [Advanced 4 --
> embedded](04_embedded.md), and [Intermediate 3c -- shared
> ownership](../intermediate/03c_shared_ownership_primer.md) all
> introduce these primitives one at a time; this chapter is a walking
> tour of what happens when a real program needs several of them at
> once.

This is a walking tour of
[`examples/language/english/concurrent_pipeline_dashboard.vani`](https://github.com/enthusiasticgeek/vani-compiler/blob/main/examples/language/english/concurrent_pipeline_dashboard.vani).

## The scenario

A small telemetry dashboard collects readings from 4 sensors,
processes each one on its own worker thread, and reports a combined
total once every worker has reported in -- while a background
heartbeat logs "still processing" on its own schedule, unrelated to
the actual work. This is a genuinely common shape (an aggregator
polling N data sources in parallel, with a liveness log running
alongside), not an artificial excuse to chain features together.

It needs, in order of appearance:

- **`Pool<i64>` / `Handle<i64>`** -- a registry of raw sensor
  readings. Each worker gets its own `Handle<i64>` instead of the
  raw value directly, mirroring how a real registry would hand out
  opaque IDs rather than pointers.
- **`Region` / `ArenaRef<i64>`** -- each worker arena-allocates a
  couple of derived values from its raw reading and frees them
  together, O(1), when its local processing scope ends.
- **`Mutex<i64>`** -- a shared running total every worker
  contributes to.
- **`Barrier`** -- a checkpoint: nobody should read the combined
  total until every worker has published its own contribution.
- **`task` + `join`** -- 3 background workers, with `main` itself
  acting as the 4th participant.
- **`task` + `detach`** -- a heartbeat logger that runs
  independently and is never waited on.

## Build & run

```bash
vanic run examples/language/english/concurrent_pipeline_dashboard.vani                          # LLVM backend
vanic run examples/language/english/concurrent_pipeline_dashboard.vani --backend=c              # C backend
vanic build examples/language/english/concurrent_pipeline_dashboard.vani -o /tmp/dash && /tmp/dash
```

---

## Step 1: register sensors as a `Vec<Handle<i64>>`

```vani
let pool: Pool<i64> = pool_new();
let h0: Handle<i64> = pool_alloc(mut ref pool, 5);
let h1: Handle<i64> = pool_alloc(mut ref pool, 10);
let h2: Handle<i64> = pool_alloc(mut ref pool, 15);
let h3: Handle<i64> = pool_alloc(mut ref pool, 20);
let handles: Vec<Handle<i64>> = vec(h0, h1, h2, h3);
```

Collecting `Handle<i64>` values into a `Vec` is the natural way to
carry a set of registry entries around -- and until this session it
hit a real C-backend gap (**BUG-189**): the generated C referenced
`intent_handle_i64` before that type was ever `typedef`'d, because
the Pool/Handle helper bundle was emitted *after* the Vec-bundle
pass that needed it. [`handle_job_queue.vani`](../intermediate/03c_shared_ownership_primer.md#pattern-2-handles-indices-not-pointers),
written earlier the same session, worked around the bug entirely by
keeping each handle as an individually-named local. This example is
the fixed, natural version -- no workaround needed.

## Step 2: each worker looks up its own reading and arena-processes it

```vani
fn worker(
  id: i64,
  h: Handle<i64>,
  pool: ref Pool<i64>,
  total: ref Mutex<i64>,
  b: mut ref Barrier
) -> i64 {
  let raw: i64 = unwrap_or(pool_get(pool, h), 0);
  let processed: i64 = 0;

  region scratch {
    let doubled: ArenaRef<i64> = region_borrow_i64(mut ref scratch, raw * 2);
    let doubled_plus_one: ArenaRef<i64> = region_borrow_i64(mut ref scratch, raw * 2 + 1);
    let derived: Vec<ArenaRef<i64>> = vec(doubled, doubled_plus_one);
    print "worker", id, "derived", len(ref derived), "arena values from raw reading", raw;
    processed = aref_load(doubled) + aref_load(doubled_plus_one);
  }
  // `scratch` drops here -- both ArenaRefs freed together, O(1).
  ...
```

`pool: ref Pool<i64>` is a **shared, read-only** reference, safe to
hand to every worker at once -- `pool_get` only reads. Collecting
`doubled`/`doubled_plus_one` into `Vec<ArenaRef<i64>>` hit two more
real bugs, also fixed this session:

- **BUG-188**: the C backend spelled `Vec<ArenaRef<i64>>`'s
  per-shape identifier as the raw C pointer type `int64_t*`,
  producing the invalid identifier `intent_vec_int64_t*` (the `*`
  was never escaped).
- **BUG-190**: on the LLVM backend, forming that same Vec *inside* a
  `region { ... }` block (which desugars to `if true { ... }`)
  could emit a malformed PHI node -- `TypedStmt::If`'s codegen
  tracked which LLVM block was "current" only *after* the whole
  `if` finished, not *while* emitting the `then`-branch's own
  statements, so `vec_fill`-style PHI-based codegen inside the
  branch wired its merge to the wrong predecessor block.

Every raw reading is odd/even-derived deterministically (`raw*2` and
`raw*2+1`), so the combined total is reproducible: readings `5, 10,
15, 20` -> processed values `21, 41, 61, 81` -> grand total `204`.

## Step 3: publish under the mutex, then rendezvous at the barrier

```vani
  {
    let g = mutex_lock(total);
    let cur: i64 = guard_get(ref g);
    guard_set(ref g, cur + processed);
  }
  // guard `g` -- and the lock on `total` -- drops HERE, at the end
  // of this block, not at the end of the function.

  let is_last: bool = barrier_wait(b);
  if is_last {
    let g2 = mutex_lock(total);
    print "checkpoint reached, running total:", guard_get(ref g2);
  }
```

The inner `{ }` block is load-bearing, not decorative. `Guard<T>`
unlocks on scope-exit, not last-use -- without the block, the guard
would stay locked until `worker` returns, and the last thread to
reach the barrier would then try to `mutex_lock(total)` again in the
`if is_last` branch while **still holding its own earlier lock on
the same mutex**. `mutex_lock` isn't reentrant, so that thread would
deadlock against itself. [`barrier_sensor_rendezvous.vani`](02b_barrier_primer.md#a-real-world-example)
hits the identical hazard with a different mutex layout; the fix is
the same shape both times.

Why a `Barrier` here and not just `join` on the 3 background
workers? Because every worker needs to know the total is *complete*
before it's safe to read -- and in this design, only the last
arriver actually reads it (the other three just publish and return).
`join`ing each task individually can't express "wait until everyone
else is also done, from *inside* the worker" -- `join` only lets the
*caller* wait on a worker, not workers wait on each other.

## Step 4: a detached heartbeat, running the whole time

```vani
fn heartbeat_log(ticks: i64) -> i64 {
  let i: i64 = 0;
  while i < ticks {
    print "[dashboard] still processing...";
    sleep_ms(15);
    i = i + 1;
  }
  return 0;
}

fn main() -> i64 {
  ...
  let hb: Task<i64> = task heartbeat_log(6);
  detach hb;
  ...
```

`heartbeat_log` has nothing to report back and no natural moment for
`main` to wait on it -- exactly the shape `detach` exists for (see
[Advanced 3 -- `detach`](03_concurrency.md#detach----fire-and-forget)).
Spawning it as a named function (not the block form `task { ... }`)
matters here too: block-form task bodies are implicitly `pure` and
can't `print` at all.

## Step 5: join the workers, cross-check the total two ways

```vani
  let t1: Task<i64> = task worker(1, handles[1], ref pool, ref total, mut ref b);
  let t2: Task<i64> = task worker(2, handles[2], ref pool, ref total, mut ref b);
  let t3: Task<i64> = task worker(3, handles[3], ref pool, ref total, mut ref b);
  let p0: i64 = worker(0, handles[0], ref pool, ref total, mut ref b);

  let p1: i64 = join t1;
  let p2: i64 = join t2;
  let p3: i64 = join t3;

  let grand_total: i64 = p0 + p1 + p2 + p3;
  print "grand total:", grand_total;
  assert grand_total == 204;

  let g = mutex_lock(ref total);
  let mutex_total: i64 = guard_get(ref g);
  print "mutex total confirms:", mutex_total;
  assert mutex_total == 204;
```

Two independent paths to the same number: summing each worker's
`join`ed return value, and reading the shared mutex directly. Both
must agree -- a cheap, real correctness check that the `Barrier` +
`Mutex` combination is actually doing its job, not just happening to
produce output that looks right.

## Sample output

```
registered 4 sensors
worker 1 derived 2 arena values from raw reading 10
worker 0 derived 2 arena values from raw reading 5
worker 2 derived 2 arena values from raw reading 15
worker 3 derived 2 arena values from raw reading 20
checkpoint reached, running total: 204
grand total: 204
mutex total confirms: 204
[dashboard] still processing...
[dashboard] still processing...
```

(Verified against the real compiler on both backends, run
repeatedly. The four `worker ... derived ...` lines can appear in
any order -- and the heartbeat's own lines can land anywhere,
including interleaved mid-line with other output, since it's
detached and genuinely unsynchronized -- but `checkpoint reached`,
`grand total`, and `mutex total confirms` always report `204`,
exactly, every run: that's the Barrier + Mutex combination's actual
correctness guarantee, not a coincidence of scheduling.)

## Cross-reference

- [Advanced 3 -- concurrency primitives](03_concurrency.md) -- full
  API reference for `task`/`join`/`detach` and all six concurrency
  primitives
- [Advanced 2b -- Barrier primer](02b_barrier_primer.md) -- why
  `Barrier` and not `join` for phase-boundary rendezvous
- [Advanced 4 -- Embedded, `unsafe`, region typing](04_embedded.md)
  -- `Pool<T>`/`Handle<T>` vs `Region`/`ArenaRef<T>`, side by side
- [Intermediate 3c -- Shared ownership without `Rc`/`Arc`](../intermediate/03c_shared_ownership_primer.md)
  -- why vāṇी reaches for handles and arenas instead of reference
  counting in the first place


---

**Previous**: [Sec.3c -- Capstone: timed tic-tac-toe ->](03c_timed_tic_tac_toe_capstone.md)
**Next**: [Sec.3b -- Condition variables: wait-until-predicate primer ->](03b_condvar_primer.md)
