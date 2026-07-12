//! Adversarial tests for the vāṇī safety standard enforcement passes.
//!
//! Each test writes a minimal .vani program to a unique temp directory,
//! runs `vanic check`, and asserts the compiler either:
//!   - Rejects with a diagnostic containing a specific keyword, OR
//!   - Accepts cleanly (no false positives).
//!
//! Tests are grouped by the safety pass they probe. "Adversarial" means
//! the violation is hidden in non-obvious control flow — the common
//! straight-line cases are covered by the language tutorial examples.

use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn vanic() -> String {
    env!("CARGO_BIN_EXE_intentc").to_string()
}

/// Write `src` to a fresh temp file tagged with `name` and return its path.
fn tmp(name: &str, src: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "safety-adv-{}-{}-{}",
        name,
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir_all(&dir).unwrap();
    let path = dir.join("test.vani");
    fs::write(&path, src).unwrap();
    path
}

/// Run `vanic check` on `path`. Returns `(success, stderr)`.
fn check(path: &PathBuf) -> (bool, String) {
    let out = Command::new(vanic())
        .args(["check", path.to_str().unwrap()])
        .output()
        .expect("vanic check must launch");
    (out.status.success(), String::from_utf8_lossy(&out.stderr).into_owned())
}

/// Assert the program is rejected and stderr contains `keyword`.
fn assert_rejected(path: &PathBuf, keyword: &str) {
    let (ok, stderr) = check(path);
    assert!(
        !ok,
        "expected compiler to reject the program (looking for '{}'), but it accepted it.\
         \nstderr: {}",
        keyword, stderr
    );
    assert!(
        stderr.contains(keyword),
        "expected '{}' in stderr, got:\n{}",
        keyword, stderr
    );
}

/// Assert the program compiles cleanly with no diagnostic.
fn assert_accepted(path: &PathBuf) {
    let (ok, stderr) = check(path);
    assert!(
        ok,
        "expected compiler to accept the program, but it rejected it.\nstderr: {}",
        stderr
    );
}

// ── Category 1: ASIL-D / DO-178C composite tag constraints ──────────────────

#[test]
fn asil_d_missing_bounded_stack_is_parse_error() {
    let path = tmp("asil_d_no_bs", r#"
#[asil_d]
#[wcet(cycles = 1000)]
fn ctrl(x: i64) -> i64 { return x; }
fn main() -> i64 { return 0; }
"#);
    assert_rejected(&path, "bounded_stack");
}

#[test]
fn asil_d_missing_wcet_is_parse_error() {
    let path = tmp("asil_d_no_wcet", r#"
#[asil_d]
#[bounded_stack(bytes = 1024)]
fn ctrl(x: i64) -> i64 { return x; }
fn main() -> i64 { return 0; }
"#);
    assert_rejected(&path, "wcet");
}

#[test]
fn asil_d_direct_float_use_rejected() {
    // ASIL-D implies #[no_float]. A float literal in the body must be caught.
    let path = tmp("asil_d_float_direct", r#"
#[asil_d]
#[bounded_stack(bytes = 1024)]
#[wcet(cycles = 5000)]
fn brake(x: i64) -> i64 {
  let scale: f64 = 0.01;
  return (x as f64 * scale) as i64;
}
fn main() -> i64 { return 0; }
"#);
    assert_rejected(&path, "no_float");
}

#[test]
fn asil_d_float_transitive_through_helper_rejected() {
    // The helper has no safety tag but uses float internally.
    // enforce_no_float propagates transitively: entry calls helper,
    // helper touches float → entry gets the "via call to 'helper'" diagnostic.
    let path = tmp("asil_d_float_trans", r#"
fn helper(x: i64) -> i64 {
  let f: f64 = x as f64;
  return f as i64;
}
#[asil_d]
#[bounded_stack(bytes = 1024)]
#[wcet(cycles = 5000)]
fn entry(x: i64) -> i64 {
  return helper(x);
}
fn main() -> i64 { return 0; }
"#);
    assert_rejected(&path, "no_float");
}

#[test]
fn asil_d_heap_allocation_rejected() {
    // ASIL-D implies #[no_heap]. Using vec() allocates.
    let path = tmp("asil_d_heap", r#"
#[asil_d]
#[bounded_stack(bytes = 1024)]
#[wcet(cycles = 5000)]
fn make_buf() -> i64 {
  let xs: Vec<i64> = vec(1, 2, 3);
  return len(xs) as i64;
}
fn main() -> i64 { return 0; }
"#);
    assert_rejected(&path, "heap");
}

#[test]
fn asil_d_direct_self_recursion_rejected() {
    // ASIL-D implies #[no_recursion]. A function calling itself is caught.
    let path = tmp("asil_d_recurse", r#"
#[asil_d]
#[bounded_stack(bytes = 4096)]
#[wcet(cycles = 10000)]
fn countdown(n: i64) -> i64 {
  if n <= 0 { return 0; }
  return countdown(n - 1);
}
fn main() -> i64 { return 0; }
"#);
    assert_rejected(&path, "no_recursion");
}

#[test]
fn asil_d_indirect_recursion_a_calls_b_calls_a_rejected() {
    // Indirect recursion: the BFS in enforce_no_recursion must follow
    // a two-hop call chain (entry → helper → entry) to find the cycle.
    let path = tmp("asil_d_indirect_recurse", r#"
fn helper(x: i64) -> i64 {
  if x > 0 { return entry(x - 1); }
  return 0;
}
#[asil_d]
#[bounded_stack(bytes = 4096)]
#[wcet(cycles = 10000)]
fn entry(x: i64) -> i64 {
  return helper(x);
}
fn main() -> i64 { return 0; }
"#);
    assert_rejected(&path, "recurses");
}

// ── Category 2: MISRA C 2012 rule enforcement ────────────────────────────────

#[test]
fn misra_two_returns_in_flat_body_rejected() {
    let path = tmp("misra_2ret", r#"
#[misra_c_2012]
fn clamp(x: i64) -> i64 {
  if x < 0 { return 0; }
  return x;
}
fn main() -> i64 { return 0; }
"#);
    assert_rejected(&path, "MISRA 15.5");
}

#[test]
fn misra_returns_buried_in_nested_if_else_rejected() {
    // Two returns hidden in a deep if/else chain — not visible at the
    // top-level statement list. collect_returns must recurse into branches.
    let path = tmp("misra_nested_ret", r#"
#[misra_c_2012]
fn classify(x: i64, y: i64) -> i64 {
  if x > 0 {
    if y > 0 {
      return 1;
    } else {
      return 2;
    }
  }
  return 0;
}
fn main() -> i64 { return 0; }
"#);
    assert_rejected(&path, "MISRA 15.5");
}

#[test]
fn misra_return_inside_for_loop_body_rejected() {
    // One top-level return PLUS one inside a for-loop body.
    // collect_returns must descend into loop bodies.
    let path = tmp("misra_loop_ret", r#"
#[misra_c_2012]
fn find_positive(n: i64) -> i64 {
  for i from 0 to n {
    if i > 0 { return i; }
  }
  return -1;
}
fn main() -> i64 { return 0; }
"#);
    assert_rejected(&path, "MISRA 15.5");
}

#[test]
fn misra_dead_branch_always_true_condition_rejected() {
    let path = tmp("misra_dead_true", r#"
#[misra_c_2012]
fn demo(x: i64) -> i64 {
  if true { return x + 1; }
  return x;
}
fn main() -> i64 { return 0; }
"#);
    assert_rejected(&path, "MISRA 14.1");
}

#[test]
fn misra_dead_while_false_loop_rejected() {
    let path = tmp("misra_dead_while", r#"
#[misra_c_2012]
fn demo(x: i64) -> i64 {
  while false {
    return x;
  }
  return x + 1;
}
fn main() -> i64 { return 0; }
"#);
    assert_rejected(&path, "MISRA 14.1");
}

#[test]
fn misra_eval_order_same_var_adjacent_args_rejected() {
    // MISRA 13.2: variable `v` appears in arg positions 0 and 1 of
    // the same call. The checker flags adjacent (consecutive) duplicates.
    let path = tmp("misra_eval_order", r#"
fn add(a: i64, b: i64) -> i64 { return a + b; }
#[misra_c_2012]
fn demo(v: i64) -> i64 {
  return add(v, v);
}
fn main() -> i64 { return 0; }
"#);
    assert_rejected(&path, "MISRA 13.2");
}

// ── Category 3: WCET budget enforcement ────────────────────────────────────

#[test]
fn wcet_budget_exceeded_by_arithmetic_rejected() {
    // budget=5 but a*b+c*d requires ~10 cycles on the conservative model.
    let path = tmp("wcet_exceed", r#"
#[wcet(cycles = 5)]
fn four_muls(a: i64, b: i64, c: i64, d: i64) -> i64 {
  return a * b + c * d;
}
fn main() -> i64 { return 0; }
"#);
    assert_rejected(&path, "wcet");
}

#[test]
fn wcet_unbounded_while_loop_rejected() {
    // A while loop with a variable-bound condition is UNBOUNDED in the
    // model, so any wcet annotation on this function must be rejected.
    let path = tmp("wcet_while", r#"
#[wcet(cycles = 1000)]
fn sum_to(n: i64) -> i64 {
  let acc: i64 = 0;
  let i: i64 = 0;
  while i < n {
    let acc = acc + i;
    let i = i + 1;
  }
  return acc;
}
fn main() -> i64 { return 0; }
"#);
    assert_rejected(&path, "UNBOUNDED");
}

#[test]
fn wcet_unbounded_loop_in_one_branch_rejected() {
    // UNBOUNDED propagates even when only one branch of an if has a while.
    // The function-level estimate becomes UNBOUNDED, violating the budget.
    let path = tmp("wcet_branch_while", r#"
#[wcet(cycles = 500)]
fn maybe_loop(flag: i64, n: i64) -> i64 {
  if flag > 0 {
    let i: i64 = 0;
    while i < n {
      let i = i + 1;
    }
    return n;
  }
  return 0;
}
fn main() -> i64 { return 0; }
"#);
    assert_rejected(&path, "UNBOUNDED");
}

// ── Category 4: Bounded stack enforcement ───────────────────────────────────

#[test]
fn bounded_stack_exceeded_by_local_bindings_rejected() {
    // 1 i64 param (8 bytes) + 3 i64 locals (24 bytes) + 32-byte
    // frame overhead = 64 bytes total. Budget is 48 → must be rejected.
    let path = tmp("bounded_stack_exceed", r#"
#[bounded_stack(bytes = 48)]
fn big_locals(x: i64) -> i64 {
  let a: i64 = x + 1;
  let b: i64 = a + 1;
  let c: i64 = b + 1;
  return c;
}
fn main() -> i64 { return 0; }
"#);
    assert_rejected(&path, "bounded_stack");
}

#[test]
fn bounded_stack_exceeded_via_call_chain_rejected() {
    // The budget is set on the entry function. Its call chain includes
    // a callee with large locals, pushing the total over budget.
    // entry: 0+32=32 bytes, leaf: 32+32=64 bytes. Total: 96 > 80.
    let path = tmp("bounded_stack_chain", r#"
fn leaf(x: i64) -> i64 {
  let a: i64 = x + 1;
  let b: i64 = a + 1;
  let c: i64 = b + 1;
  let d: i64 = c + 1;
  return d;
}
#[bounded_stack(bytes = 80)]
fn entry(x: i64) -> i64 {
  return leaf(x);
}
fn main() -> i64 { return 0; }
"#);
    assert_rejected(&path, "bounded_stack");
}

// ── Category 5: Lock-order deadlock detection (S-19) ─────────────────────────

#[test]
fn lock_order_two_functions_opposite_order_rejected() {
    // fn acquire_xy locks m_x then m_y (edge m_x → m_y).
    // fn acquire_yx locks m_y then m_x (edge m_y → m_x).
    // DFS cycle detection must find the cycle m_x → m_y → m_x.
    let path = tmp("lock_order", r#"
fn acquire_xy(m_x: ref Mutex<i64>, m_y: ref Mutex<i64>) -> i64 {
  let gx: Guard<i64> = mutex_lock(m_x);
  let gy: Guard<i64> = mutex_lock(m_y);
  return 0;
}
fn acquire_yx(m_y: ref Mutex<i64>, m_x: ref Mutex<i64>) -> i64 {
  let gy: Guard<i64> = mutex_lock(m_y);
  let gx: Guard<i64> = mutex_lock(m_x);
  return 0;
}
fn main() -> i64 {
  let m_x: Mutex<i64> = mutex_new(0);
  let m_y: Mutex<i64> = mutex_new(0);
  let _ = acquire_xy(ref m_x, ref m_y);
  let _ = acquire_yx(ref m_y, ref m_x);
  return 0;
}
"#);
    assert_rejected(&path, "S-19");
}

#[test]
fn lock_order_both_orderings_in_single_function_branches_rejected() {
    // Adversarial: both lock orderings appear in DIFFERENT BRANCHES of a
    // single function. The linear walk of collect_lock_sequence appends
    // both branch sequences to the same seq, producing edges m_x→m_y AND
    // m_y→m_x from one function — enough for a cycle.
    let path = tmp("lock_intra_branch", r#"
fn both_orders(flag: i64, m_x: ref Mutex<i64>, m_y: ref Mutex<i64>) -> i64 {
  if flag > 0 {
    let gx: Guard<i64> = mutex_lock(m_x);
    let gy: Guard<i64> = mutex_lock(m_y);
    return 1;
  } else {
    let gy: Guard<i64> = mutex_lock(m_y);
    let gx: Guard<i64> = mutex_lock(m_x);
    return 2;
  }
}
fn main() -> i64 {
  let m_x: Mutex<i64> = mutex_new(0);
  let m_y: Mutex<i64> = mutex_new(0);
  return both_orders(1, ref m_x, ref m_y);
}
"#);
    assert_rejected(&path, "S-19");
}

// ── Category 6: ISR priority inversion (S-20) ───────────────────────────────

#[test]
fn isr_two_priorities_sharing_mutex_name_rejected() {
    // Two ISRs with different priority= values both call mutex_lock on a
    // parameter named 'shared'. The name-based matching in S-20 flags this.
    let path = tmp("isr_priority", r#"
#[interrupt(priority = 1)]
fn high_isr(shared: ref Mutex<i64>) -> i64 {
  let g: Guard<i64> = mutex_lock(shared);
  return 0;
}
#[interrupt(priority = 5)]
fn low_isr(shared: ref Mutex<i64>) -> i64 {
  let g: Guard<i64> = mutex_lock(shared);
  return 0;
}
fn main() -> i64 { return 0; }
"#);
    assert_rejected(&path, "S-20");
}

#[test]
fn isr_same_priority_shared_mutex_not_flagged() {
    // Two ISRs at THE SAME priority sharing a mutex are NOT flagged by S-20
    // (same priority → cannot preempt each other). Only strict inequality
    // triggers the check.
    let path = tmp("isr_same_prio", r#"
#[interrupt(priority = 3)]
fn isr_a(shared: ref Mutex<i64>) -> i64 {
  let g: Guard<i64> = mutex_lock(shared);
  return 0;
}
#[interrupt(priority = 3)]
fn isr_b(shared: ref Mutex<i64>) -> i64 {
  let g: Guard<i64> = mutex_lock(shared);
  return 0;
}
fn main() -> i64 { return 0; }
"#);
    // This should be ACCEPTED — same priority is not a priority inversion.
    // We only verify no panic; the program might still fail for other reasons
    // (e.g. heap use in ISR), so just check stderr lacks S-20.
    let (_ok, stderr) = check(&path);
    assert!(
        !stderr.contains("S-20"),
        "S-20 must not fire for ISRs at the same priority, got:\n{}",
        stderr
    );
}

#[test]
fn isr_no_priority_attr_no_s20_check() {
    // ISRs declared without priority= do NOT participate in the S-20
    // priority-inversion check at all. This verifies the guard
    // `interrupt_priority.is_some()` in enforce_isr_preemption.
    let path = tmp("isr_no_prio", r#"
#[interrupt]
fn isr_a(shared: ref Mutex<i64>) -> i64 {
  let g: Guard<i64> = mutex_lock(shared);
  return 0;
}
#[interrupt]
fn isr_b(shared: ref Mutex<i64>) -> i64 {
  let g: Guard<i64> = mutex_lock(shared);
  return 0;
}
fn main() -> i64 { return 0; }
"#);
    let (_ok, stderr) = check(&path);
    assert!(
        !stderr.contains("S-20"),
        "S-20 must not fire when ISRs have no priority= attribute, got:\n{}",
        stderr
    );
}

// ── Category 7: Inline-aware stack depth (S-14) ──────────────────────────────

#[test]
fn inline_callee_locals_folded_into_caller_frame() {
    // Without inlining: caller frame (32+32=64) + adder frame (8+8+32=48) = 112 bytes.
    // With #[inline] on adder: caller frame = caller_locals(0) + adder_locals(16) + 32 = 48 bytes.
    // Budget is 96 bytes. With inlining the depth is 48 ≤ 96 (passes).
    // Without inlining the depth would be 64+48=112 > 96 (would fail).
    // This verifies that #[inline] actually reduces the measured depth.
    let path = tmp("inline_stack", r#"
#[inline]
fn adder(a: i64, b: i64) -> i64 { return a + b; }
#[bounded_stack(bytes = 96)]
fn caller(x: i64) -> i64 { return adder(x, x + 1); }
fn main() -> i64 { return caller(5); }
"#);
    assert_accepted(&path);
}

#[test]
fn inline_callee_keeps_subcallee_frames_separate() {
    // adder is #[inline] so its 16 bytes are folded into caller's frame.
    // But adder calls leaf (not inline), so leaf gets its own frame push.
    // caller_frame = caller_locals(0) + adder_locals(16) + overhead(32) = 48
    // leaf_frame = 16 + 32 = 48
    // total = 48 + 48 = 96
    // budget = 95 → should be rejected.
    let path = tmp("inline_subcallee", r#"
fn leaf(x: i64) -> i64 {
  let a: i64 = x;
  let b: i64 = a + 1;
  return b;
}
#[inline]
fn adder(a: i64, b: i64) -> i64 { return leaf(a) + b; }
#[bounded_stack(bytes = 95)]
fn caller(x: i64) -> i64 { return adder(x, x + 1); }
fn main() -> i64 { return caller(5); }
"#);
    assert_rejected(&path, "bounded_stack");
}

// ── Category 8: Valid programs — no false positives ─────────────────────────

#[test]
fn asil_d_complete_valid_program_accepted() {
    // A fully annotated ASIL-D function with no violations must compile clean.
    let path = tmp("asil_d_valid", r#"
#[asil_d]
#[bounded_stack(bytes = 2048)]
#[wcet(cycles = 5000)]
fn brake_torque(speed: i64, pedal: i64) -> i64 {
  return speed * pedal / 100;
}
fn main() -> i64 {
  let t: i64 = brake_torque(80, 50);
  assert t >= 0;
  return 0;
}
"#);
    assert_accepted(&path);
}

#[test]
fn misra_single_exit_function_accepted() {
    // A MISRA-tagged function with exactly one return statement at the end
    // must be accepted cleanly (single-exit rule satisfied).
    let path = tmp("misra_single_exit", r#"
#[misra_c_2012]
fn clamp_nonneg(x: i64) -> i64 {
  let result: i64 = if x < 0 { 0 } else { x };
  return result;
}
fn main() -> i64 { return 0; }
"#);
    assert_accepted(&path);
}

#[test]
fn lock_order_consistent_ordering_accepted() {
    // Both functions acquire locks in the SAME order (m_x → m_y).
    // No cycle in the acquisition-order graph → no S-19 diagnostic.
    let path = tmp("lock_consistent", r#"
fn task_a(m_x: ref Mutex<i64>, m_y: ref Mutex<i64>) -> i64 {
  let gx: Guard<i64> = mutex_lock(m_x);
  let gy: Guard<i64> = mutex_lock(m_y);
  return 1;
}
fn task_b(m_x: ref Mutex<i64>, m_y: ref Mutex<i64>) -> i64 {
  let gx: Guard<i64> = mutex_lock(m_x);
  let gy: Guard<i64> = mutex_lock(m_y);
  return 2;
}
fn main() -> i64 {
  let m_x: Mutex<i64> = mutex_new(0);
  let m_y: Mutex<i64> = mutex_new(0);
  let _ = task_a(ref m_x, ref m_y);
  let _ = task_b(ref m_x, ref m_y);
  return 0;
}
"#);
    let (_ok, stderr) = check(&path);
    assert!(
        !stderr.contains("S-19"),
        "S-19 must not fire when all functions acquire locks in the same order, got:\n{}",
        stderr
    );
}

// ── Category 9: Known-gap documentation tests ───────────────────────────────
//
// These tests were previously KNOWN GAPS and have been FIXED (L20, L21, L22).
// They now assert the compiler correctly REJECTS these programs.
// Fixed in: collect_lock_sequence (L20), collect_locked_mutexes (L21),
// check_eval_order_expr (L22).

#[test]
fn gap_s19_lock_order_via_transitive_call_not_detected() {
    // FIXED (L20): collect_lock_sequence now follows calls into user-defined
    // helpers when building the lock-acquisition sequence. fn_a locks m_x
    // then (via helper) m_y; fn_b locks m_y then (via helper) m_x —
    // effective sequences [m_x, m_y] and [m_y, m_x] form a cycle.
    let path = tmp("gap_lock_trans", r#"
fn acquire_second_y(m_y: ref Mutex<i64>) -> i64 {
  let gy: Guard<i64> = mutex_lock(m_y);
  return 0;
}
fn acquire_second_x(m_x: ref Mutex<i64>) -> i64 {
  let gx: Guard<i64> = mutex_lock(m_x);
  return 0;
}
fn fn_a(m_x: ref Mutex<i64>, m_y: ref Mutex<i64>) -> i64 {
  let gx: Guard<i64> = mutex_lock(m_x);
  return acquire_second_y(m_y);
}
fn fn_b(m_x: ref Mutex<i64>, m_y: ref Mutex<i64>) -> i64 {
  let gy: Guard<i64> = mutex_lock(m_y);
  return acquire_second_x(m_x);
}
fn main() -> i64 {
  let m_x: Mutex<i64> = mutex_new(0);
  let m_y: Mutex<i64> = mutex_new(0);
  let _ = fn_a(ref m_x, ref m_y);
  let _ = fn_b(ref m_x, ref m_y);
  return 0;
}
"#);
    assert_rejected(&path, "S-19");
}

#[test]
fn gap_s20_isr_mutex_through_helper_not_detected() {
    // FIXED (L21): collect_locked_mutexes now follows calls into user-defined
    // helpers. Both ISRs call do_lock() which acquires the mutex — the helper's
    // mutex is now attributed to both ISRs and the priority-inversion is detected.
    let path = tmp("gap_isr_helper", r#"
fn do_lock(shared: ref Mutex<i64>) -> i64 {
  let g: Guard<i64> = mutex_lock(shared);
  return 0;
}
#[interrupt(priority = 1)]
fn high_isr(shared: ref Mutex<i64>) -> i64 {
  return do_lock(shared);
}
#[interrupt(priority = 5)]
fn low_isr(shared: ref Mutex<i64>) -> i64 {
  return do_lock(shared);
}
fn main() -> i64 { return 0; }
"#);
    assert_rejected(&path, "S-20");
}

#[test]
fn gap_misra_13_2_non_adjacent_duplicate_not_detected() {
    // FIXED (L22): check_eval_order_expr now uses seen.remove() instead of
    // seen.get(), so any second occurrence of a variable at any distance
    // (not just adjacent) is detected. foo(x, y, x) is now caught.
    let path = tmp("gap_eval_order_nonadj", r#"
fn three_args(a: i64, b: i64, c: i64) -> i64 { return a + b + c; }
#[misra_c_2012]
fn demo(v: i64, w: i64) -> i64 {
  return three_args(v, w, v);
}
fn main() -> i64 { return 0; }
"#);
    assert_rejected(&path, "MISRA 13.2");
}
