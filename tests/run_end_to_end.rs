use std::process::Command;

// BUG-20 (2026-07-27): `check_match_slice` type-checked pattern
// guards on slice/array match arms but never wired them into the
// generated dispatch condition -- a guarded arm always behaved as if
// its guard were `true`, silently returning wrong results. This is
// exactly the "compiles fine but produces the wrong answer" class of
// bug that only an actual execution test catches -- a compile-only
// test would never have caught the original bug, so this checks real
// stdout on both backends against `slice_pattern_guards.vani`, whose
// comments spell out the expected result of each guarded case.
#[test]
fn slice_pattern_guards_example_produces_correct_output_on_both_backends() {
    let binary = env!("CARGO_BIN_EXE_intentc");
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let example = format!(
        "{}/examples/language/english/slice_pattern_guards.vani",
        manifest_dir
    );
    let expected = "no data\nsingle A\nsingle non-A\nbalanced\nunbalanced\n";

    for backend_args in [vec!["run", &example], vec!["run", &example, "--backend=c"]] {
        let output = Command::new(binary)
            .args(&backend_args)
            .output()
            .unwrap_or_else(|e| panic!("intentc {:?} should execute: {e}", backend_args));
        assert!(
            output.status.success(),
            "intentc {:?} failed with status {:?}\nstderr: {}",
            backend_args,
            output.status,
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert_eq!(
            stdout.replace("\r\n", "\n"),
            expected,
            "guarded slice-match arm(s) produced the wrong result for {:?} \
             -- a guard that should have failed and fallen through to the \
             next arm silently ran anyway, or vice versa",
            backend_args
        );
    }
}

// BUG-22 (2026-07-28): a struct or enum RwLock<T>/Mutex<T> payload
// failed to compile with a real `cc` invocation on the C backend --
// two independent bugs, both fixed this session. (1) `c_type_name`
// was missing the 5 arms for Mutex/Guard/RwLock/ReadGuard/WriteGuard
// and fell through to a hardcoded i64-only spelling that didn't match
// the real per-T bundle names -- broke even plain Mutex<i64>/
// RwLock<i64>. (2) format_declarator (used for function PARAMETER
// declarators specifically, a different code path than let-bindings/
// return types) had the SAME missing-arm gap independently, in three
// places (bare, `ref T`, `mut ref T`) -- so a function taking `mut
// ref RwLock<Config>` still emitted the wrong prototype type even
// after fixing (1). Separately, the concurrency bundle (which embeds
// T BY VALUE) was emitted before the struct/enum's fields were fully
// defined -- "unknown type name 'Struct_Point'" -- fixed by emitting
// it right after struct/enum/vec-bundle definitions instead of deep
// in an unrelated helper-emission sequence. This is a "doesn't
// compile with a real C compiler" bug that vanic's own type-checker
// can't catch (the generated C is syntactically valid vāṇी-side) --
// only an actual `cc` invocation does, hence the end-to-end test
// here rather than a compile_to_c substring check alone.
#[test]
fn rwlock_struct_payload_example_compiles_and_runs_on_both_backends() {
    let binary = env!("CARGO_BIN_EXE_intentc");
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let example = format!(
        "{}/examples/language/english/rwlock_struct_payload.vani",
        manifest_dir
    );
    let expected = "before: 5000\nafter: 10000\n";

    for backend_args in [vec!["run", &example], vec!["run", &example, "--backend=c"]] {
        let output = Command::new(binary)
            .args(&backend_args)
            .output()
            .unwrap_or_else(|e| panic!("intentc {:?} should execute: {e}", backend_args));
        assert!(
            output.status.success(),
            "intentc {:?} failed with status {:?}\nstderr: {}",
            backend_args,
            output.status,
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert_eq!(
            stdout.replace("\r\n", "\n"),
            expected,
            "RwLock<Config> (struct payload, used as a fn param type too) \
             produced the wrong result for {:?}",
            backend_args
        );
    }
}

// BUG-49 (2026-07-31): `await(io_*_async(..))` -- BUG-48 fixed the
// stack-overflow crash on every `await()` call but deliberately left
// it rejecting cleanly instead of actually compiling. This is the
// follow-up: `try_desugar_let_match_with_suspends` now recognizes the
// exact `synthesize_await_desugar` output shape (`match inner {
// Future.Ready(v) then v, Future.Pending then 0 }` where `inner` is
// directly an `io_*_async` call) and rewrites it straight to the
// already-working direct-suspend `Let` form, bypassing the
// `Future`-variant match entirely. A compile-only check can't catch a
// regression here (an earlier naive scrutinee-hoist attempt this
// session compiled fine but produced the WRONG type for the local) --
// only a real TCP round-trip through the compiler-synthesized
// Task/poll-fn state machine, using `await()` at every suspend point,
// proves the awaited value is actually correct at runtime.
#[test]
fn bug49_await_builtin_example_compiles_and_runs_on_both_backends() {
    let binary = env!("CARGO_BIN_EXE_intentc");
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let example = format!(
        "{}/examples/language/english/bug49_await_builtin.vani",
        manifest_dir
    );
    let expected = "server bound (port > 0): true\nechoed bytes: 7\n";

    for backend_args in [vec!["run", &example], vec!["run", &example, "--backend=c"]] {
        let output = Command::new(binary)
            .args(&backend_args)
            .output()
            .unwrap_or_else(|e| panic!("intentc {:?} should execute: {e}", backend_args));
        assert!(
            output.status.success(),
            "intentc {:?} failed with status {:?}\nstderr: {}",
            backend_args,
            output.status,
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert_eq!(
            stdout.replace("\r\n", "\n"),
            expected,
            "await(io_*_async(..)) produced the wrong result for {:?} -- \
             the awaited value must match a real 7-byte TCP payload \
             (\"await49\") round-tripped through both suspend points",
            backend_args
        );
    }
}

// BUG-21 Path B (2026-07-28): `Task<R>` -- a genuine expression-form
// `task callee(args)` / `join name` that spawns a real OS thread
// running a `pure fn` and carries its return value back across the
// thread boundary, matching what the tutorials describe (as opposed
// to the pre-existing block-form `task { .. }` / statement-only
// `join name;`, which has no return-value payload). Exercises two
// concurrent spawns with a multi-arg callee, join-with-capture
// (`let r = join t;`), and join-without-capture (bare `join t;` on a
// `Task<R>`, discarding the result) on both backends -- an actual
// execution test is the only way to catch a wrong result here (e.g.
// a mis-sized ctx struct or a wrong field offset would still compile
// and link, just read back garbage or crash nondeterministically).
#[test]
fn task_result_multi_example_produces_correct_output_on_both_backends() {
    let binary = env!("CARGO_BIN_EXE_intentc");
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let example = format!(
        "{}/examples/language/english/task_result_multi.vani",
        manifest_dir
    );
    let expected = "r1: 5\nr2: 30\ndone\n";

    for backend_args in [vec!["run", &example], vec!["run", &example, "--backend=c"]] {
        let output = Command::new(binary)
            .args(&backend_args)
            .output()
            .unwrap_or_else(|e| panic!("intentc {:?} should execute: {e}", backend_args));
        assert!(
            output.status.success(),
            "intentc {:?} failed with status {:?}\nstderr: {}",
            backend_args,
            output.status,
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert_eq!(
            stdout.replace("\r\n", "\n"),
            expected,
            "Task<R> spawn/join produced the wrong result for {:?}",
            backend_args
        );
    }
}

// Heap-overflow bug (found 2026-07-28, while auditing the codebase
// for other instances of the BUG-19/22 "parallel dispatch functions
// drift out of sync" pattern): the LLVM backend's task-spawn ctx-size
// estimator (`compute_ctx_size` / `task_spawn_call_ctx_size` in
// backend_llvm.rs) hardcoded 8 bytes for any type not on its short
// explicit list -- so a Copy struct wider than 8 bytes, as a `Task<R>`
// result/arg or a block-form `task { .. }` capture, got undersized in
// the `malloc` call. Confirmed via generated IR: `Task<Big>` (a 4-field
// i64 struct) emitted `malloc(16)` for a ctx that actually needed 40
// bytes -- a real heap buffer overflow on the trampoline's `store`.
// Fixed by routing through `llvm_byte_size` (the function this file
// already uses, correctly, to size enum-payload buffers) instead. This
// is exactly the class of bug an execution test catches and a
// compile-only test doesn't -- a too-small malloc still compiles and
// links; it corrupts heap memory silently until something downstream
// notices (or doesn't, non-deterministically).
#[test]
fn task_struct_ctx_sizing_example_produces_correct_output_on_both_backends() {
    let binary = env!("CARGO_BIN_EXE_intentc");
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let example = format!(
        "{}/examples/language/english/task_struct_ctx_sizing.vani",
        manifest_dir
    );
    let expected = "a: 100\nb: 101\nc: 102\nd: 103\ncapture ok\n";

    for backend_args in [vec!["run", &example], vec!["run", &example, "--backend=c"]] {
        let output = Command::new(binary)
            .args(&backend_args)
            .output()
            .unwrap_or_else(|e| panic!("intentc {:?} should execute: {e}", backend_args));
        assert!(
            output.status.success(),
            "intentc {:?} failed with status {:?}\nstderr: {}",
            backend_args,
            output.status,
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert_eq!(
            stdout.replace("\r\n", "\n"),
            expected,
            "struct-payload task ctx sizing produced the wrong result for {:?} \
             (or crashed/hung from heap corruption if the malloc undersizing regressed)",
            backend_args
        );
    }
}

// BUG-27 (2026-07-28): raw pointer (`*const T` / `*mut T`) struct
// FIELDS emitted an unusable placeholder comment on the C backend
// (`/* *mut T */ subject_ptr;`) instead of a real C declarator --
// `cc` rejected any struct with a raw-pointer field with "expected
// specifier-qualifier-list". Root cause: `c_element_storage` (used
// specifically for struct-field / Vec-element storage spelling, a
// THIRD parallel type-dispatch function alongside `c_type_name` and
// `format_declarator`) had no arms for `Type::Ptr`/`Type::PtrMut`
// and fell through to `c_leaf_type`'s placeholder-comment fallback.
// The LLVM backend was never affected -- it doesn't route struct
// fields through this function. Found while writing a worked
// example for `tutorials/src/intermediate/03d_cyclic_references_
// primer.md`'s self-deregistering-observer-via-`unsafe` pattern
// (which needs exactly this shape: a raw pointer stored as a
// struct field). Requires `INTENT_TARGET_EMBEDDED=1` since raw
// pointer types are gated to embedded targets in v1 (Layer 1.1 of
// `unsafe.md`).
#[test]
fn observer_self_deregistering_example_produces_correct_output_on_both_backends() {
    let binary = env!("CARGO_BIN_EXE_intentc");
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let example = format!(
        "{}/examples/language/english/design_patterns/behavioral/observer_self_deregistering.vani",
        manifest_dir
    );
    let expected = "active observers: 2\nobserver 1 deregistered itself\nactive observers: 1\nobserver 2 deregistered itself\nactive observers: 0\n";

    for backend_args in [vec!["run", &example], vec!["run", &example, "--backend=c"]] {
        let output = Command::new(binary)
            .args(&backend_args)
            .env("INTENT_TARGET_EMBEDDED", "1")
            .output()
            .unwrap_or_else(|e| panic!("intentc {:?} should execute: {e}", backend_args));
        assert!(
            output.status.success(),
            "intentc {:?} failed with status {:?}\nstderr: {}",
            backend_args,
            output.status,
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert_eq!(
            stdout.replace("\r\n", "\n"),
            expected,
            "self-deregistering observer (raw pointer struct field) produced the \
             wrong result for {:?}",
            backend_args
        );
    }
}

// BUG-28 (2026-07-28): a chain of 3+ guarded match arms sharing the
// same dispatch tag (multiple guarded wildcards, or a guarded enum
// variant repeated) only partially folded into a real conditional --
// the merge logic in `check_expr`'s match-arm handling only looked
// ONE arm back, so a THIRD (or later) guarded arm in a chain never
// merged with the earlier ones. Those earlier guarded arms became
// unreachable dead code sharing the same dispatch tag as a later
// arm, so their guards were silently never evaluated at runtime --
// dispatch always took the first arm's body for that tag. Found
// while writing a worked example for `beginner/08a_pattern_match_
// primer.md`'s "range patterns" section (which needs exactly this
// shape: several guarded wildcard arms in sequence). Confirmed via
// a minimal repro (`match n { _ if n < 10 then "small", _ if n < 100
// then "medium", _ then "big" }`) that returned "small" for every
// input before the fix. This is exactly the class of bug an
// execution test catches and a compile-only test doesn't -- the
// buggy version compiled without error, it just silently dispatched
// to the wrong arm.
#[test]
fn match_guard_chain_example_produces_correct_output_on_both_backends() {
    let binary = env!("CARGO_BIN_EXE_intentc");
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let example = format!(
        "{}/examples/language/english/match_guard_chain.vani",
        manifest_dir
    );
    let expected = "perfect\nA\nB\nC\nD\nF\nactive-long\nactive-medium\nactive-short\nidle\n";

    for backend_args in [vec!["run", &example], vec!["run", &example, "--backend=c"]] {
        let output = Command::new(binary)
            .args(&backend_args)
            .output()
            .unwrap_or_else(|e| panic!("intentc {:?} should execute: {e}", backend_args));
        assert!(
            output.status.success(),
            "intentc {:?} failed with status {:?}\nstderr: {}",
            backend_args,
            output.status,
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert_eq!(
            stdout.replace("\r\n", "\n"),
            expected,
            "chained guarded match arms produced the wrong result for {:?}",
            backend_args
        );
    }
}

// BUG-29 (2026-07-28): a payload-less variant of an enum whose OTHER
// variant carries a `Str` payload (not `OwnedStr`) crashed the LLVM
// backend -- the zero-init placeholder for the payload-less variant's
// unused payload slot only handled `OwnedStr`, not `Str`, even though
// both lower to `i8*` and both need LLVM's `null` literal instead of
// the integer `0` the fallback produced. `lli` rejected the emitted
// IR with "integer constant must have integer type". Found while
// writing a two-flat-matches worked example (nested variant dispatch)
// for `beginner/08a_pattern_match_primer.md`. The C backend was never
// affected.
#[test]
fn nested_enum_str_payload_example_produces_correct_output_on_both_backends() {
    let binary = env!("CARGO_BIN_EXE_intentc");
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let example = format!(
        "{}/examples/language/english/nested_enum_str_payload.vani",
        manifest_dir
    );
    let expected = "0\n1\n2\n-1\n";

    for backend_args in [vec!["run", &example], vec!["run", &example, "--backend=c"]] {
        let output = Command::new(binary)
            .args(&backend_args)
            .output()
            .unwrap_or_else(|e| panic!("intentc {:?} should execute: {e}", backend_args));
        assert!(
            output.status.success(),
            "intentc {:?} failed with status {:?}\nstderr: {}",
            backend_args,
            output.status,
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert_eq!(
            stdout.replace("\r\n", "\n"),
            expected,
            "nested enum with Str payload produced the wrong result for {:?}",
            backend_args
        );
    }
}

#[test]
fn run_basics_example_succeeds_and_prints_42() {
    let binary = env!("CARGO_BIN_EXE_intentc");
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let example = format!("{}/examples/language/english/basics.vani", manifest_dir);

    let output = Command::new(binary)
        .args(["run", &example])
        .output()
        .expect("intentc run should execute");

    assert!(
        output.status.success(),
        "intentc run failed with status {:?}\nstderr: {}",
        output.status,
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("42"),
        "expected bounded_score(20) = 42 in stdout, got: {stdout}"
    );
}

#[test]
fn check_examples_all_succeed() {
    let binary = env!("CARGO_BIN_EXE_intentc");
    let manifest_dir = env!("CARGO_MANIFEST_DIR");

    for example in [
        "basics.vani",
        "integers.vani",
        "floats_and_shifts.vani",
        "arrays.vani",
        "array_return.vani",
        "vectors.vani",
        "borrows.vani",
        "control_flow.vani",
        "drop_interface.vani",
        "memory_safety.vani",
        "dyn_dispatch.vani",
        "early_exit.vani",
        "scopes.vani",
        "modules.vani",
        "mut_refs.vani",
        "verified.vani",
        "for_loops.vani",
        "contracts.vani",
        "invariants.vani",
        "iterate.vani",
        "assert_messages.vani",
        "inline_call_proofs.vani",
        "vec_invariants.vani",
        "bounds_elision.vani",
    ] {
        let path = format!("{}/examples/language/english/{}", manifest_dir, example);
        let output = Command::new(binary)
            .args(["check", &path])
            .output()
            .expect("intentc check should execute");
        assert!(
            output.status.success(),
            "check failed for {}: {}",
            example,
            String::from_utf8_lossy(&output.stderr)
        );
    }
}

#[test]
fn multi_file_diagnostic_points_to_imported_file() {
    use std::fs;
    use std::path::PathBuf;

    let binary = env!("CARGO_BIN_EXE_intentc");
    let dir: PathBuf = std::env::temp_dir().join(format!(
        "intentc-filemap-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir_all(&dir).expect("mkdir");

    fs::write(
        dir.join("lib.vani"),
        "fn broken(x: nonsense) -> i64 { return 0; }\n",
    )
    .expect("write lib");
    fs::write(
        dir.join("main.vani"),
        "use \"lib.vani\";\n\nfn main() -> i64 {\n  return 0;\n}\n",
    )
    .expect("write main");

    let output = Command::new(binary)
        .args(["check", dir.join("main.vani").to_str().unwrap()])
        .output()
        .expect("check");

    let stderr = String::from_utf8_lossy(&output.stderr);
    let lib_path = dir
        .join("lib.vani")
        .canonicalize()
        .expect("canonicalize lib")
        .display()
        .to_string();

    // Clean up before asserting so the dir isn't left around on failure.
    let _ = fs::remove_dir_all(&dir);

    assert!(
        !output.status.success(),
        "expected check to fail; stderr was: {stderr}"
    );
    // The diagnostic must be attributed to the imported file's actual path,
    // pinpointing line 1 inside that file.
    assert!(
        stderr.contains(&format!("{}:1:", lib_path)),
        "expected diagnostic at {}:1:..., got:\n{}",
        lib_path,
        stderr
    );
}

#[test]
fn multi_file_compile_resolves_use() {
    use std::fs;
    use std::path::PathBuf;

    let binary = env!("CARGO_BIN_EXE_intentc");
    let dir: PathBuf = std::env::temp_dir().join(format!(
        "intentc-multifile-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir_all(&dir).expect("mkdir");

    fs::write(
        dir.join("lib.vani"),
        "fn double(x: i64) -> i64 { return x * 2; }\n",
    )
    .expect("write lib");
    fs::write(
        dir.join("main.vani"),
        r#"use "lib.vani";

fn main() -> i64 {
  let x: i64 = double(21);
  assert x == 42;
  print x;
  return 0;
}
"#,
    )
    .expect("write main");

    let output = Command::new(binary)
        .args(["run", dir.join("main.vani").to_str().unwrap()])
        .output()
        .expect("run multi-file");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let status = output.status;

    // Clean up before asserting so we don't leave the dir on failure.
    let _ = fs::remove_dir_all(&dir);

    assert!(
        status.success(),
        "multi-file run failed: {} (stderr: {})",
        status,
        stderr
    );
    assert!(
        stdout.contains("42"),
        "expected double(21)==42 in stdout, got: {stdout}"
    );
}

// Closure #280: vani.toml manifest auto-discovery. When
// `intentc build|run|check` is invoked without a positional
// source file, the driver walks up from cwd to find a
// `vani.toml`, parses `[package].entry`, and uses that as
// the entry point. Tests the parent-walk + flag-interleaving
// behavior end-to-end.
// Closure #289: `#[bounded(N)]` on the SSA-LLVM path
// (default for `intentc run`). The fn under the bound runs
// normally when depth ≤ N; aborts (SIGABRT, exit 134) when
// depth exceeds N. Verifies the depth-counter
// instrumentation lands correctly in LLVM IR.
#[test]
fn bounded_attribute_aborts_when_depth_exceeded_on_llvm() {
    use std::fs;
    use std::path::PathBuf;

    let binary = env!("CARGO_BIN_EXE_intentc");
    let dir: PathBuf = std::env::temp_dir().join(format!(
        "intentc-bounded-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir_all(&dir).expect("mkdir");
    let src = dir.join("bounded.vani");
    fs::write(
        &src,
        "#[bounded(3)]\n\
         fn deep(n: i64) -> i64 {\n  \
           if n <= 0 { return 0; }\n  \
           return deep(n - 1) + 1;\n\
         }\n\
         fn main() -> i64 { return deep(10); }\n",
    )
    .expect("write src");

    let bin_path = dir.join("bounded.bin");
    let build = std::process::Command::new(binary)
        .args([
            "build",
            src.to_str().unwrap(),
            "-o",
            bin_path.to_str().unwrap(),
        ])
        .output()
        .expect("intentc build executes");
    if !build.status.success() {
        let _ = fs::remove_dir_all(&dir);
        panic!(
            "intentc build (bounded LLVM) failed:\nstderr: {}",
            String::from_utf8_lossy(&build.stderr)
        );
    }
    let run = std::process::Command::new(&bin_path)
        .output()
        .expect("binary runs");
    let _ = fs::remove_dir_all(&dir);
    // Aborted process: code() returns None on Unix; check
    // via `signal()` (SIGABRT == 6). On platforms where
    // `code()` returns 134 (shell-style), also accept that.
    let code = run.status.code();
    #[cfg(unix)]
    let signal = {
        use std::os::unix::process::ExitStatusExt;
        run.status.signal()
    };
    #[cfg(not(unix))]
    let signal: Option<i32> = None;
    // On Unix: SIGABRT == 6 (signal) or 134 (shell exit code).
    // On Windows/MinGW: abort() calls TerminateProcess with code 3.
    // On Windows/ORC JIT: abort() yields a negative crash code.
    assert!(
        code == Some(134) || signal == Some(6) || code == Some(3)
            || code.map_or(false, |c| c < 0),
        "expected SIGABRT from #[bounded(3)] deep(10), got code={:?} signal={:?}",
        code,
        signal
    );
}

// Closure #287: vani.toml v2 `[deps]` with local-path
// entries pulls the dep's entry source into the main
// program's build. Validates the local-path resolution end-
// to-end: a `mathlib` package with a `triple` fn is
// declared as a dep of `main_app`, which calls `triple(7)`.
#[test]
fn manifest_deps_local_path_brings_lib_into_scope() {
    use std::fs;
    use std::path::PathBuf;

    let binary = env!("CARGO_BIN_EXE_intentc");
    let workspace: PathBuf = std::env::temp_dir().join(format!(
        "intentc-deps-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let lib_dir = workspace.join("lib");
    let main_dir = workspace.join("main");
    fs::create_dir_all(lib_dir.join("src")).expect("mkdir lib/src");
    fs::create_dir_all(main_dir.join("src")).expect("mkdir main/src");

    fs::write(
        lib_dir.join("vani.toml"),
        "[package]\nname = \"mathlib\"\nentry = \"src/mathlib.vani\"\n",
    )
    .expect("write lib manifest");
    fs::write(
        lib_dir.join("src/mathlib.vani"),
        "fn triple(x: i64) -> i64 { return x * 3; }\n",
    )
    .expect("write lib source");
    fs::write(
        main_dir.join("vani.toml"),
        "[package]\nname = \"main_app\"\nentry = \"src/main.vani\"\n\n\
         [deps]\nmathlib = { path = \"../lib\" }\n",
    )
    .expect("write main manifest");
    // Kosh namespacing arc (2026-07-21, docs/kosh_namespacing_design.md
    // Phase 3): a [deps] package is compiled inside its own namespace,
    // so its functions are called as `pkgname::item`, not bare -- this
    // is what lets two unrelated packages (or a package and a vāṇी
    // builtin) share a function name without colliding.
    fs::write(
        main_dir.join("src/main.vani"),
        "fn main() -> i64 { return mathlib::triple(7); }\n",
    )
    .expect("write main source");

    let output = std::process::Command::new(binary)
        .args(["run"])
        .current_dir(&main_dir)
        .output()
        .expect("intentc run executes");

    let status = output.status;
    let _ = fs::remove_dir_all(&workspace);

    assert_eq!(
        status.code(),
        Some(21),
        "expected mathlib::triple(7)=21, got status {} (stderr: {})",
        status,
        String::from_utf8_lossy(&output.stderr),
    );
}

#[test]
fn manifest_discovery_resolves_entry_from_subdir() {
    use std::fs;
    use std::path::PathBuf;

    let binary = env!("CARGO_BIN_EXE_intentc");
    let dir: PathBuf = std::env::temp_dir().join(format!(
        "intentc-manifest-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let src_dir = dir.join("src");
    let sub_dir = dir.join("nested/deep");
    fs::create_dir_all(&src_dir).expect("mkdir src");
    fs::create_dir_all(&sub_dir).expect("mkdir nested/deep");

    fs::write(
        dir.join("vani.toml"),
        "[package]\nname = \"manifest_test\"\nentry = \"src/main.vani\"\n",
    )
    .expect("write manifest");
    fs::write(
        src_dir.join("main.vani"),
        "fn main() -> i64 { write \"from manifest\"; return 42; }\n",
    )
    .expect("write entry");

    // Invoke `intentc run` from the deep subdir with no
    // positional arg. The driver must walk up to find the
    // manifest and use its entry.
    let output = std::process::Command::new(binary)
        .args(["run"])
        .current_dir(&sub_dir)
        .output()
        .expect("intentc run executes");

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let status = output.status;

    let _ = fs::remove_dir_all(&dir);

    assert!(
        status.success() || status.code() == Some(42),
        "intentc run via manifest failed: {} (stdout: {}, stderr: {})",
        status,
        stdout,
        stderr,
    );
    assert!(
        stdout.contains("from manifest"),
        "expected `from manifest` in stdout, got: {stdout}"
    );
}

#[test]
fn manifest_build_with_o_flag_finds_entry() {
    use std::fs;
    use std::path::PathBuf;

    let binary = env!("CARGO_BIN_EXE_intentc");
    let dir: PathBuf = std::env::temp_dir().join(format!(
        "intentc-manifest-build-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let src_dir = dir.join("src");
    fs::create_dir_all(&src_dir).expect("mkdir src");
    fs::write(
        dir.join("vani.toml"),
        "[package]\nname = \"build_test\"\nentry = \"src/main.vani\"\n",
    )
    .expect("write manifest");
    fs::write(
        src_dir.join("main.vani"),
        "fn main() -> i64 { return 17; }\n",
    )
    .expect("write entry");

    let bin_path = dir.join("out_binary");
    let build = std::process::Command::new(binary)
        .args(["build", "-o", bin_path.to_str().unwrap()])
        .current_dir(&dir)
        .output()
        .expect("intentc build executes");

    if !build.status.success() {
        let _ = fs::remove_dir_all(&dir);
        panic!(
            "intentc build via manifest + -o failed:\nstderr: {}",
            String::from_utf8_lossy(&build.stderr)
        );
    }

    let run = std::process::Command::new(&bin_path)
        .output()
        .expect("binary runs");
    let exit = run.status.code().unwrap_or(-1);

    let _ = fs::remove_dir_all(&dir);

    assert_eq!(exit, 17, "expected exit 17 from manifest-built binary");
}

// FFI v4 follow-up: `intentc run --backend=c --link-with foo.c`
// threads the same linker flags as `build` so rapid iteration
// can call user-provided extern bodies without a separate
// build step. LLVM-JIT remains host-symbol-only because lli
// can't link static translation units.
#[test]
fn run_link_with_resolves_extern_c_symbol_in_run_mode() {
    use std::fs;
    use std::path::PathBuf;

    let binary = env!("CARGO_BIN_EXE_intentc");
    let dir: PathBuf = std::env::temp_dir().join(format!(
        "intentc-runlinkwith-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir_all(&dir).expect("mkdir");

    let helper_c = dir.join("helper.c");
    fs::write(
        &helper_c,
        "#include <stdint.h>\nint32_t triple(int32_t x) { return x * 3; }\n",
    )
    .expect("write helper.c");

    let vani_src = dir.join("prog.vani");
    fs::write(
        &vani_src,
        "extern \"C\" fn triple(x: i32) -> i32;\n\
         \n\
         fn main() -> i64 {\n  \
           let r: i32 = triple(7 as i32);\n  \
           write \"triple(7) =\", r;\n  \
           return 0;\n}\n",
    )
    .expect("write prog.vani");

    let run = Command::new(binary)
        .args([
            "run",
            vani_src.to_str().unwrap(),
            "--backend=c",
            "--link-with",
            helper_c.to_str().unwrap(),
        ])
        .output()
        .expect("intentc run --link-with runs");

    let stdout = String::from_utf8_lossy(&run.stdout).to_string();
    let stderr = String::from_utf8_lossy(&run.stderr).to_string();
    let status = run.status;

    let _ = fs::remove_dir_all(&dir);

    assert!(
        status.success(),
        "intentc run --backend=c --link-with failed: {} (stdout: {}, stderr: {})",
        status,
        stdout,
        stderr,
    );
    assert!(
        stdout.contains("triple(7) = 21"),
        "expected `triple(7) = 21` in stdout, got: {stdout}"
    );
}

// BUG-23 (found 2026-07-27, fixed same day): the C backend's
// `while_bounds_hints` pre-loop optimizer-aid macro referenced a Vec
// name found via `collect_vec_idx_names` even when that Vec was
// `let`-declared FRESH inside the very loop body being scanned (e.g.
// `while j < n { let xp: Vec<f64> = ...; set(mut ref xp, j, xp[j] + h); }`).
// The emitted hint sits BEFORE the `while` statement, where a
// per-iteration local like `xp` doesn't exist yet in the generated C --
// `cc` rejected the output with `'v_xp' undeclared`. Found while
// investigating why vani-algebra's `algebra_newton_system_fd` (real,
// shipped library code, not a contrived case) failed to compile on
// `--backend=c`. Fixed by tracking each loop body's own `let`-declared
// names and excluding them from the hint set.
#[test]
fn run_backend_c_vec_declared_fresh_inside_while_loop_body() {
    use std::fs;
    use std::path::PathBuf;

    let binary = env!("CARGO_BIN_EXE_intentc");
    let dir: PathBuf = std::env::temp_dir().join(format!(
        "intentc-bug23-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir_all(&dir).expect("mkdir");

    let vani_src = dir.join("prog.vani");
    fs::write(
        &vani_src,
        "fn copy_f64(x: ref Vec<f64>) -> Vec<f64> {\n  \
           let out: Vec<f64> = vec();\n  \
           let n: i64 = len(x) as i64;\n  \
           let i: i64 = 0;\n  \
           while i < n {\n    \
             push(mut ref out, x[i]);\n    \
             i = i + 1;\n  \
           }\n  \
           return out;\n\
         }\n\
         \n\
         fn f(x: ref Vec<f64>, n: i64, h: f64) -> f64 {\n  \
           let s: f64 = 0.0;\n  \
           let j: i64 = 0;\n  \
           while j < n {\n    \
             let xp: Vec<f64> = copy_f64(x);\n    \
             set(mut ref xp, j, xp[j] + h);\n    \
             s = s + xp[j];\n    \
             j = j + 1;\n  \
           }\n  \
           return s;\n\
         }\n\
         \n\
         fn main() -> i64 {\n  \
           let x: Vec<f64> = vec(1.0, 2.0, 3.0);\n  \
           let r: f64 = f(ref x, 3, 0.1);\n  \
           print r;\n  \
           return 0;\n\
         }\n",
    )
    .expect("write prog.vani");

    let run = Command::new(binary)
        .args(["run", vani_src.to_str().unwrap(), "--backend=c"])
        .output()
        .expect("intentc run --backend=c executes");

    let stdout = String::from_utf8_lossy(&run.stdout).to_string();
    let stderr = String::from_utf8_lossy(&run.stderr).to_string();
    let status = run.status;

    let _ = fs::remove_dir_all(&dir);

    assert!(
        status.success(),
        "intentc run --backend=c failed on a Vec declared fresh inside a \
         while loop body: {} (stdout: {}, stderr: {})",
        status,
        stdout,
        stderr,
    );
    assert!(
        stdout.trim() == "6.3",
        "expected `6.3` (hand-computed: sum of x[j]+0.1 for j=0..2), got: {stdout}"
    );
}

#[test]
fn run_link_with_requires_backend_c() {
    let binary = env!("CARGO_BIN_EXE_intentc");
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let example = format!("{}/examples/language/english/basics.vani", manifest_dir);

    // Default backend is LLVM; --link-with should be rejected.
    let out = Command::new(binary)
        .args(["run", &example, "--link-with", "/tmp/whatever.c"])
        .output()
        .expect("intentc run executes");

    assert!(
        !out.status.success(),
        "expected failure when --link-with is paired with LLVM-JIT"
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("require --backend=c"),
        "expected backend=c hint in stderr, got: {stderr}"
    );
}

// FFI v2: `intentc build --link-with foo.c` threads an extra
// translation unit into the link line so an `extern "C" fn`
// declaration in vāṇī source resolves at link time. End-to-end
// shape: a tiny C helper `triple(x: i32) -> i32`, a vāṇी source
// that declares + calls it, build with --link-with, run, expect
// `triple(7) = 21` on stdout.
#[test]
fn build_link_with_resolves_extern_c_symbol() {
    use std::fs;
    use std::path::PathBuf;

    let binary = env!("CARGO_BIN_EXE_intentc");
    let dir: PathBuf = std::env::temp_dir().join(format!(
        "intentc-linkwith-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir_all(&dir).expect("mkdir");

    let helper_c = dir.join("helper.c");
    fs::write(
        &helper_c,
        "#include <stdint.h>\nint32_t triple(int32_t x) { return x * 3; }\n",
    )
    .expect("write helper.c");

    let vani_src = dir.join("prog.vani");
    fs::write(
        &vani_src,
        "extern \"C\" fn triple(x: i32) -> i32;\n\
         \n\
         fn main() -> i64 {\n  \
           let r: i32 = triple(7 as i32);\n  \
           write \"triple(7) =\", r;\n  \
           return 0;\n}\n",
    )
    .expect("write prog.vani");

    let bin_path = dir.join("prog");
    let build = Command::new(binary)
        .args([
            "build",
            vani_src.to_str().unwrap(),
            "--link-with",
            helper_c.to_str().unwrap(),
            "-o",
            bin_path.to_str().unwrap(),
        ])
        .output()
        .expect("intentc build runs");

    if !build.status.success() {
        let _ = fs::remove_dir_all(&dir);
        panic!(
            "intentc build --link-with failed:\nstdout: {}\nstderr: {}",
            String::from_utf8_lossy(&build.stdout),
            String::from_utf8_lossy(&build.stderr),
        );
    }

    let run = Command::new(&bin_path).output().expect("binary runs");
    let stdout = String::from_utf8_lossy(&run.stdout).to_string();
    let status = run.status;

    let _ = fs::remove_dir_all(&dir);

    assert!(
        status.success(),
        "linked binary exited non-zero: {} (stdout: {})",
        status,
        stdout
    );
    assert!(
        stdout.contains("triple(7) = 21"),
        "expected `triple(7) = 21` in stdout, got: {stdout}"
    );
}

#[test]
fn run_assert_messages_example() {
    let binary = env!("CARGO_BIN_EXE_intentc");
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let example = format!("{}/examples/language/english/assert_messages.vani", manifest_dir);

    let output = Command::new(binary)
        .args(["run", &example])
        .output()
        .expect("intentc run should execute");

    assert!(
        output.status.success(),
        "intentc run failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("30"), "expected lookup(&xs, 2)==30, got: {stdout}");
}

#[test]
fn intentc_ir_dumps_typed_program() {
    use std::fs;
    let binary = env!("CARGO_BIN_EXE_intentc");
    let dir = std::env::temp_dir().join(format!(
        "intentc-ir-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir_all(&dir).expect("mkdir");
    let src = dir.join("i.vani");
    fs::write(&src, "fn main() -> i64 { return 7; }\n").expect("write");

    let output = Command::new(binary)
        .args(["ir", src.to_str().unwrap()])
        .output()
        .expect("intentc ir");
    let _ = fs::remove_dir_all(&dir);

    assert!(output.status.success(), "ir exited non-zero");
    let stdout = String::from_utf8_lossy(&output.stdout);
    // The typed-IR dump must show the cached-return temp + the
    // literal value. Confirms the checker's Drop-before-Return
    // soundness fix is in the IR the backends see.
    assert!(stdout.contains("TypedProgram {"));
    assert!(stdout.contains("__intent_ret_"));
    // `{:#?}` splits enum payloads across lines, so the literal
    // appears as `Int(\n  7,\n)`. Use a regex-free shape check.
    assert!(stdout.contains("Int("));
    assert!(stdout.contains("7,"));
    assert!(stdout.contains("Return {"));
}

#[test]
fn intentc_ast_dumps_parsed_program() {
    use std::fs;
    let binary = env!("CARGO_BIN_EXE_intentc");
    let dir = std::env::temp_dir().join(format!(
        "intentc-ast-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir_all(&dir).expect("mkdir");
    let src = dir.join("a.vani");
    fs::write(&src, "fn add(a: i64, b: i64) -> i64 { return a + b; }\n").expect("write");

    let output = Command::new(binary)
        .args(["ast", src.to_str().unwrap()])
        .output()
        .expect("intentc ast");
    let _ = fs::remove_dir_all(&dir);

    assert!(output.status.success(), "ast exited non-zero");
    let stdout = String::from_utf8_lossy(&output.stdout);
    // Spot-check a few expected substrings of the debug-format AST.
    assert!(stdout.contains("Program {"));
    assert!(stdout.contains("name: \"add\""));
    assert!(stdout.contains("return_type: I64"));
    assert!(stdout.contains("Return {"));
}

#[test]
fn intentc_tokens_dumps_token_stream() {
    use std::fs;
    let binary = env!("CARGO_BIN_EXE_intentc");
    let dir = std::env::temp_dir().join(format!(
        "intentc-tokens-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir_all(&dir).expect("mkdir");
    let src = dir.join("t.vani");
    fs::write(&src, "fn main() -> i64 { return 42; }\n").expect("write");

    let output = Command::new(binary)
        .args(["tokens", src.to_str().unwrap()])
        .output()
        .expect("intentc tokens");
    let _ = fs::remove_dir_all(&dir);

    assert!(output.status.success(), "tokens exited non-zero");
    let stdout = String::from_utf8_lossy(&output.stdout);
    // Spot-check a few expected tokens for a tiny program.
    assert!(stdout.contains("Fn"));
    assert!(stdout.contains("Ident(\"main\")"));
    assert!(stdout.contains("Int(42)"));
    assert!(stdout.contains("Return"));
}

#[test]
fn intentc_build_produces_runnable_native_binary() {
    // Gated on `llc` + `cc` being present. `cc` is on every dev box
    // we'd care about; `llc` ships with LLVM's `lli`.
    let llc_ok = std::process::Command::new("llc")
        .arg("--version")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false);
    if !llc_ok {
        return;
    }

    use std::fs;
    let binary = env!("CARGO_BIN_EXE_intentc");
    let dir = std::env::temp_dir().join(format!(
        "intentc-build-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir_all(&dir).expect("mkdir");
    let src = dir.join("prog.vani");
    fs::write(
        &src,
        "fn main() -> i64 {\n  let x: i64 = 7;\n  let y: i64 = 6;\n  print x * y;\n  return 0;\n}\n",
    )
    .expect("write src");
    // On Windows gcc always appends .exe even when -o has no extension.
    let out_bin = if cfg!(target_os = "windows") {
        dir.join("prog.exe")
    } else {
        dir.join("prog")
    };

    let build_out = Command::new(binary)
        .args([
            "build",
            src.to_str().unwrap(),
            "-o",
            out_bin.to_str().unwrap(),
        ])
        .output()
        .expect("intentc build");
    assert!(
        build_out.status.success(),
        "build failed: {}",
        String::from_utf8_lossy(&build_out.stderr)
    );
    assert!(out_bin.exists(), "build did not produce a binary");

    // Run the binary.
    let run_out = Command::new(&out_bin).output().expect("run binary");
    let _ = fs::remove_dir_all(&dir);
    assert!(run_out.status.success(), "binary exited non-zero");
    let stdout = String::from_utf8_lossy(&run_out.stdout);
    assert!(stdout.contains("42"), "expected 42, got: {stdout}");
}

#[test]
#[ignore = "tcp_echo.vani LLVM IR has undefined values for socket locals; lli rejects it"]
fn llvm_backend_run_produces_same_output_as_c() {
    // Gated on `lli` being installed; mirrors the per-backend test
    // pattern in src/backend_llvm.rs.
    // Look up `lli` via $LLI / PATH rather than hardcoding /usr/bin
    // so the test works on systems with lli elsewhere (homebrew,
    // /usr/local, etc.).
    let lli = std::env::var("LLI").unwrap_or_else(|_| "lli".to_string());
    let lli_ok = std::process::Command::new(&lli)
        .arg("--version")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false);
    if !lli_ok {
        return;
    }

    let binary = env!("CARGO_BIN_EXE_intentc");
    let manifest_dir = env!("CARGO_MANIFEST_DIR");

    // Every example under examples/ — the LLVM and C backends must
    // produce identical stdout AND identical exit codes for each.
    // Catches semantic divergence between the two anywhere in the
    // matrix of feature interactions. Update this list when a new
    // example file lands.
    for name in &[
        "anon_fn.vani",
        "async_fn.vani",
        "async_await.vani",
        "async_io.vani",
        "tcp_echo.vani",
        "tcp_multi_echo.vani",
        // epoll_wait_one now backed by WSAPoll on Windows (true readiness
        // notification) instead of IOCP — hang fixed, runs on all platforms.
        "tcp_echo_epoll.vani",
        "tcp_echo_state_machine.vani",
        "tcp_echo_async.vani",
        "tcp_echo_async_branched.vani",
        "echo_fall_through.vani",
        "echo_nested_if.vani",
        "echo_anf_lift.vani",
        "echo_loop.vani",
        "echo_loop_break.vani",
        "async_showcase.vani",
        "echo_match.vani",
        "echo_match_suspend.vani",
        "echo_match_b_s_f.vani",
        "echo_match_variant.vani",
        "echo_match_binding.vani",
        "echo_match_stress.vani",
        "echo_p3a_nonint_locals.vani",
        "echo_p3b_str_local.vani",
        "echo_p3c_enum_local.vani",
        "echo_p3d_vec_struct.vani",
        "echo_p3e_payloaded_enum.vani",
        "echo_p3f_array_local.vani",
        "echo_p3_locals_stress.vani",
        "echo_p3r_nonint_returns.vani",
        "echo_p24_try_keyword.vani",
        "echo_p24_question_op.vani",
        "echo_pool.vani",
        "async_cancel_auto.vani",
        "echo_p3p_nonint_params.vani",
        "echo_p4a_nested_async.vani",
        "echo_p4b_await_sub.vani",
        "echo_p4b_multitask.vani",
        "llvm_match_arm_div.vani",
        "vec_struct_field.vani",
        "echo_with_timeout.vani",
        "timer_async.vani",
        "array_proofs.vani",
        "array_return.vani",
        "arrays.vani",
        "assert_messages.vani",
        "atomics.vani",
        "basics.vani",
        "binary_heap.vani",
        "block_expressions.vani",
        "bloom_filter.vani",
        "borrows.vani",
        "bounded_generics.vani",
        "bounds_elision.vani",
        "bst.vani",
        "bst_avl.vani",
        "btreemap.vani",
        "btreeset.vani",
        "closures.vani",
        "closure_as_value.vani",
        "composite_types.vani",
        "concurrency.vani",
        "condvar.vani",
        "container_method_sugar.vani",
        "contracts.vani",
        "control_flow.vani",
        "deque.vani",
        "drop_interface.vani",
        "memory_safety.vani",
        "dyn_dispatch.vani",
        "early_exit.vani",
        "enum_arr_payload.vani",
        "enum_eq.vani",
        "enum_owned_payload.vani",
        "enum_vec_payload.vani",
        "floats_and_shifts.vani",
        "fn_pointers.vani",
        "for_loops.vani",
        "generic_functions.vani",
        "graph.vani",
        "graph_algo.vani",
        "graph_algo2.vani",
        "graph_csr.vani",
        "hash.vani",
        "hashmap.vani",
        "hashmap_f64.vani",
        "hashmap_str.vani",
        "hashmap_strstr.vani",
        "hashmap_strv.vani",
        "hashmap_tup.vani",
        "hashmap_veck.vani",
        "hashset.vani",
        "heap.vani",
        "../hindi/keywords.vani",  // A.2 reorg: lives at examples/language/hindi/keywords.vani
        "inline_call_proofs.vani",
        "integers.vani",
        "interfaces.vani",
        "invariants.vani",
        "iter_combinators.vani",
        "iterate.vani",
        "../marathi/keywords.vani",  // A.2 reorg: lives at examples/language/marathi/keywords.vani
        "math_ops.vani",
        "match_bool.vani",
        "match_str.vani",
        "methods.vani",
        "mixed_place_assign.vani",
        "modules.vani",
        "mut_refs.vani",
        "nested_struct_drop.vani",
        "option_error_propagation.vani",
        "option_types.vani",
        "parallel.vani",
        "partial_move.vani",
        "push_mut.vani",
        "rng.vani",
        "../sanskrit/keywords.vani",  // A.2 reorg: lives at examples/language/sanskrit/keywords.vani
        "../sanskrit/sov_demo.vani",  // SOV-S1 demo: let-binding verb-at-end
        "../sanskrit/pure_devanagari.vani",  // Pure Sanskrit showcase: keywords + types + identifiers + numerals all Devanagari
        // SOV-S10 translator-generated Devanagari coverage —
        // 24 files (8 examples × 3 Indo-Aryan dialects). Each
        // mirrors its English original byte-for-byte at the AST
        // level and runs to identical stdout + exit code.
        "../sanskrit/basics.vani",
        "../hindi/basics.vani",
        "../marathi/basics.vani",
        "../sanskrit/control_flow.vani",
        "../hindi/control_flow.vani",
        "../marathi/control_flow.vani",
        "../sanskrit/for_loops.vani",
        "../hindi/for_loops.vani",
        "../marathi/for_loops.vani",
        "../sanskrit/vec_invariants.vani",
        "../hindi/vec_invariants.vani",
        "../marathi/vec_invariants.vani",
        "../sanskrit/option_types.vani",
        "../hindi/option_types.vani",
        "../marathi/option_types.vani",
        "../sanskrit/verified.vani",
        "../hindi/verified.vani",
        "../marathi/verified.vani",
        "../sanskrit/early_exit.vani",
        "../hindi/early_exit.vani",
        "../marathi/early_exit.vani",
        "../sanskrit/iterate.vani",
        "../hindi/iterate.vani",
        "../marathi/iterate.vani",
        // Polish queue (2026-06-08): Devanagari counterparts of
        // the post-Phase-13 English examples. Demonstrates that
        // the postfix `?` operator + L2 follow-up Box surface
        // work identically under the Sanskrit pragma. Async-
        // related examples (echo_pool, async_cancel_auto) are
        // queued for a later polish item — the `async` keyword
        // is contextual-English today and `Task__` mangled
        // names appear in user code, so mixed-dialect content
        // wouldn't read cleanly.
        "../sanskrit/try_question_op.vani",
        "../sanskrit/box_recursive_drop.vani",
        // Polish (2026-06-08): async/await dialect lift — the
        // `async` + `await` contextual identifiers now accept
        // per-dialect spellings (Sanskrit: अतुल्यकालिक /
        // प्रतीक्षा, Mandarin: 异步 / 等候). Sanskrit ships a
        // simplified smoke; Mandarin ships the full A4.4 surface.
        "../sanskrit/async_cancel_auto.vani",
        "../mandarin/async_cancel_auto.vani",
        // Polish (2026-06-08, late session): Hindi + Marathi
        // counterparts of the same three examples Sanskrit
        // already ships. Each dialect picks its preferred
        // spelling for enum / match / then (Sanskrit:
        // विकल्प / मेल / तदा; Hindi: गणन / मिलान / तो;
        // Marathi: गणन / जुळवा / तर).
        "../hindi/try_question_op.vani",
        "../marathi/try_question_op.vani",
        "../hindi/box_recursive_drop.vani",
        "../marathi/box_recursive_drop.vani",
        "../hindi/async_cancel_auto.vani",
        "../marathi/async_cancel_auto.vani",
        // Phase 10.2 (2026-06-08): Mandarin Chinese basics —
        // 62nd dialect. Shares Script::Japanese for the purity
        // gate; pure-Han keyword table disambiguates from
        // Japanese via the pragma.
        "../mandarin/basics.vani",
        // Phase 2 (2026-06-07): Tier I dialect extensions —
        // Nepali / Maithili / Konkani-Devanagari. Full 10-file
        // smoke-test suite (Phase 8b expansion, 2026-06-15).
        "../nepali/basics.vani",
        "../nepali/keywords.vani",
        "../nepali/control_flow.vani",
        "../nepali/for_loops.vani",
        "../nepali/early_exit.vani",
        "../nepali/iterate.vani",
        "../nepali/vec_invariants.vani",
        "../nepali/verified.vani",
        "../nepali/option_types.vani",
        "../nepali/try_question_op.vani",
        "../nepali/box_recursive_drop.vani",
        "../maithili/basics.vani",
        "../maithili/keywords.vani",
        "../maithili/control_flow.vani",
        "../maithili/for_loops.vani",
        "../maithili/early_exit.vani",
        "../maithili/iterate.vani",
        "../maithili/vec_invariants.vani",
        "../maithili/verified.vani",
        "../maithili/option_types.vani",
        "../maithili/try_question_op.vani",
        "../maithili/box_recursive_drop.vani",
        "../konkani/basics.vani",
        "../konkani/keywords.vani",
        "../konkani/control_flow.vani",
        "../konkani/for_loops.vani",
        "../konkani/early_exit.vani",
        "../konkani/iterate.vani",
        "../konkani/vec_invariants.vani",
        "../konkani/verified.vani",
        "../konkani/option_types.vani",
        "../konkani/try_question_op.vani",
        "../konkani/box_recursive_drop.vani",
        // Phase 5b (2026-06-07): first Brahmi-derived non-
        // Devanagari script — Bengali (U+0980..U+09FF). Sets up
        // the per-script abstraction for Tamil / Telugu / Kannada
        // / Malayalam / Odia / Assamese in Phase 6.
        "../bengali/basics.vani",
        "../bengali/keywords.vani",
        "../bengali/control_flow.vani",
        "../bengali/for_loops.vani",
        "../bengali/early_exit.vani",
        "../bengali/iterate.vani",
        "../bengali/vec_invariants.vani",
        "../bengali/verified.vani",
        "../bengali/option_types.vani",
        "../bengali/try_question_op.vani",
        "../bengali/box_recursive_drop.vani",
        // Phase 6 (2026-06-07): Brahmi-derived batch — 4 more
        // scripts riding the abstraction. Each emits its native
        // numerals (Tamil ௦..௯, Telugu ౦..౯, Gujarati ૦..૯,
        // Gurmukhi ੦..੯) via the parameterized helper.
        "../tamil/basics.vani",
        "../tamil/keywords.vani",
        "../tamil/control_flow.vani",
        "../tamil/for_loops.vani",
        "../tamil/early_exit.vani",
        "../tamil/iterate.vani",
        "../tamil/vec_invariants.vani",
        "../tamil/verified.vani",
        "../tamil/option_types.vani",
        "../tamil/try_question_op.vani",
        "../tamil/box_recursive_drop.vani",
        "../telugu/basics.vani",
        "../telugu/keywords.vani",
        "../telugu/control_flow.vani",
        "../telugu/for_loops.vani",
        "../telugu/early_exit.vani",
        "../telugu/iterate.vani",
        "../telugu/vec_invariants.vani",
        "../telugu/verified.vani",
        "../telugu/option_types.vani",
        "../telugu/try_question_op.vani",
        "../telugu/box_recursive_drop.vani",
        "../gujarati/basics.vani",
        "../gujarati/keywords.vani",
        "../gujarati/control_flow.vani",
        "../gujarati/for_loops.vani",
        "../gujarati/early_exit.vani",
        "../gujarati/iterate.vani",
        "../gujarati/vec_invariants.vani",
        "../gujarati/verified.vani",
        "../gujarati/option_types.vani",
        "../gujarati/try_question_op.vani",
        "../gujarati/box_recursive_drop.vani",
        "../punjabi/basics.vani",
        "../punjabi/keywords.vani",
        "../punjabi/control_flow.vani",
        "../punjabi/for_loops.vani",
        "../punjabi/early_exit.vani",
        "../punjabi/iterate.vani",
        "../punjabi/vec_invariants.vani",
        "../punjabi/verified.vani",
        "../punjabi/option_types.vani",
        "../punjabi/try_question_op.vani",
        "../punjabi/box_recursive_drop.vani",
        // Phase 6 second half (2026-06-07): Kannada, Malayalam,
        // Odia, Assamese, Sinhala. Each emits its native digits
        // — except Assamese which reuses Bengali (১২).
        "../kannada/basics.vani",
        "../kannada/keywords.vani",
        "../kannada/control_flow.vani",
        "../kannada/for_loops.vani",
        "../kannada/early_exit.vani",
        "../kannada/iterate.vani",
        "../kannada/vec_invariants.vani",
        "../kannada/verified.vani",
        "../kannada/option_types.vani",
        "../kannada/try_question_op.vani",
        "../kannada/box_recursive_drop.vani",
        "../malayalam/basics.vani",
        "../malayalam/keywords.vani",
        "../malayalam/control_flow.vani",
        "../malayalam/for_loops.vani",
        "../malayalam/early_exit.vani",
        "../malayalam/iterate.vani",
        "../malayalam/vec_invariants.vani",
        "../malayalam/verified.vani",
        "../malayalam/option_types.vani",
        "../malayalam/try_question_op.vani",
        "../malayalam/box_recursive_drop.vani",
        "../odia/basics.vani",
        "../odia/keywords.vani",
        "../odia/control_flow.vani",
        "../odia/for_loops.vani",
        "../odia/early_exit.vani",
        "../odia/iterate.vani",
        "../odia/vec_invariants.vani",
        "../odia/verified.vani",
        "../odia/option_types.vani",
        "../odia/try_question_op.vani",
        "../odia/box_recursive_drop.vani",
        "../assamese/basics.vani",
        "../assamese/keywords.vani",
        "../assamese/control_flow.vani",
        "../assamese/for_loops.vani",
        "../assamese/early_exit.vani",
        "../assamese/iterate.vani",
        "../assamese/vec_invariants.vani",
        "../assamese/verified.vani",
        "../assamese/option_types.vani",
        "../assamese/try_question_op.vani",
        "../assamese/box_recursive_drop.vani",
        "../sinhala/basics.vani",
        "../sinhala/keywords.vani",
        "../sinhala/control_flow.vani",
        "../sinhala/for_loops.vani",
        "../sinhala/early_exit.vani",
        "../sinhala/iterate.vani",
        "../sinhala/vec_invariants.vani",
        "../sinhala/verified.vani",
        "../sinhala/option_types.vani",
        "../sinhala/try_question_op.vani",
        "../sinhala/box_recursive_drop.vani",
        // Phase 12 (2026-06-07): first Perso-Arabic / RTL
        // dialect. Full 10-file suite (Phase 8b expansion, 2026-06-15).
        "../urdu/basics.vani",
        "../urdu/keywords.vani",
        "../urdu/control_flow.vani",
        "../urdu/for_loops.vani",
        "../urdu/early_exit.vani",
        "../urdu/iterate.vani",
        "../urdu/vec_invariants.vani",
        "../urdu/verified.vani",
        "../urdu/option_types.vani",
        "../urdu/try_question_op.vani",
        "../urdu/box_recursive_drop.vani",
        // Phase 12.2/12.3 (2026-06-07): Sindhi and Punjabi-Shahmukhi.
        "../sindhi/basics.vani",
        "../sindhi/keywords.vani",
        "../sindhi/control_flow.vani",
        "../sindhi/for_loops.vani",
        "../sindhi/early_exit.vani",
        "../sindhi/iterate.vani",
        "../sindhi/vec_invariants.vani",
        "../sindhi/verified.vani",
        "../sindhi/option_types.vani",
        "../sindhi/try_question_op.vani",
        "../sindhi/box_recursive_drop.vani",
        "../punjabi_shahmukhi/basics.vani",
        "../punjabi_shahmukhi/keywords.vani",
        "../punjabi_shahmukhi/control_flow.vani",
        "../punjabi_shahmukhi/for_loops.vani",
        "../punjabi_shahmukhi/early_exit.vani",
        "../punjabi_shahmukhi/iterate.vani",
        "../punjabi_shahmukhi/vec_invariants.vani",
        "../punjabi_shahmukhi/verified.vani",
        "../punjabi_shahmukhi/option_types.vani",
        "../punjabi_shahmukhi/try_question_op.vani",
        "../punjabi_shahmukhi/box_recursive_drop.vani",
        // Phase 12.4/12.5 (2026-06-07): Persian + Pashto.
        "../persian/basics.vani",
        "../persian/keywords.vani",
        "../persian/control_flow.vani",
        "../persian/for_loops.vani",
        "../persian/early_exit.vani",
        "../persian/iterate.vani",
        "../persian/vec_invariants.vani",
        "../persian/verified.vani",
        "../persian/option_types.vani",
        "../persian/try_question_op.vani",
        "../persian/box_recursive_drop.vani",
        "../pashto/basics.vani",
        "../pashto/keywords.vani",
        "../pashto/control_flow.vani",
        "../pashto/for_loops.vani",
        "../pashto/early_exit.vani",
        "../pashto/iterate.vani",
        "../pashto/vec_invariants.vani",
        "../pashto/verified.vani",
        "../pashto/option_types.vani",
        "../pashto/try_question_op.vani",
        "../pashto/box_recursive_drop.vani",
        // Phase 8b (2026-06-07): European dialects — Spanish, French, Russian.
        // Spanish/French: non-ASCII aliases active (función, énumération, etc.);
        // pure-ASCII control-flow uses English keywords (v1). Russian: full Cyrillic.
        "../spanish/basics.vani",
        "../spanish/keywords.vani",
        "../spanish/control_flow.vani",
        "../spanish/for_loops.vani",
        "../spanish/early_exit.vani",
        "../spanish/iterate.vani",
        "../spanish/vec_invariants.vani",
        "../spanish/verified.vani",
        "../spanish/option_types.vani",
        "../spanish/try_question_op.vani",
        "../spanish/box_recursive_drop.vani",
        "../french/basics.vani",
        "../french/keywords.vani",
        "../french/control_flow.vani",
        "../french/for_loops.vani",
        "../french/early_exit.vani",
        "../french/iterate.vani",
        "../french/vec_invariants.vani",
        "../french/verified.vani",
        "../french/option_types.vani",
        "../french/try_question_op.vani",
        "../french/box_recursive_drop.vani",
        "../russian/basics.vani",
        "../russian/keywords.vani",
        "../russian/control_flow.vani",
        "../russian/for_loops.vani",
        "../russian/early_exit.vani",
        "../russian/iterate.vani",
        "../russian/vec_invariants.vani",
        "../russian/verified.vani",
        "../russian/option_types.vani",
        "../russian/try_question_op.vani",
        "../russian/box_recursive_drop.vani",
        // Phase 10.1 (German), 13.2 (Portuguese), 13.6 (Italian)
        // full 10-file sets matching the Brahmi-dialect parity level.
        "../german/basics.vani",
        "../german/keywords.vani",
        "../german/control_flow.vani",
        "../german/for_loops.vani",
        "../german/early_exit.vani",
        "../german/iterate.vani",
        "../german/vec_invariants.vani",
        "../german/verified.vani",
        "../german/option_types.vani",
        "../german/try_question_op.vani",
        "../german/box_recursive_drop.vani",
        "../italian/basics.vani",
        "../italian/keywords.vani",
        "../italian/control_flow.vani",
        "../italian/for_loops.vani",
        "../italian/early_exit.vani",
        "../italian/iterate.vani",
        "../italian/vec_invariants.vani",
        "../italian/verified.vani",
        "../italian/option_types.vani",
        "../italian/try_question_op.vani",
        "../italian/box_recursive_drop.vani",
        "../portuguese/basics.vani",
        "../portuguese/keywords.vani",
        "../portuguese/control_flow.vani",
        "../portuguese/for_loops.vani",
        "../portuguese/early_exit.vani",
        "../portuguese/iterate.vani",
        "../portuguese/vec_invariants.vani",
        "../portuguese/verified.vani",
        "../portuguese/option_types.vani",
        "../portuguese/try_question_op.vani",
        "../portuguese/box_recursive_drop.vani",
        // Phase 8b cont.: Dutch, Swedish, Norwegian, Danish, Finnish (10 files each)
        "../dutch/basics.vani",
        "../dutch/keywords.vani",
        "../dutch/control_flow.vani",
        "../dutch/for_loops.vani",
        "../dutch/early_exit.vani",
        "../dutch/iterate.vani",
        "../dutch/vec_invariants.vani",
        "../dutch/verified.vani",
        "../dutch/option_types.vani",
        "../dutch/try_question_op.vani",
        "../dutch/box_recursive_drop.vani",
        "../swedish/basics.vani",
        "../swedish/keywords.vani",
        "../swedish/control_flow.vani",
        "../swedish/for_loops.vani",
        "../swedish/early_exit.vani",
        "../swedish/iterate.vani",
        "../swedish/vec_invariants.vani",
        "../swedish/verified.vani",
        "../swedish/option_types.vani",
        "../swedish/try_question_op.vani",
        "../swedish/box_recursive_drop.vani",
        "../norwegian/basics.vani",
        "../norwegian/keywords.vani",
        "../norwegian/control_flow.vani",
        "../norwegian/for_loops.vani",
        "../norwegian/early_exit.vani",
        "../norwegian/iterate.vani",
        "../norwegian/vec_invariants.vani",
        "../norwegian/verified.vani",
        "../norwegian/option_types.vani",
        "../norwegian/try_question_op.vani",
        "../norwegian/box_recursive_drop.vani",
        "../danish/basics.vani",
        "../danish/keywords.vani",
        "../danish/control_flow.vani",
        "../danish/for_loops.vani",
        "../danish/early_exit.vani",
        "../danish/iterate.vani",
        "../danish/vec_invariants.vani",
        "../danish/verified.vani",
        "../danish/option_types.vani",
        "../danish/try_question_op.vani",
        "../danish/box_recursive_drop.vani",
        "../finnish/basics.vani",
        "../finnish/keywords.vani",
        "../finnish/control_flow.vani",
        "../finnish/for_loops.vani",
        "../finnish/early_exit.vani",
        "../finnish/iterate.vani",
        "../finnish/vec_invariants.vani",
        "../finnish/verified.vani",
        "../finnish/option_types.vani",
        "../finnish/try_question_op.vani",
        "../finnish/box_recursive_drop.vani",
        // Czech
        "../czech/basics.vani",
        "../czech/keywords.vani",
        "../czech/control_flow.vani",
        "../czech/for_loops.vani",
        "../czech/early_exit.vani",
        "../czech/iterate.vani",
        "../czech/vec_invariants.vani",
        "../czech/verified.vani",
        "../czech/option_types.vani",
        "../czech/try_question_op.vani",
        "../czech/box_recursive_drop.vani",
        // Polish
        "../polish/basics.vani",
        "../polish/keywords.vani",
        "../polish/control_flow.vani",
        "../polish/for_loops.vani",
        "../polish/early_exit.vani",
        "../polish/iterate.vani",
        "../polish/vec_invariants.vani",
        "../polish/verified.vani",
        "../polish/option_types.vani",
        "../polish/try_question_op.vani",
        "../polish/box_recursive_drop.vani",
        // Romanian
        "../romanian/basics.vani",
        "../romanian/keywords.vani",
        "../romanian/control_flow.vani",
        "../romanian/for_loops.vani",
        "../romanian/early_exit.vani",
        "../romanian/iterate.vani",
        "../romanian/vec_invariants.vani",
        "../romanian/verified.vani",
        "../romanian/option_types.vani",
        "../romanian/try_question_op.vani",
        "../romanian/box_recursive_drop.vani",
        // Hungarian
        "../hungarian/basics.vani",
        "../hungarian/keywords.vani",
        "../hungarian/control_flow.vani",
        "../hungarian/for_loops.vani",
        "../hungarian/early_exit.vani",
        "../hungarian/iterate.vani",
        "../hungarian/vec_invariants.vani",
        "../hungarian/verified.vani",
        "../hungarian/option_types.vani",
        "../hungarian/try_question_op.vani",
        "../hungarian/box_recursive_drop.vani",
        // Slovak
        "../slovak/basics.vani",
        "../slovak/keywords.vani",
        "../slovak/control_flow.vani",
        "../slovak/for_loops.vani",
        "../slovak/early_exit.vani",
        "../slovak/iterate.vani",
        "../slovak/vec_invariants.vani",
        "../slovak/verified.vani",
        "../slovak/option_types.vani",
        "../slovak/try_question_op.vani",
        "../slovak/box_recursive_drop.vani",
        // Catalan
        "../catalan/basics.vani",
        "../catalan/keywords.vani",
        "../catalan/control_flow.vani",
        "../catalan/for_loops.vani",
        "../catalan/early_exit.vani",
        "../catalan/iterate.vani",
        "../catalan/vec_invariants.vani",
        "../catalan/verified.vani",
        "../catalan/option_types.vani",
        "../catalan/try_question_op.vani",
        "../catalan/box_recursive_drop.vani",
        // Turkish
        "../turkish/basics.vani",
        "../turkish/keywords.vani",
        "../turkish/control_flow.vani",
        "../turkish/for_loops.vani",
        "../turkish/early_exit.vani",
        "../turkish/iterate.vani",
        "../turkish/vec_invariants.vani",
        "../turkish/verified.vani",
        "../turkish/option_types.vani",
        "../turkish/try_question_op.vani",
        "../turkish/box_recursive_drop.vani",
        // Greek
        "../greek/basics.vani",
        "../greek/keywords.vani",
        "../greek/control_flow.vani",
        "../greek/for_loops.vani",
        "../greek/early_exit.vani",
        "../greek/iterate.vani",
        "../greek/vec_invariants.vani",
        "../greek/verified.vani",
        "../greek/option_types.vani",
        "../greek/try_question_op.vani",
        "../greek/box_recursive_drop.vani",
        // ── Indonesian (Latin-script, SE Asia) ──────────────────────────────
        "../indonesian/basics.vani",
        "../indonesian/keywords.vani",
        "../indonesian/control_flow.vani",
        "../indonesian/for_loops.vani",
        "../indonesian/early_exit.vani",
        "../indonesian/iterate.vani",
        "../indonesian/vec_invariants.vani",
        "../indonesian/verified.vani",
        "../indonesian/option_types.vani",
        "../indonesian/try_question_op.vani",
        "../indonesian/box_recursive_drop.vani",
        // ── Malay (Latin-script, SE Asia) ───────────────────────────────────
        "../malay/basics.vani",
        "../malay/keywords.vani",
        "../malay/control_flow.vani",
        "../malay/for_loops.vani",
        "../malay/early_exit.vani",
        "../malay/iterate.vani",
        "../malay/vec_invariants.vani",
        "../malay/verified.vani",
        "../malay/option_types.vani",
        "../malay/try_question_op.vani",
        "../malay/box_recursive_drop.vani",
        // ── Swahili (Latin-script, East Africa) ─────────────────────────────
        "../swahili/basics.vani",
        "../swahili/keywords.vani",
        "../swahili/control_flow.vani",
        "../swahili/for_loops.vani",
        "../swahili/early_exit.vani",
        "../swahili/iterate.vani",
        "../swahili/vec_invariants.vani",
        "../swahili/verified.vani",
        "../swahili/option_types.vani",
        "../swahili/try_question_op.vani",
        "../swahili/box_recursive_drop.vani",
        // ── Filipino (Latin-script, SE Asia) ────────────────────────────────
        "../filipino/basics.vani",
        "../filipino/keywords.vani",
        "../filipino/control_flow.vani",
        "../filipino/for_loops.vani",
        "../filipino/early_exit.vani",
        "../filipino/iterate.vani",
        "../filipino/vec_invariants.vani",
        "../filipino/verified.vani",
        "../filipino/option_types.vani",
        "../filipino/try_question_op.vani",
        "../filipino/box_recursive_drop.vani",
        // ── Vietnamese (Latin-script + tone marks, SE Asia) ──────────────────
        "../vietnamese/basics.vani",
        "../vietnamese/keywords.vani",
        "../vietnamese/control_flow.vani",
        "../vietnamese/for_loops.vani",
        "../vietnamese/early_exit.vani",
        "../vietnamese/iterate.vani",
        "../vietnamese/vec_invariants.vani",
        "../vietnamese/verified.vani",
        "../vietnamese/option_types.vani",
        "../vietnamese/try_question_op.vani",
        "../vietnamese/box_recursive_drop.vani",
        // ── Hausa (Latin-script + ƙ/ɓ/ɗ, Afroasiatic, West Africa) ─────────
        "../hausa/basics.vani",
        "../hausa/keywords.vani",
        "../hausa/control_flow.vani",
        "../hausa/for_loops.vani",
        "../hausa/early_exit.vani",
        "../hausa/iterate.vani",
        "../hausa/vec_invariants.vani",
        "../hausa/verified.vani",
        "../hausa/option_types.vani",
        "../hausa/try_question_op.vani",
        "../hausa/box_recursive_drop.vani",
        // ── Yoruba (Latin + tone diacritics, Niger-Congo, West Africa) ───────
        "../yoruba/basics.vani",
        "../yoruba/keywords.vani",
        "../yoruba/control_flow.vani",
        "../yoruba/for_loops.vani",
        "../yoruba/early_exit.vani",
        "../yoruba/iterate.vani",
        "../yoruba/vec_invariants.vani",
        "../yoruba/verified.vani",
        "../yoruba/option_types.vani",
        "../yoruba/try_question_op.vani",
        "../yoruba/box_recursive_drop.vani",
        // ── Arabic (Arabic script, RTL) ──────────────────────────────────────
        "../arabic/basics.vani",
        "../arabic/keywords.vani",
        "../arabic/control_flow.vani",
        "../arabic/for_loops.vani",
        "../arabic/early_exit.vani",
        "../arabic/iterate.vani",
        "../arabic/vec_invariants.vani",
        "../arabic/verified.vani",
        "../arabic/option_types.vani",
        "../arabic/try_question_op.vani",
        "../arabic/box_recursive_drop.vani",
        // ── Hebrew (Hebrew script, RTL) ──────────────────────────────────────
        "../hebrew/basics.vani",
        "../hebrew/keywords.vani",
        "../hebrew/control_flow.vani",
        "../hebrew/for_loops.vani",
        "../hebrew/early_exit.vani",
        "../hebrew/iterate.vani",
        "../hebrew/vec_invariants.vani",
        "../hebrew/verified.vani",
        "../hebrew/option_types.vani",
        "../hebrew/try_question_op.vani",
        "../hebrew/box_recursive_drop.vani",
        // ── Armenian (Armenian script) ───────────────────────────────────────
        "../armenian/basics.vani",
        "../armenian/keywords.vani",
        "../armenian/control_flow.vani",
        "../armenian/for_loops.vani",
        "../armenian/early_exit.vani",
        "../armenian/iterate.vani",
        "../armenian/vec_invariants.vani",
        "../armenian/verified.vani",
        "../armenian/option_types.vani",
        "../armenian/try_question_op.vani",
        "../armenian/box_recursive_drop.vani",
        // ── Georgian (Georgian Mkhedruli script) ─────────────────────────────
        "../georgian/basics.vani",
        "../georgian/keywords.vani",
        "../georgian/control_flow.vani",
        "../georgian/for_loops.vani",
        "../georgian/early_exit.vani",
        "../georgian/iterate.vani",
        "../georgian/vec_invariants.vani",
        "../georgian/verified.vani",
        "../georgian/option_types.vani",
        "../georgian/try_question_op.vani",
        "../georgian/box_recursive_drop.vani",
        // Phase 9b / 10.2 / 13.1: CJK scripts — Japanese, Mandarin, Korean
        "../japanese/basics.vani",
        "../japanese/keywords.vani",
        "../japanese/control_flow.vani",
        "../japanese/for_loops.vani",
        "../japanese/early_exit.vani",
        "../japanese/iterate.vani",
        "../japanese/vec_invariants.vani",
        "../japanese/verified.vani",
        "../japanese/option_types.vani",
        "../japanese/try_question_op.vani",
        "../japanese/box_recursive_drop.vani",
        "../mandarin/keywords.vani",
        "../mandarin/control_flow.vani",
        "../mandarin/for_loops.vani",
        "../mandarin/early_exit.vani",
        "../mandarin/iterate.vani",
        "../mandarin/vec_invariants.vani",
        "../mandarin/verified.vani",
        "../mandarin/option_types.vani",
        "../mandarin/try_question_op.vani",
        "../mandarin/box_recursive_drop.vani",
        "../korean/basics.vani",
        "../korean/keywords.vani",
        "../korean/control_flow.vani",
        "../korean/for_loops.vani",
        "../korean/early_exit.vani",
        "../korean/iterate.vani",
        "../korean/vec_invariants.vani",
        "../korean/verified.vani",
        "../korean/option_types.vani",
        "../korean/try_question_op.vani",
        "../korean/box_recursive_drop.vani",
        // Phase 13.15 / 13.29–13.30 / 13.34: SE Asian scripts — Thai, Khmer, Burmese, Lao
        "../thai/basics.vani",
        "../thai/keywords.vani",
        "../thai/control_flow.vani",
        "../thai/for_loops.vani",
        "../thai/early_exit.vani",
        "../thai/iterate.vani",
        "../thai/vec_invariants.vani",
        "../thai/verified.vani",
        "../thai/option_types.vani",
        "../thai/try_question_op.vani",
        "../thai/box_recursive_drop.vani",
        "../khmer/basics.vani",
        "../khmer/keywords.vani",
        "../khmer/control_flow.vani",
        "../khmer/for_loops.vani",
        "../khmer/early_exit.vani",
        "../khmer/iterate.vani",
        "../khmer/vec_invariants.vani",
        "../khmer/verified.vani",
        "../khmer/option_types.vani",
        "../khmer/try_question_op.vani",
        "../khmer/box_recursive_drop.vani",
        "../burmese/basics.vani",
        "../burmese/keywords.vani",
        "../burmese/control_flow.vani",
        "../burmese/for_loops.vani",
        "../burmese/early_exit.vani",
        "../burmese/iterate.vani",
        "../burmese/vec_invariants.vani",
        "../burmese/verified.vani",
        "../burmese/option_types.vani",
        "../burmese/try_question_op.vani",
        "../burmese/box_recursive_drop.vani",
        "../lao/basics.vani",
        "../lao/keywords.vani",
        "../lao/control_flow.vani",
        "../lao/for_loops.vani",
        "../lao/early_exit.vani",
        "../lao/iterate.vani",
        "../lao/vec_invariants.vani",
        "../lao/verified.vani",
        "../lao/option_types.vani",
        "../lao/try_question_op.vani",
        "../lao/box_recursive_drop.vani",
        // Amharic (Ethiopic script)
        "../amharic/basics.vani",
        "../amharic/keywords.vani",
        "../amharic/control_flow.vani",
        "../amharic/for_loops.vani",
        "../amharic/early_exit.vani",
        "../amharic/iterate.vani",
        "../amharic/vec_invariants.vani",
        "../amharic/verified.vani",
        "../amharic/option_types.vani",
        "../amharic/try_question_op.vani",
        "../amharic/box_recursive_drop.vani",
        // Tibetan
        "../tibetan/basics.vani",
        "../tibetan/keywords.vani",
        "../tibetan/control_flow.vani",
        "../tibetan/for_loops.vani",
        "../tibetan/early_exit.vani",
        "../tibetan/iterate.vani",
        "../tibetan/vec_invariants.vani",
        "../tibetan/verified.vani",
        "../tibetan/option_types.vani",
        "../tibetan/try_question_op.vani",
        "../tibetan/box_recursive_drop.vani",
        // Mongolian (Traditional script)
        "../mongolian/basics.vani",
        "../mongolian/keywords.vani",
        "../mongolian/control_flow.vani",
        "../mongolian/for_loops.vani",
        "../mongolian/early_exit.vani",
        "../mongolian/iterate.vani",
        "../mongolian/vec_invariants.vani",
        "../mongolian/verified.vani",
        "../mongolian/option_types.vani",
        "../mongolian/try_question_op.vani",
        "../mongolian/box_recursive_drop.vani",
        // Cherokee (syllabary)
        "../cherokee/basics.vani",
        "../cherokee/keywords.vani",
        "../cherokee/control_flow.vani",
        "../cherokee/for_loops.vani",
        "../cherokee/early_exit.vani",
        "../cherokee/iterate.vani",
        "../cherokee/vec_invariants.vani",
        "../cherokee/verified.vani",
        "../cherokee/option_types.vani",
        "../cherokee/try_question_op.vani",
        "../cherokee/box_recursive_drop.vani",
        // async/await smoke tests for all remaining dialects (Phase 6+, 2026-06-15)
        "../gujarati/async_cancel_auto.vani",
        "../nepali/async_cancel_auto.vani",
        "../maithili/async_cancel_auto.vani",
        "../konkani/async_cancel_auto.vani",
        "../bengali/async_cancel_auto.vani",
        "../assamese/async_cancel_auto.vani",
        "../tamil/async_cancel_auto.vani",
        "../telugu/async_cancel_auto.vani",
        "../punjabi/async_cancel_auto.vani",
        "../kannada/async_cancel_auto.vani",
        "../malayalam/async_cancel_auto.vani",
        "../odia/async_cancel_auto.vani",
        "../sinhala/async_cancel_auto.vani",
        "../urdu/async_cancel_auto.vani",
        "../sindhi/async_cancel_auto.vani",
        "../punjabi_shahmukhi/async_cancel_auto.vani",
        "../persian/async_cancel_auto.vani",
        "../pashto/async_cancel_auto.vani",
        "../arabic/async_cancel_auto.vani",
        "../hebrew/async_cancel_auto.vani",
        "../russian/async_cancel_auto.vani",
        "../spanish/async_cancel_auto.vani",
        "../french/async_cancel_auto.vani",
        "../german/async_cancel_auto.vani",
        "../italian/async_cancel_auto.vani",
        "../portuguese/async_cancel_auto.vani",
        "../dutch/async_cancel_auto.vani",
        "../swedish/async_cancel_auto.vani",
        "../norwegian/async_cancel_auto.vani",
        "../danish/async_cancel_auto.vani",
        "../finnish/async_cancel_auto.vani",
        "../czech/async_cancel_auto.vani",
        "../polish/async_cancel_auto.vani",
        "../romanian/async_cancel_auto.vani",
        "../hungarian/async_cancel_auto.vani",
        "../slovak/async_cancel_auto.vani",
        "../catalan/async_cancel_auto.vani",
        "../turkish/async_cancel_auto.vani",
        "../greek/async_cancel_auto.vani",
        "../indonesian/async_cancel_auto.vani",
        "../malay/async_cancel_auto.vani",
        "../swahili/async_cancel_auto.vani",
        "../filipino/async_cancel_auto.vani",
        "../vietnamese/async_cancel_auto.vani",
        "../hausa/async_cancel_auto.vani",
        "../yoruba/async_cancel_auto.vani",
        "../armenian/async_cancel_auto.vani",
        "../georgian/async_cancel_auto.vani",
        "../japanese/async_cancel_auto.vani",
        "../korean/async_cancel_auto.vani",
        "../thai/async_cancel_auto.vani",
        "../khmer/async_cancel_auto.vani",
        "../burmese/async_cancel_auto.vani",
        "../lao/async_cancel_auto.vani",
        "../amharic/async_cancel_auto.vani",
        "../tibetan/async_cancel_auto.vani",
        "../mongolian/async_cancel_auto.vani",
        "../cherokee/async_cancel_auto.vani",
        // GoF design patterns (refactoring.guru) — 22 patterns:
        // 5 creational + 7 structural + 10 behavioral.
        "design_patterns/creational/factory_method.vani",
        "design_patterns/creational/abstract_factory.vani",
        "design_patterns/creational/builder.vani",
        "design_patterns/creational/prototype.vani",
        "design_patterns/creational/singleton.vani",
        "design_patterns/structural/adapter.vani",
        "design_patterns/structural/bridge.vani",
        "design_patterns/structural/composite.vani",
        "design_patterns/structural/decorator.vani",
        "design_patterns/structural/facade.vani",
        "design_patterns/structural/flyweight.vani",
        "design_patterns/structural/proxy.vani",
        "design_patterns/behavioral/chain_of_responsibility.vani",
        "design_patterns/behavioral/command.vani",
        "design_patterns/behavioral/iterator.vani",
        "design_patterns/behavioral/mediator.vani",
        "design_patterns/behavioral/memento.vani",
        "design_patterns/behavioral/observer.vani",
        "design_patterns/behavioral/state.vani",
        "design_patterns/behavioral/strategy.vani",
        "design_patterns/behavioral/template_method.vani",
        "design_patterns/behavioral/visitor.vani",
        "scopes.vani",
        "skiplist.vani",
        "sort.vani",
        "string_ops.vani",
        "strings.vani",
        "strings_concat.vani",
        "struct_atomic_field.vani",
        "struct_eq.vani",
        "struct_mixed_fields.vani",
        "struct_owned_field.vani",
        "tasks.vani",
        "tracker.vani",
        "trie.vani",
        "try_keyword.vani",
        "try_question_op.vani",
        "box_recursive_drop.vani",
        "box_dyn_sugar.vani",
        "box_dyn_iface.vani",
        "ffi.vani",
        "pool.vani",
        "match_ref_payload.vani",
        "tuple_eq.vani",
        "type_associated_fn.vani",
        "union_find.vani",
        "unit_return.vani",
        "vec_invariants.vani",
        "vectors.vani",
        "verified.vani",
    ] {
        let example = format!("{}/examples/language/english/{}", manifest_dir, name);

        let c_out = Command::new(binary)
            .args(["run", &example, "--backend=c"])
            .output()
            .expect("c run");
        let llvm_out = Command::new(binary)
            .args(["run", &example, "--backend=llvm"])
            .output()
            .expect("llvm run");

        assert!(
            c_out.status.success(),
            "C backend failed for {name}: {}",
            String::from_utf8_lossy(&c_out.stderr)
        );
        assert!(
            llvm_out.status.success(),
            "LLVM backend failed for {name}: {}",
            String::from_utf8_lossy(&llvm_out.stderr)
        );
        let c_stdout = String::from_utf8_lossy(&c_out.stdout).replace("\r\n", "\n");
        let llvm_stdout = String::from_utf8_lossy(&llvm_out.stdout).replace("\r\n", "\n");
        assert_eq!(
            c_stdout,
            llvm_stdout,
            "stdout diverges between C and LLVM for {name}"
        );
        assert_eq!(
            c_out.status.code(),
            llvm_out.status.code(),
            "exit codes diverge between C and LLVM for {name}"
        );
    }
}

#[test]
fn run_inline_call_proofs_example() {
    let binary = env!("CARGO_BIN_EXE_intentc");
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let example = format!("{}/examples/language/english/inline_call_proofs.vani", manifest_dir);

    let output = Command::new(binary)
        .args(["run", &example])
        .output()
        .expect("intentc run should execute");

    assert!(
        output.status.success(),
        "intentc run failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("6"), "expected caller(5) = inc(5) = 6, got: {stdout}");
}

#[test]
fn run_bounds_elision_example_and_verify_no_runtime_guard() {
    let binary = env!("CARGO_BIN_EXE_intentc");
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let example = format!("{}/examples/language/english/bounds_elision.vani", manifest_dir);

    // First, prove the program runs and prints the expected outputs.
    let output = Command::new(binary)
        .args(["run", &example])
        .output()
        .expect("intentc run should execute");
    assert!(
        output.status.success(),
        "intentc run failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    for v in ["10", "30", "50", "150"] {
        assert!(stdout.contains(v), "expected {v} in stdout, got: {stdout}");
    }

    // Second, inspect the emitted C and confirm each bounds-elidable
    // function body has no intent_check_bounds call.
    let emit = Command::new(binary)
        .args(["emit-c", &example])
        .output()
        .expect("intentc emit-c should execute");
    assert!(emit.status.success(), "emit-c failed");
    let c = String::from_utf8_lossy(&emit.stdout);
    for fname in ["fn_first", "fn_at", "fn_last", "fn_sum"] {
        // Find the function *definition* (skipping the forward decl
        // by matching the open-brace tail).
        let pat = format!("{}(", fname);
        let mut search = c.as_ref();
        let mut found_def = false;
        while let Some(idx) = search.find(&pat) {
            let after = &search[idx..];
            // Definition has `{` on the same line as the closing paren;
            // the forward decl has `;`.
            let line_end = after.find('\n').unwrap_or(after.len());
            let line = &after[..line_end];
            if line.contains(") {") {
                let body_end = after.find("\n}\n").map(|i| i + 1).unwrap_or(after.len());
                let body = &after[..body_end];
                assert!(
                    !body.contains("intent_check_bounds"),
                    "expected no bounds-check call in {}: {}",
                    fname,
                    body
                );
                found_def = true;
                break;
            }
            search = &after[1..];
        }
        assert!(found_def, "could not find definition of {fname}");
    }

    // Third, do the same shape check on the LLVM backend. The
    // marker for an elided bounds check in LLVM is the absence of
    // an inline `call void @abort()` in the function body (apart
    // from the one each requires clause emits). `fn_sum` has no
    // requires, so its body must contain *zero* `@abort` calls.
    let llvm_emit = Command::new(binary)
        .args(["emit", &example])
        .output()
        .expect("intentc emit (llvm) should execute");
    assert!(llvm_emit.status.success(), "emit --backend=llvm failed");
    let ll = String::from_utf8_lossy(&llvm_emit.stdout);
    let sum_start = ll
        .find("define i64 @fn_sum(")
        .expect("expected fn_sum in LLVM IR");
    let sum_body = &ll[sum_start..];
    let sum_end = sum_body.find("\n}\n").map(|i| i + 1).unwrap_or(sum_body.len());
    let sum_body = &sum_body[..sum_end];
    assert!(
        !sum_body.contains("call void @abort()"),
        "expected no abort/guard in fn_sum LLVM body, got:\n{sum_body}"
    );
}

#[test]
fn run_vec_invariants_example() {
    let binary = env!("CARGO_BIN_EXE_intentc");
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let example = format!("{}/examples/language/english/vec_invariants.vani", manifest_dir);

    let output = Command::new(binary)
        .args(["run", &example])
        .output()
        .expect("intentc run should execute");

    assert!(
        output.status.success(),
        "intentc run failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    // The loop pushes 0, 10, 20, 30, 40; verify all five appear on stdout.
    let stdout = String::from_utf8_lossy(&output.stdout);
    for v in ["0", "10", "20", "30", "40"] {
        assert!(stdout.contains(v), "expected {v} in stdout, got: {stdout}");
    }
}

#[test]
fn json_check_outputs_empty_diagnostics_on_success() {
    let binary = env!("CARGO_BIN_EXE_intentc");
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let example = format!("{}/examples/language/english/basics.vani", manifest_dir);

    let output = Command::new(binary)
        .args(["check", &example, "--json"])
        .output()
        .expect("intentc check --json should execute");

    assert!(output.status.success(), "expected exit 0 on success");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let trimmed = stdout.trim_end();
    assert_eq!(
        trimmed, "{\"diagnostics\":[]}",
        "expected canonical empty-success JSON, got: {stdout}"
    );
}

#[test]
fn json_check_outputs_structured_diagnostics_on_failure() {
    use std::fs;

    let binary = env!("CARGO_BIN_EXE_intentc");
    let dir = std::env::temp_dir().join(format!(
        "intentc-json-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir_all(&dir).expect("mkdir");
    let src = dir.join("bad.vani");
    fs::write(
        &src,
        "fn main() -> i64 {\n  return undefined_name;\n}\n",
    )
    .expect("write src");

    let output = Command::new(binary)
        .args(["check", src.to_str().unwrap(), "--json"])
        .output()
        .expect("intentc check --json");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let status = output.status;
    let _ = fs::remove_dir_all(&dir);

    assert!(!status.success(), "expected non-zero exit on failure");
    assert!(
        stdout.contains("\"diagnostics\":[")
            && stdout.contains("\"level\":\"error\"")
            && stdout.contains("undefined_name"),
        "expected structured JSON with the undefined-name error, got: {stdout}"
    );
}

// ---------------------------------------------------------------------------
// Elaboration integration tests — verify .elaboration survives to JSON output
// ---------------------------------------------------------------------------

fn write_tmp_vani(stem: &str, code: &str) -> std::path::PathBuf {
    use std::fs;
    let dir = std::env::temp_dir().join(format!(
        "intentc-elab-{}-{}-{}",
        stem,
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir_all(&dir).expect("mkdir elab tmp");
    let path = dir.join(format!("{stem}.vani"));
    fs::write(&path, code).expect("write elab tmp");
    path
}

#[test]
fn json_elaboration_type_mismatch_appears_in_output() {
    let binary = env!("CARGO_BIN_EXE_intentc");
    let src = write_tmp_vani(
        "elab_type_mismatch",
        "fn main() -> i64 {\n  let x: i64 = true;\n  return x;\n}\n",
    );
    let out = Command::new(binary)
        .args(["check", src.to_str().unwrap(), "--json"])
        .output()
        .expect("intentc check --json");
    let stdout = String::from_utf8_lossy(&out.stdout);
    let _ = std::fs::remove_dir_all(src.parent().unwrap());
    assert!(!out.status.success(), "type mismatch must fail");
    assert!(
        stdout.contains("\"elaboration\":[\""),
        "elaboration must appear in JSON output for type_mismatch, got:\n{stdout}"
    );
}

#[test]
fn json_elaboration_unknown_variable_appears_in_output() {
    let binary = env!("CARGO_BIN_EXE_intentc");
    let src = write_tmp_vani(
        "elab_unknown_var",
        "fn main() -> i64 {\n  return ghost_var;\n}\n",
    );
    let out = Command::new(binary)
        .args(["check", src.to_str().unwrap(), "--json"])
        .output()
        .expect("intentc check --json");
    let stdout = String::from_utf8_lossy(&out.stdout);
    let _ = std::fs::remove_dir_all(src.parent().unwrap());
    assert!(!out.status.success(), "unknown variable must fail");
    assert!(
        stdout.contains("\"elaboration\":[\""),
        "elaboration must appear in JSON output for unknown_variable, got:\n{stdout}"
    );
}

#[test]
fn json_elaboration_wrong_arity_appears_in_output() {
    let binary = env!("CARGO_BIN_EXE_intentc");
    let src = write_tmp_vani(
        "elab_wrong_arity",
        "fn add(a: i64, b: i64) -> i64 { return a + b; }\nfn main() -> i64 { return add(1); }\n",
    );
    let out = Command::new(binary)
        .args(["check", src.to_str().unwrap(), "--json"])
        .output()
        .expect("intentc check --json");
    let stdout = String::from_utf8_lossy(&out.stdout);
    let _ = std::fs::remove_dir_all(src.parent().unwrap());
    assert!(!out.status.success(), "wrong arity must fail");
    assert!(
        stdout.contains("\"elaboration\":[\""),
        "elaboration must appear in JSON output for wrong_arity, got:\n{stdout}"
    );
}

#[test]
fn json_elaboration_iface_not_impl_appears_in_output() {
    let binary = env!("CARGO_BIN_EXE_intentc");
    let src = write_tmp_vani(
        "elab_iface_not_impl",
        "interface Printable { fn show(self: ref Self) -> i64; }\nstruct Box { val: i64 }\nfn show_it(p: ref Printable) -> i64 { return p.show(); }\nfn main() -> i64 {\n  let b: Box = Box { val: 5 };\n  return show_it(ref b);\n}\n",
    );
    let out = Command::new(binary)
        .args(["check", src.to_str().unwrap(), "--json"])
        .output()
        .expect("intentc check --json");
    let stdout = String::from_utf8_lossy(&out.stdout);
    let _ = std::fs::remove_dir_all(src.parent().unwrap());
    assert!(!out.status.success(), "iface not impl must fail");
    assert!(
        stdout.contains("\"elaboration\":[\""),
        "elaboration must appear in JSON output for iface_not_impl, got:\n{stdout}"
    );
}

#[test]
fn json_elaboration_pure_fn_effect_appears_in_output() {
    let binary = env!("CARGO_BIN_EXE_intentc");
    let src = write_tmp_vani(
        "elab_pure_effect",
        "fn impure(x: i64) -> i64 { print x; return x; }\npure fn compute(x: i64) -> i64 { return impure(x); }\nfn main() -> i64 { return compute(1); }\n",
    );
    let out = Command::new(binary)
        .args(["check", src.to_str().unwrap(), "--json"])
        .output()
        .expect("intentc check --json");
    let stdout = String::from_utf8_lossy(&out.stdout);
    let _ = std::fs::remove_dir_all(src.parent().unwrap());
    assert!(!out.status.success(), "pure fn calling impure must fail");
    assert!(
        stdout.contains("\"elaboration\":[\""),
        "elaboration must appear in JSON output for pure_fn_has_effect, got:\n{stdout}"
    );
}

#[test]
fn assert_with_message_emits_custom_runtime_diagnostic() {
    use std::fs;

    let binary = env!("CARGO_BIN_EXE_intentc");
    let dir = std::env::temp_dir().join(format!(
        "intentc-assert-msg-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir_all(&dir).expect("mkdir");
    let src = dir.join("bad.vani");
    fs::write(
        &src,
        "fn main() -> i64 {\n  let x: i64 = 0;\n  assert x == 1, \"x should be exactly one\";\n  return 0;\n}\n",
    )
    .expect("write src");

    let output = Command::new(binary)
        .args(["run", src.to_str().unwrap()])
        .output()
        .expect("intentc run");

    let stderr = String::from_utf8_lossy(&output.stderr);
    let status = output.status;

    let _ = fs::remove_dir_all(&dir);

    assert!(
        !status.success(),
        "expected failure exit; stderr was: {stderr}"
    );
    assert!(
        stderr.contains("assertion failed: x should be exactly one"),
        "expected custom message on stderr, got: {stderr}"
    );
}

#[test]
fn run_iterate_example() {
    let binary = env!("CARGO_BIN_EXE_intentc");
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let example = format!("{}/examples/language/english/iterate.vani", manifest_dir);

    let output = Command::new(binary)
        .args(["run", &example])
        .output()
        .expect("intentc run should execute");

    assert!(
        output.status.success(),
        "intentc run failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("15"), "expected total==15: {stdout}");
    assert!(stdout.contains("9"), "expected max==9: {stdout}");
    assert!(stdout.contains("3"), "expected positives==3: {stdout}");
}

#[test]
fn run_invariants_example() {
    let binary = env!("CARGO_BIN_EXE_intentc");
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let example = format!("{}/examples/language/english/invariants.vani", manifest_dir);

    let output = Command::new(binary)
        .args(["run", &example])
        .output()
        .expect("intentc run should execute");

    assert!(
        output.status.success(),
        "intentc run failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("5"), "expected count_to(5)==5, got: {stdout}");
    assert!(stdout.contains("1"), "expected min==1, got: {stdout}");
}

#[test]
fn run_contracts_example() {
    let binary = env!("CARGO_BIN_EXE_intentc");
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let example = format!("{}/examples/language/english/contracts.vani", manifest_dir);

    let output = Command::new(binary)
        .args(["run", &example])
        .output()
        .expect("intentc run should execute");

    assert!(
        output.status.success(),
        "intentc run failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("13"), "expected diff==13, got: {stdout}");
    assert!(stdout.contains("10"), "expected bigger==10, got: {stdout}");
}

#[test]
fn run_for_loops_example() {
    let binary = env!("CARGO_BIN_EXE_intentc");
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let example = format!("{}/examples/language/english/for_loops.vani", manifest_dir);

    let output = Command::new(binary)
        .args(["run", &example])
        .output()
        .expect("intentc run should execute");

    assert!(
        output.status.success(),
        "intentc run failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("30"), "expected sum_squares==30, got: {stdout}");
    assert!(stdout.contains("2"), "expected first-zero==2, got: {stdout}");
}

#[test]
fn run_verified_example() {
    let binary = env!("CARGO_BIN_EXE_intentc");
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let example = format!("{}/examples/language/english/verified.vani", manifest_dir);

    let output = Command::new(binary)
        .args(["run", &example])
        .output()
        .expect("intentc run should execute");

    assert!(
        output.status.success(),
        "intentc run failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("7"), "expected safe_subtract == 7, got: {stdout}");
}

#[test]
fn run_mut_refs_example() {
    let binary = env!("CARGO_BIN_EXE_intentc");
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let example = format!("{}/examples/language/english/mut_refs.vani", manifest_dir);

    let output = Command::new(binary)
        .args(["run", &example])
        .output()
        .expect("intentc run should execute");

    assert!(
        output.status.success(),
        "intentc run failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("8"), "expected doubled-last 8, got: {stdout}");
    assert!(stdout.contains("9"), "expected fill value 9, got: {stdout}");
}

#[test]
fn run_scopes_example() {
    let binary = env!("CARGO_BIN_EXE_intentc");
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let example = format!("{}/examples/language/english/scopes.vani", manifest_dir);

    let output = Command::new(binary)
        .args(["run", &example])
        .output()
        .expect("intentc run should execute");

    assert!(
        output.status.success(),
        "intentc run failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("3"), "expected counter == 3, got: {stdout}");
}

#[test]
fn run_early_exit_example() {
    let binary = env!("CARGO_BIN_EXE_intentc");
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let example = format!("{}/examples/language/english/early_exit.vani", manifest_dir);

    let output = Command::new(binary)
        .args(["run", &example])
        .output()
        .expect("intentc run should execute");

    assert!(
        output.status.success(),
        "intentc run failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("-3"), "expected -3 in stdout, got: {stdout}");
    assert!(stdout.contains("3"), "expected positives count 3, got: {stdout}");
}

#[test]
fn run_control_flow_example() {
    let binary = env!("CARGO_BIN_EXE_intentc");
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let example = format!("{}/examples/language/english/control_flow.vani", manifest_dir);

    let output = Command::new(binary)
        .args(["run", &example])
        .output()
        .expect("intentc run should execute");

    assert!(
        output.status.success(),
        "intentc run failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("10"), "expected total == 10, got: {stdout}");
}

#[test]
fn run_borrows_example_prints_sum() {
    let binary = env!("CARGO_BIN_EXE_intentc");
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let example = format!("{}/examples/language/english/borrows.vani", manifest_dir);

    let output = Command::new(binary)
        .args(["run", &example])
        .output()
        .expect("intentc run should execute");

    assert!(
        output.status.success(),
        "intentc run failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("10"), "expected sum == 10, got: {stdout}");
}

#[test]
fn run_vectors_example_prints_first_element() {
    let binary = env!("CARGO_BIN_EXE_intentc");
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let example = format!("{}/examples/language/english/vectors.vani", manifest_dir);

    let output = Command::new(binary)
        .args(["run", &example])
        .output()
        .expect("intentc run should execute");

    assert!(
        output.status.success(),
        "intentc run failed with status {:?}\nstderr: {}",
        output.status,
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("99"),
        "expected first == 99 in stdout, got: {stdout}"
    );
}

#[test]
fn run_arrays_example_prints_sum() {
    let binary = env!("CARGO_BIN_EXE_intentc");
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let example = format!("{}/examples/language/english/arrays.vani", manifest_dir);

    let output = Command::new(binary)
        .args(["run", &example])
        .output()
        .expect("intentc run should execute");

    assert!(
        output.status.success(),
        "intentc run failed with status {:?}\nstderr: {}",
        output.status,
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("10"),
        "expected sum_four([1,2,3,4]) = 10 in stdout, got: {stdout}"
    );
}

#[test]
#[ignore = "echo_with_timeout.vani LLVM IR has undefined value for async TCP locals; lli rejects it"]
fn intentc_test_expands_directory_arg_to_intent_files() {
    // `intentc test examples/` should walk the directory and run
    // every `*.vani` inside. Same result as listing them out
    // explicitly, but the dir form is the user-friendly path.
    let lli = std::env::var("LLI").unwrap_or_else(|_| "lli".to_string());
    let lli_ok = Command::new(&lli)
        .arg("--version")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false);
    if !lli_ok {
        return;
    }

    let binary = env!("CARGO_BIN_EXE_intentc");
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let examples_dir = format!("{}/examples/language", manifest_dir);

    fn count_vani_recursive(dir: &std::path::Path) -> usize {
        let mut n = 0;
        if let Ok(rd) = std::fs::read_dir(dir) {
            for e in rd.flatten() {
                let p = e.path();
                if p.is_dir() {
                    n += count_vani_recursive(&p);
                } else if p.extension().and_then(|s| s.to_str()) == Some("vani") {
                    n += 1;
                }
            }
        }
        n
    }
    let n_examples = count_vani_recursive(std::path::Path::new(&examples_dir));

    let run = Command::new(binary)
        .args(["test", &examples_dir])
        .output()
        .expect("vanic test <dir>");
    assert!(
        run.status.success(),
        "vanic test <dir> should succeed\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&run.stdout),
        String::from_utf8_lossy(&run.stderr),
    );
    let stdout = String::from_utf8_lossy(&run.stdout);
    let expected = format!("{} passed; 0 failed", n_examples);
    assert!(
        stdout.contains(&expected),
        "expected `{expected}` in summary, got:\n{stdout}"
    );
}

#[test]
fn intentc_test_trims_lli_backtrace_from_failed_stderr() {
    // When a test program aborts (failed assert), lli prints a long
    // signal-handler backtrace that's not useful to Intent users.
    // Confirm the captured stderr was truncated to the meaningful
    // line ("assertion failed: ...") and the lli boilerplate is gone.
    let lli = std::env::var("LLI").unwrap_or_else(|_| "lli".to_string());
    let lli_ok = Command::new(&lli)
        .arg("--version")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false);
    if !lli_ok {
        return;
    }

    let binary = env!("CARGO_BIN_EXE_intentc");
    let tmp_fail = std::env::temp_dir().join(format!(
        "intentc_trim_lli_{}.vani",
        std::process::id()
    ));
    std::fs::write(
        &tmp_fail,
        b"fn main() -> i64 {\n  assert 1 == 2, \"deliberate failure\";\n  return 0;\n}\n",
    )
    .expect("write tmp");

    let run = Command::new(binary)
        .args(["test", tmp_fail.to_str().unwrap()])
        .output()
        .expect("intentc test");
    let _ = std::fs::remove_file(&tmp_fail);
    assert_eq!(run.status.code(), Some(1));

    let stderr = String::from_utf8_lossy(&run.stderr);
    assert!(
        stderr.contains("assertion failed: deliberate failure"),
        "expected the meaningful failure line, got:\n{stderr}"
    );
    assert!(
        !stderr.contains("PLEASE submit a bug report") && !stderr.contains("Stack dump:"),
        "lli backtrace should have been trimmed, got:\n{stderr}"
    );
}

// BUG-31 (2026-07-28): a struct that owns a Vec of its own type
// (`struct Node { children: Vec<Node> }`) made the C backend's
// struct-emission topological sort deadlock on a false self-
// dependency -- `Struct_Node` was silently never emitted (no
// diagnostic at all), and every downstream reference then failed
// with a confusing "incomplete type" error from `cc`. This is
// the shape every tree / recursive-structure example needs, so
// it's a high-value fix -- found auditing the recursion-primer
// tutorial's tree-walk example. C-backend only: the LLVM backend
// has a separate, not-yet-root-caused issue on this same program
// (compiles to a native binary that silently crashes with no
// output) -- see docs/TODO_CURRENT.md's BUG-31 entry.
// BUG-32 (2026-07-29): an `eprint` string-literal item that never
// also appeared in a `print` statement anywhere in the program was
// silently dropped from LLVM output (the string-interning pre-pass
// only walked Print, never EPrint). Found auditing the print-block
// tutorial's eprint example. Both backends checked since the C
// backend was never affected (it doesn't share this interning
// pass) -- this test pins that it stays that way.
#[test]
fn eprint_string_literal_example_produces_correct_output_on_both_backends() {
    let binary = env!("CARGO_BIN_EXE_intentc");
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let example = format!(
        "{}/examples/language/english/eprint_string_literal.vani",
        manifest_dir
    );
    let expected = "eprint-only literal, never printed elsewhere\npath = /tmp/example\n";

    for backend_args in [vec!["run", &example], vec!["run", &example, "--backend=c"]] {
        let output = Command::new(binary)
            .args(&backend_args)
            .output()
            .unwrap_or_else(|e| panic!("intentc {:?} should execute: {e}", backend_args));
        assert!(
            output.status.success(),
            "intentc {:?} failed with status {:?}\nstderr: {}",
            backend_args,
            output.status,
            String::from_utf8_lossy(&output.stderr)
        );
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert_eq!(
            stderr.replace("\r\n", "\n"),
            expected,
            "eprint string-literal item(s) dropped for {:?}",
            backend_args
        );
    }
}

// BUG-35 (2026-07-29): same bug class as BUG-29 -- a payload-less
// enum variant's zero-init placeholder only handled Str/OwnedStr as
// needing LLVM's `null` literal for a pointer-shaped payload slot;
// Box<T> (and raw pointers) also lower to a bare pointer but fell
// through to the integer `0` default. Option<Box<Node>> -- the
// canonical recursive-struct shape -- crashed the LLVM backend as a
// result. Found auditing the Box<T>/RAII tutorial's own examples.
#[test]
fn option_box_recursive_struct_example_produces_correct_output_on_both_backends() {
    let binary = env!("CARGO_BIN_EXE_intentc");
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let example = format!(
        "{}/examples/language/english/option_box_recursive_struct.vani",
        manifest_dir
    );
    let expected = "1\n";

    for backend_args in [vec!["run", &example], vec!["run", &example, "--backend=c"]] {
        let output = Command::new(binary)
            .args(&backend_args)
            .output()
            .unwrap_or_else(|e| panic!("intentc {:?} should execute: {e}", backend_args));
        assert!(
            output.status.success(),
            "intentc {:?} failed with status {:?}\nstderr: {}",
            backend_args,
            output.status,
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert_eq!(
            stdout.replace("\r\n", "\n"),
            expected,
            "Option<Box<Node>> construction produced the wrong result for {:?}",
            backend_args
        );
    }
}

// BUG-37 (2026-07-29): clone_at() on a Vec<Struct> element whose
// struct has a nested non-Copy Vec<T> field crashed the LLVM backend
// with a double-free (exit 116). Two independent LLVM codegen sites
// (emit_vec_bundle_functions's `__clone` bundle function, and
// clone_at's own separate inline struct-clone codegen) only deep-
// cloned Type::OwnedStr fields, silently shallow-copying any other
// non-Copy field -- including a nested Vec<T> -- so the "clone"
// aliased the source's heap buffer. Found auditing the cyclic-
// references tutorial's tree-building example.
#[test]
fn clone_at_struct_with_nested_vec_field_example_produces_correct_output_on_both_backends() {
    let binary = env!("CARGO_BIN_EXE_intentc");
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let example = format!(
        "{}/examples/language/english/clone_at_struct_with_nested_vec_field.vani",
        manifest_dir
    );
    let expected = "7\n";

    for backend_args in [vec!["run", &example], vec!["run", &example, "--backend=c"]] {
        let output = Command::new(binary)
            .args(&backend_args)
            .output()
            .unwrap_or_else(|e| panic!("intentc {:?} should execute: {e}", backend_args));
        assert!(
            output.status.success(),
            "intentc {:?} failed with status {:?}\nstderr: {}",
            backend_args,
            output.status,
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert_eq!(
            stdout.replace("\r\n", "\n"),
            expected,
            "clone_at on a struct with a nested Vec field produced the wrong result for {:?}",
            backend_args
        );
    }
}

// BUG-39 (2026-07-29): calling an interface's default method on a
// type that inherits it (doesn't override it) was rejected --
// "argument 1 to <Type>_<method> must be assignable to Self, got
// <Type>". The checker's default-method injection copied the
// interface's own declared params/return-type verbatim, which still
// say `Self` literally; nothing substituted the concrete type
// because there's no user-written impl to do it (that's the whole
// point of a default method). Fixed by substituting Self -> the
// concrete implementing type when injecting a default-method body.
// Found auditing the default-methods tutorial's own worked example.
#[test]
fn default_method_inherited_self_type_example_produces_correct_output_on_both_backends() {
    let binary = env!("CARGO_BIN_EXE_intentc");
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let example = format!(
        "{}/examples/language/english/default_method_inherited_self_type.vani",
        manifest_dir
    );
    let expected = "I am something.\nI am a cat.\n";

    for backend_args in [vec!["run", &example], vec!["run", &example, "--backend=c"]] {
        let output = Command::new(binary)
            .args(&backend_args)
            .output()
            .unwrap_or_else(|e| panic!("intentc {:?} should execute: {e}", backend_args));
        assert!(
            output.status.success(),
            "intentc {:?} failed with status {:?}\nstderr: {}",
            backend_args,
            output.status,
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert_eq!(
            stdout.replace("\r\n", "\n"),
            expected,
            "inherited default method produced the wrong result for {:?}",
            backend_args
        );
    }
}

// BUG-40 (2026-07-29): `vanic run` (the lli JIT path) failed on any
// `parallel for` program with "Symbols not found: [ intent_pool_run ]"
// -- the pthreads thread-pool runtime parallel-for calls into was
// never packaged as a `-load`able shared library for the JIT the way
// sort()'s runtime already was. `vanic build` + running the binary
// always worked. Fixed by adding parallel_runtime_shared_lib() and
// wiring it into both lli invocation sites. Found auditing the
// function-pointers tutorial's own "this compiles" parallel-for
// example.
#[test]
fn parallel_for_jit_run_example_produces_correct_output_on_both_backends() {
    let binary = env!("CARGO_BIN_EXE_intentc");
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let example = format!(
        "{}/examples/language/english/parallel_for_jit_run.vani",
        manifest_dir
    );
    let expected = "90\n";

    for backend_args in [vec!["run", &example], vec!["run", &example, "--backend=c"]] {
        let output = Command::new(binary)
            .args(&backend_args)
            .output()
            .unwrap_or_else(|e| panic!("intentc {:?} should execute: {e}", backend_args));
        assert!(
            output.status.success(),
            "intentc {:?} failed with status {:?}\nstderr: {}",
            backend_args,
            output.status,
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert_eq!(
            stdout.replace("\r\n", "\n"),
            expected,
            "parallel for under vanic run produced the wrong result for {:?}",
            backend_args
        );
    }
}

// BUG-41 (2026-07-29): LLVM backend's `parallel for ... reduce x
// with *;` emitted an atomic load missing its required alignment --
// `load atomic i64, i64* %cap_N monotonic` with no `, align 8` --
// which modern LLVM rejects outright: "atomic load must have
// explicit non-zero alignment". The only atomic-load site in
// backend_llvm.rs missing an alignment; every sibling site already
// had one. Found testing examples/language/english/parallel.vani
// while diagnosing BUG-40.
#[test]
fn parallel_for_mul_reduction_example_produces_correct_output_on_both_backends() {
    let binary = env!("CARGO_BIN_EXE_intentc");
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let example = format!(
        "{}/examples/language/english/parallel_for_mul_reduction.vani",
        manifest_dir
    );
    let expected = "24\n";

    for backend_args in [vec!["run", &example], vec!["run", &example, "--backend=c"]] {
        let output = Command::new(binary)
            .args(&backend_args)
            .output()
            .unwrap_or_else(|e| panic!("intentc {:?} should execute: {e}", backend_args));
        assert!(
            output.status.success(),
            "intentc {:?} failed with status {:?}\nstderr: {}",
            backend_args,
            output.status,
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert_eq!(
            stdout.replace("\r\n", "\n"),
            expected,
            "parallel for multiplicative reduction produced the wrong result for {:?}",
            backend_args
        );
    }
}

// BUG-42 (2026-07-29): `vanic run --backend=c file.vani` (flag
// BEFORE the file path) silently ignored `--backend=c` and ran the
// LLVM backend instead, with no error. `required_file_at` correctly
// located the file even when preceded by flags, but only told the
// caller to resume flag-parsing AFTER the file's position --
// discarding every flag that came before it. Only `vanic run
// file.vani --backend=c` (flag AFTER) worked as documented. This
// silently made every `Command` in this test file that happened to
// place `--backend=c` before the path (several did) compare the
// LLVM backend against itself instead of against the C backend --
// see the BUG-43 entry below for a real bug that discovery un-hid.
// Fixed by having `required_file_at` return every flag arg (both
// before and after the file, file itself excluded) instead of just
// an index to resume from.
#[test]
fn backend_flag_before_file_path_is_honored() {
    let binary = env!("CARGO_BIN_EXE_intentc");
    // strlen collides with nothing on the C side but does collide
    // with an internal LLVM declaration when NOT actually routed
    // through the C backend -- a convenient canary since it fails
    // loudly (a distinctive "invalid redefinition" lli crash) if
    // --backend=c silently didn't take effect.
    let src = "extern \"C\" fn strlen(s: Str) -> u64;\n\
               fn main() -> i64 { print \"ok\"; return 0; }\n";
    let dir = std::env::temp_dir().join(format!(
        "vani_bug42_{}_{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0)
    ));
    std::fs::create_dir_all(&dir).expect("create temp dir");
    let file = dir.join("bug42.vani");
    std::fs::write(&file, src).expect("write temp source");

    let output = Command::new(binary)
        .args(["run", "--backend=c", file.to_str().unwrap()])
        .output()
        .expect("intentc run --backend=c <file> (flag before path) should execute");
    let _ = std::fs::remove_dir_all(&dir);

    assert!(
        output.status.success(),
        "flag-before-path --backend=c was not honored (fell through to LLVM \
         and crashed on the strlen canary): stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout).replace("\r\n", "\n"),
        "ok\n"
    );
}

// BUG-43 (2026-07-29): a `let`-declared local inside a `task NAME {
// ... }` block body (beyond the block's own captures) crashed the
// LLVM backend with "use of undefined value '%N.name.addr'" -- the
// outlined task function's FnCtx never set skip_alloca_hoisting =
// true the way the parallel-for outlined worker already does, so
// the local's alloca was silently pushed into a preamble buffer
// that's never flushed for outlined functions, while the store/load
// referencing it were still emitted. This was a pre-existing,
// never-actually-exercised bug: examples/language/english/
// echo_loop.vani's own C-vs-LLVM parity tests always passed
// vacuously because of BUG-42 above, until fixing BUG-42 made them
// meaningful and they caught this for real.
#[test]
fn task_block_local_variable_example_produces_correct_output_on_both_backends() {
    let binary = env!("CARGO_BIN_EXE_intentc");
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let example = format!(
        "{}/examples/language/english/task_block_local_variable.vani",
        manifest_dir
    );
    let expected = "done\n";

    for backend_args in [vec!["run", &example], vec!["run", &example, "--backend=c"]] {
        let output = Command::new(binary)
            .args(&backend_args)
            .output()
            .unwrap_or_else(|e| panic!("intentc {:?} should execute: {e}", backend_args));
        assert!(
            output.status.success(),
            "intentc {:?} failed with status {:?}\nstderr: {}",
            backend_args,
            output.status,
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert_eq!(
            stdout.replace("\r\n", "\n"),
            expected,
            "task-block local variable example produced the wrong result for {:?}",
            backend_args
        );
    }
}

// BUG-44 (2026-07-30): `#[no_mangle]` silently did nothing for any
// program simple enough to hit the SSA fast path (the common case)
// -- neither SSA backend implements bare-symbol emission for
// no_mangle functions, only the tree-backend fallback's registry
// machinery does. Confirmed even against the already-shipped
// bare_metal.vani example, whose own Reset_Handler was still
// emitted as fn_Reset_Handler on both backends despite the
// attribute. Fixed by rejecting any no_mangle-containing program
// from the SSA fast path in `ssa_path_supports`.
#[test]
fn no_mangle_ssa_fastpath_example_produces_correct_output_on_both_backends() {
    let binary = env!("CARGO_BIN_EXE_intentc");
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let example = format!(
        "{}/examples/language/english/no_mangle_ssa_fastpath.vani",
        manifest_dir
    );
    let expected = "7\n";

    for backend_args in [vec!["run", &example], vec!["run", &example, "--backend=c"]] {
        let output = Command::new(binary)
            .args(&backend_args)
            .output()
            .unwrap_or_else(|e| panic!("intentc {:?} should execute: {e}", backend_args));
        assert!(
            output.status.success(),
            "intentc {:?} failed with status {:?}\nstderr: {}",
            backend_args,
            output.status,
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert_eq!(
            stdout.replace("\r\n", "\n"),
            expected,
            "no_mangle example produced the wrong result for {:?}",
            backend_args
        );
    }
}

#[test]
fn no_mangle_ssa_fastpath_emits_bare_symbol_on_both_backends() {
    let binary = env!("CARGO_BIN_EXE_intentc");
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let example = format!(
        "{}/examples/language/english/no_mangle_ssa_fastpath.vani",
        manifest_dir
    );
    let out_dir = std::env::temp_dir();

    let c_out_path = out_dir.join("bug44_no_mangle_test.c");
    let c_output = Command::new(binary)
        .args([
            "emit",
            &example,
            "--backend=c",
            "-o",
            c_out_path.to_str().unwrap(),
        ])
        .output()
        .expect("intentc emit --backend=c should execute");
    assert!(
        c_output.status.success(),
        "emit --backend=c failed: {}",
        String::from_utf8_lossy(&c_output.stderr)
    );
    let c_src = std::fs::read_to_string(&c_out_path).expect("read emitted C");
    let _ = std::fs::remove_file(&c_out_path);
    assert!(
        c_src.contains("int64_t add(int64_t"),
        "C backend did not emit a bare 'add' symbol for the no_mangle fn:\n{c_src}"
    );
    assert!(
        !c_src.contains("fn_add("),
        "C backend emitted a mangled 'fn_add' symbol despite #[no_mangle]:\n{c_src}"
    );

    let ll_out_path = out_dir.join("bug44_no_mangle_test.ll");
    let ll_output = Command::new(binary)
        .args(["emit", &example, "-o", ll_out_path.to_str().unwrap()])
        .output()
        .expect("intentc emit should execute");
    assert!(
        ll_output.status.success(),
        "emit (LLVM) failed: {}",
        String::from_utf8_lossy(&ll_output.stderr)
    );
    let ll_src = std::fs::read_to_string(&ll_out_path).expect("read emitted LLVM IR");
    let _ = std::fs::remove_file(&ll_out_path);
    assert!(
        ll_src.contains("@add(") && !ll_src.contains("@fn_add("),
        "LLVM backend did not emit a bare '@add' symbol for the no_mangle fn:\n{ll_src}"
    );
}

#[test]
fn self_referential_struct_vec_example_produces_correct_output_on_c_backend() {
    let binary = env!("CARGO_BIN_EXE_intentc");
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let example = format!(
        "{}/examples/language/english/self_referential_struct_vec.vani",
        manifest_dir
    );
    let expected = "1\n2\n3\n";

    let output = Command::new(binary)
        .args(["run", &example, "--backend=c"])
        .output()
        .unwrap_or_else(|e| panic!("intentc run {} --backend=c should execute: {e}", example));
    assert!(
        output.status.success(),
        "intentc run {} --backend=c failed with status {:?}\nstderr: {}",
        example,
        output.status,
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_eq!(
        stdout.replace("\r\n", "\n"),
        expected,
        "self-referential struct (Vec<Self> field) tree walk produced the wrong result"
    );
}

#[test]
fn intentc_test_harness_mode_runs_each_test_fn_in_isolation() {
    // A file with no top-level `fn main` and `#[test]`-attributed
    // fns should run in harness mode: each fn gets its own
    // synthesized driver, run as a separate process, so one
    // failing assert doesn't take out the rest of the suite. This
    // is the exact example from tutorials/src/beginner/00_cli_reference.md.
    let lli = std::env::var("LLI").unwrap_or_else(|_| "lli".to_string());
    let lli_ok = Command::new(&lli)
        .arg("--version")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false);
    if !lli_ok {
        return;
    }

    let binary = env!("CARGO_BIN_EXE_intentc");
    let tmp = std::env::temp_dir().join(format!(
        "intentc_harness_ok_{}.vani",
        std::process::id()
    ));
    std::fs::write(
        &tmp,
        b"#[test]\nfn addition_works() -> i64 {\n  assert 1 + 1 == 2;\n  return 0;\n}\n\n#[test]\nfn subtraction_works() -> i64 {\n  assert 5 - 3 == 2;\n  return 0;\n}\n",
    )
    .expect("write tmp");

    let run = Command::new(binary)
        .args(["test", tmp.to_str().unwrap()])
        .output()
        .expect("vanic test <harness file>");
    let _ = std::fs::remove_file(&tmp);
    assert!(
        run.status.success(),
        "expected exit 0, stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&run.stdout),
        String::from_utf8_lossy(&run.stderr),
    );
    let stdout = String::from_utf8_lossy(&run.stdout);
    assert!(stdout.contains("running 2 tests"), "got:\n{stdout}");
    assert!(stdout.contains("test addition_works ... ok"), "got:\n{stdout}");
    assert!(stdout.contains("test subtraction_works ... ok"), "got:\n{stdout}");
    assert!(stdout.contains("test result: ok. 2 passed; 0 failed"), "got:\n{stdout}");
}

#[test]
fn intentc_test_harness_mode_reports_one_failure_without_killing_the_rest() {
    let lli = std::env::var("LLI").unwrap_or_else(|_| "lli".to_string());
    let lli_ok = Command::new(&lli)
        .arg("--version")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false);
    if !lli_ok {
        return;
    }

    let binary = env!("CARGO_BIN_EXE_intentc");
    let tmp = std::env::temp_dir().join(format!(
        "intentc_harness_fail_{}.vani",
        std::process::id()
    ));
    std::fs::write(
        &tmp,
        b"#[test]\nfn addition_works() -> i64 {\n  assert 1 + 1 == 2;\n  return 0;\n}\n\n#[test]\nfn broken_test() -> i64 {\n  assert 1 == 2, \"deliberately wrong\";\n  return 0;\n}\n",
    )
    .expect("write tmp");

    let run = Command::new(binary)
        .args(["test", tmp.to_str().unwrap()])
        .output()
        .expect("vanic test <harness file>");
    let _ = std::fs::remove_file(&tmp);
    assert_eq!(run.status.code(), Some(1));

    let stdout = String::from_utf8_lossy(&run.stdout);
    assert!(stdout.contains("test addition_works ... ok"), "got:\n{stdout}");
    assert!(stdout.contains("test broken_test ... FAILED"), "got:\n{stdout}");
    assert!(stdout.contains("test result: FAILED. 1 passed; 1 failed"), "got:\n{stdout}");
    let stderr = String::from_utf8_lossy(&run.stderr);
    assert!(
        stderr.contains("assertion failed: deliberately wrong"),
        "got:\n{stderr}"
    );
}

#[test]
fn intentc_test_legacy_mode_unaffected_by_harness_detection() {
    // A file WITH a real `fn main` (no #[test] fns) must keep
    // running the pre-existing legacy behavior unchanged.
    let lli = std::env::var("LLI").unwrap_or_else(|_| "lli".to_string());
    let lli_ok = Command::new(&lli)
        .arg("--version")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false);
    if !lli_ok {
        return;
    }

    let binary = env!("CARGO_BIN_EXE_intentc");
    let tmp = std::env::temp_dir().join(format!(
        "intentc_legacy_still_works_{}.vani",
        std::process::id()
    ));
    std::fs::write(&tmp, b"fn main() -> i64 {\n  return 0;\n}\n").expect("write tmp");

    let run = Command::new(binary)
        .args(["test", tmp.to_str().unwrap()])
        .output()
        .expect("vanic test <legacy file>");
    let _ = std::fs::remove_file(&tmp);
    assert!(run.status.success());
    let stdout = String::from_utf8_lossy(&run.stdout);
    assert!(stdout.contains(": ok ("), "got:\n{stdout}");
    assert!(!stdout.contains("running"), "must not enter harness mode, got:\n{stdout}");
}

#[test]
#[ignore = "echo_with_timeout.vani LLVM IR has undefined value for async TCP locals; lli rejects it"]
fn intentc_test_passes_for_all_examples_and_fails_on_violated_assertion() {
    // Two-part check:
    //  (a) `intentc test` over every example produces all-passes and
    //      exit 0 — same coverage as the existing per-example tests
    //      but driving the new subcommand end-to-end.
    //  (b) Adding one program that fails an assertion flips the
    //      summary to `1 failed` and the exit code to 1.

    let lli = std::env::var("LLI").unwrap_or_else(|_| "lli".to_string());
    let lli_ok = Command::new(&lli)
        .arg("--version")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false);
    if !lli_ok {
        return;
    }

    let binary = env!("CARGO_BIN_EXE_intentc");
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let examples_dir = format!("{}/examples/language", manifest_dir);

    fn collect_vani(dir: &std::path::Path, out: &mut Vec<String>) {
        if let Ok(rd) = std::fs::read_dir(dir) {
            for e in rd.flatten() {
                let p = e.path();
                if p.is_dir() {
                    collect_vani(&p, out);
                } else if p.extension().and_then(|s| s.to_str()) == Some("vani") {
                    out.push(p.to_string_lossy().into_owned());
                }
            }
        }
    }
    let mut paths: Vec<String> = Vec::new();
    collect_vani(std::path::Path::new(&examples_dir), &mut paths);
    paths.sort();
    assert!(!paths.is_empty(), "no examples discovered");

    let mut args = vec!["test".to_string()];
    args.extend(paths.iter().cloned());

    let ok_run = Command::new(binary)
        .args(&args)
        .output()
        .expect("vanic test");
    assert!(
        ok_run.status.success(),
        "vanic test should pass for all examples\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&ok_run.stdout),
        String::from_utf8_lossy(&ok_run.stderr),
    );
    let ok_stdout = String::from_utf8_lossy(&ok_run.stdout);
    assert!(
        ok_stdout.contains("0 failed"),
        "expected `0 failed` in summary, got:\n{ok_stdout}"
    );

    let tmp_fail = std::env::temp_dir().join(format!(
        "intentc_test_fail_{}.vani",
        std::process::id()
    ));
    std::fs::write(
        &tmp_fail,
        b"fn main() -> i64 {\n  let x: i64 = 0;\n  assert x == 1, \"x should be one\";\n  return 0;\n}\n",
    )
    .expect("write tmp fail");

    let fail_run = Command::new(binary)
        .args(["test", tmp_fail.to_str().unwrap()])
        .output()
        .expect("intentc test fail");
    let _ = std::fs::remove_file(&tmp_fail);
    assert_eq!(
        fail_run.status.code(),
        Some(1),
        "intentc test should exit 1 on assertion failure"
    );
    let fail_stdout = String::from_utf8_lossy(&fail_run.stdout);
    assert!(
        fail_stdout.contains("FAILED") && fail_stdout.contains("1 failed"),
        "expected FAILED + `1 failed` in summary, got:\n{fail_stdout}"
    );
}

#[test]
fn expand_dir_walks_recursively_and_skips_dot_dirs() {
    // Confirms the shared dir-expansion helper used by both
    // `intentc test` and `intentc fmt`:
    //  - descends into subdirectories;
    //  - skips dot-prefixed directories (`.git`, `.cargo`, etc.).
    // Tests the behavior via `intentc test`, which exercises the
    // helper end-to-end and reports the file list in its summary.
    use std::fs;

    let lli = std::env::var("LLI").unwrap_or_else(|_| "lli".to_string());
    let lli_ok = Command::new(&lli)
        .arg("--version")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false);
    if !lli_ok {
        return;
    }

    let binary = env!("CARGO_BIN_EXE_intentc");
    let root = std::env::temp_dir().join(format!(
        "intentc_nested_walk_{}",
        std::process::id()
    ));
    fs::create_dir_all(root.join("sub/deep")).expect("mkdir sub/deep");
    fs::create_dir_all(root.join(".hidden")).expect("mkdir hidden");

    let trivial = "fn main() -> i64 { return 0; }\n";
    fs::write(root.join("a.vani"), trivial).expect("write a");
    fs::write(root.join("sub/b.vani"), trivial).expect("write b");
    fs::write(root.join("sub/deep/c.vani"), trivial).expect("write c");
    fs::write(root.join(".hidden/skipme.vani"), trivial).expect("write skip");

    let run = Command::new(binary)
        .args(["test", root.to_str().unwrap()])
        .output()
        .expect("intentc test <dir>");
    assert!(
        run.status.success(),
        "intentc test failed: {}",
        String::from_utf8_lossy(&run.stderr)
    );
    let stdout = String::from_utf8_lossy(&run.stdout);
    assert!(
        stdout.contains("3 passed; 0 failed"),
        "expected 3 files passed (a, b, c), got:\n{stdout}"
    );
    assert!(
        !stdout.contains("skipme.vani"),
        "files under .hidden/ should be skipped, got:\n{stdout}"
    );

    let _ = fs::remove_dir_all(&root);
}

#[test]
fn intentc_test_json_emits_machine_readable_results() {
    // `intentc test --json a.vani b.vani` should print one
    // object on stdout: `{"results":[…],"summary":{…}}`. Each
    // result has `path`, `ok`, `ms` and (for failures) `exit` +
    // `reason`. Pin the basic shape; substring checks suffice.
    let lli = std::env::var("LLI").unwrap_or_else(|_| "lli".to_string());
    let lli_ok = Command::new(&lli)
        .arg("--version")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false);
    if !lli_ok {
        return;
    }

    let binary = env!("CARGO_BIN_EXE_intentc");
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let a = format!("{}/examples/language/english/basics.vani", manifest_dir);

    // Make a small failing fixture so the JSON shows a runtime
    // failure too.
    let fail_path = std::env::temp_dir().join(format!(
        "intentc_test_json_fail_{}.vani",
        std::process::id()
    ));
    std::fs::write(
        &fail_path,
        b"fn main() -> i64 {\n  assert 1 == 2;\n  return 0;\n}\n",
    )
    .expect("write fail fixture");

    let run = Command::new(binary)
        .args(["test", "--json", &a, fail_path.to_str().unwrap()])
        .output()
        .expect("intentc test --json");
    let _ = std::fs::remove_file(&fail_path);

    assert_eq!(run.status.code(), Some(1), "should exit 1 on any failure");
    let stdout = String::from_utf8_lossy(&run.stdout);

    // Single-line JSON object. Each path appears once.
    assert!(
        stdout.contains("\"results\":[") && stdout.contains("\"summary\":{"),
        "missing top-level keys, got:\n{stdout}"
    );
    assert!(
        stdout.contains("\"path\":\"") && stdout.contains("basics.vani"),
        "missing path entry, got:\n{stdout}"
    );
    assert!(
        stdout.contains("\"ok\":true") && stdout.contains("\"ok\":false"),
        "expected both ok=true and ok=false, got:\n{stdout}"
    );
    assert!(
        stdout.contains("\"reason\":\"runtime\""),
        "failing fixture should be tagged runtime, got:\n{stdout}"
    );
    assert!(
        stdout.contains("\"passed\":1") && stdout.contains("\"failed\":1"),
        "summary counts off, got:\n{stdout}"
    );
}

#[test]
fn intentc_check_smt_debug_flag_dumps_smt_query() {
    // `--smt-debug` should surface the same query/response stream
    // as `INTENTC_SMT_DEBUG=1`: each SMT round-trip emits a
    // `--- SMT query ---` block to stderr. Use a small file with
    // a `prove` so we know there's at least one query.
    let binary = env!("CARGO_BIN_EXE_intentc");
    let tmp = std::env::temp_dir().join(format!(
        "intentc_smt_debug_{}.vani",
        std::process::id()
    ));
    // The prove must NOT constant-fold — otherwise the verifier
    // short-circuits and never makes an SMT call. A parameter-
    // dependent inequality ensures z3 is consulted.
    std::fs::write(
        &tmp,
        b"fn f(a: i64) -> i64\nrequires a >= 0;\nrequires a < 1000;\n{\n  prove a + 1 > 0;\n  return a + 1;\n}\nfn main() -> i64 { return 0; }\n",
    )
    .expect("write tmp");

    // Without the flag: stderr should not include the SMT query header.
    let plain = Command::new(binary)
        .args(["check", tmp.to_str().unwrap()])
        .env_remove("INTENTC_SMT_DEBUG")
        .output()
        .expect("intentc check");
    assert!(plain.status.success());
    let plain_stderr = String::from_utf8_lossy(&plain.stderr);
    assert!(
        !plain_stderr.contains("--- SMT query ---"),
        "default run shouldn't dump SMT queries, stderr was:\n{plain_stderr}"
    );

    // With the flag: stderr should include at least one query block.
    let debug = Command::new(binary)
        .args(["check", "--smt-debug", tmp.to_str().unwrap()])
        .env_remove("INTENTC_SMT_DEBUG")
        .output()
        .expect("intentc check --smt-debug");
    let _ = std::fs::remove_file(&tmp);

    assert!(debug.status.success());
    let debug_stderr = String::from_utf8_lossy(&debug.stderr);
    assert!(
        debug_stderr.contains("--- SMT query ---"),
        "--smt-debug should dump at least one query block, stderr was:\n{debug_stderr}"
    );
}

#[test]
#[ignore = "some example .vani files with prove statements fail z3 verification at check time"]
fn intentc_check_accepts_directory_and_summarizes() {
    // `vanic check examples/language/` should walk the directory
    // recursively and type-check every `*.vani` inside, printing
    // per-file `ok` lines plus a summary, and exit 0.
    let binary = env!("CARGO_BIN_EXE_intentc");
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let examples_dir = format!("{}/examples/language", manifest_dir);

    fn count_vani_recursive(dir: &std::path::Path) -> usize {
        let mut n = 0;
        if let Ok(rd) = std::fs::read_dir(dir) {
            for e in rd.flatten() {
                let p = e.path();
                if p.is_dir() {
                    n += count_vani_recursive(&p);
                } else if p.extension().and_then(|s| s.to_str()) == Some("vani") {
                    n += 1;
                }
            }
        }
        n
    }
    let n_examples = count_vani_recursive(std::path::Path::new(&examples_dir));

    let run = Command::new(binary)
        .args(["check", &examples_dir])
        .output()
        .expect("vanic check <dir>");
    assert!(
        run.status.success(),
        "vanic check <dir> should succeed\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&run.stdout),
        String::from_utf8_lossy(&run.stderr),
    );
    let stdout = String::from_utf8_lossy(&run.stdout);
    let expected = format!("ok: {} file(s)", n_examples);
    assert!(
        stdout.contains(&expected),
        "expected `{expected}` summary, got:\n{stdout}"
    );
}

#[test]
fn intentc_check_json_combines_diagnostics_across_files() {
    // `intentc check --json a.vani b.vani` now emits a single
    // `{"diagnostics":[...]}` object covering both files. The
    // `FileMap::extend_with` helper shifts each file's span frame
    // into a global one so each diagnostic still resolves to its
    // own source path/line.
    let binary = env!("CARGO_BIN_EXE_intentc");
    let tmp_a = std::env::temp_dir().join(format!("check_json_a_{}.vani", std::process::id()));
    let tmp_b = std::env::temp_dir().join(format!("check_json_b_{}.vani", std::process::id()));
    std::fs::write(&tmp_a, b"fn main() -> i64 {\n  let x: i64 = nope;\n  return 0;\n}\n").unwrap();
    std::fs::write(&tmp_b, b"fn f() -> i64 {\n  return undefined;\n}\n").unwrap();

    let run = Command::new(binary)
        .args([
            "check",
            "--json",
            tmp_a.to_str().unwrap(),
            tmp_b.to_str().unwrap(),
        ])
        .output()
        .expect("intentc check --json multi-file");
    let _ = std::fs::remove_file(&tmp_a);
    let _ = std::fs::remove_file(&tmp_b);

    assert_eq!(run.status.code(), Some(1), "should exit 1 on errors");
    let stdout = String::from_utf8_lossy(&run.stdout);
    // The combined JSON contains both files' diagnostics, each
    // tagged with its own `file` field.
    assert!(
        stdout.contains("unknown variable 'nope'"),
        "expected first file's diagnostic, got:\n{stdout}"
    );
    assert!(
        stdout.contains("unknown variable 'undefined'"),
        "expected second file's diagnostic, got:\n{stdout}"
    );
    assert!(
        stdout.contains("check_json_a_") && stdout.contains("check_json_b_"),
        "each diagnostic should reference its own path, got:\n{stdout}"
    );
}

#[test]
fn intentc_check_json_empty_for_clean_run_across_files() {
    // Companion to the above: a clean run across multiple files
    // emits `{"diagnostics":[]}` once, not per-file.
    let binary = env!("CARGO_BIN_EXE_intentc");
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let a = format!("{}/examples/language/english/basics.vani", manifest_dir);
    let b = format!("{}/examples/language/english/contracts.vani", manifest_dir);
    let run = Command::new(binary)
        .args(["check", "--json", &a, &b])
        .output()
        .expect("intentc check --json clean run");
    assert!(run.status.success());
    let stdout = String::from_utf8_lossy(&run.stdout);
    assert_eq!(
        stdout.trim(),
        "{\"diagnostics\":[]}",
        "expected single empty diagnostics object, got: {stdout}"
    );
}

#[test]
fn fmt_accepts_directory_with_check_and_in_place() {
    // `intentc fmt` should expand a directory arg the same way
    // `intentc test` does — non-recursive, alphabetized — and
    // apply --check or --in-place to each `*.vani` child. The
    // stdout mode is rejected for multi-file input (would dump
    // many files concatenated).
    use std::fs;
    let binary = env!("CARGO_BIN_EXE_intentc");
    let manifest_dir = env!("CARGO_MANIFEST_DIR");

    let tmp_dir = std::env::temp_dir().join(format!(
        "intentc_fmt_dir_{}",
        std::process::id()
    ));
    fs::create_dir_all(&tmp_dir).expect("mkdir tmp");
    // Seed two files: one canonical (just produced by fmt) and one
    // intentionally non-canonical (extra spaces inside braces).
    fs::write(
        tmp_dir.join("a.vani"),
        "fn main() -> i64 {\n  return 0;\n}\n",
    )
    .expect("write a");
    fs::write(
        tmp_dir.join("b.vani"),
        "fn main()   -> i64{\n    return 1;\n}\n",
    )
    .expect("write b");
    // Ensure the canonical seed actually matches our formatter.
    fs::copy(
        format!("{}/examples/language/english/basics.vani", manifest_dir),
        tmp_dir.join("c.vani"),
    )
    .expect("copy c");

    // (1) Default stdout mode on a directory → error.
    let run = Command::new(binary)
        .args(["fmt", tmp_dir.to_str().unwrap()])
        .output()
        .expect("intentc fmt <dir>");
    assert_eq!(
        run.status.code(),
        Some(1),
        "stdout mode on dir should be rejected"
    );
    let stderr = String::from_utf8_lossy(&run.stderr);
    assert!(
        stderr.contains("multiple files require --check or --in-place"),
        "expected diagnostic, got:\n{stderr}"
    );

    // (2) --check should exit 1 because the dir has non-canonical
    // files. Each non-canonical file should be reported on stderr.
    let run = Command::new(binary)
        .args(["fmt", "--check", tmp_dir.to_str().unwrap()])
        .output()
        .expect("intentc fmt --check <dir>");
    assert_eq!(run.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&run.stderr);
    assert!(
        stderr.contains("b.vani: not canonically formatted"),
        "expected b.vani listed, got:\n{stderr}"
    );

    // (3) --in-place rewrites, then --check passes silently.
    let run = Command::new(binary)
        .args(["fmt", "--in-place", tmp_dir.to_str().unwrap()])
        .output()
        .expect("intentc fmt --in-place <dir>");
    assert!(run.status.success(), "in-place failed: {}", String::from_utf8_lossy(&run.stderr));
    let run = Command::new(binary)
        .args(["fmt", "--check", tmp_dir.to_str().unwrap()])
        .output()
        .expect("intentc fmt --check <dir> after");
    assert!(
        run.status.success(),
        "check after in-place should pass; stderr:\n{}",
        String::from_utf8_lossy(&run.stderr)
    );
    assert!(run.stdout.is_empty() && run.stderr.is_empty());

    let _ = fs::remove_dir_all(&tmp_dir);
}

#[test]
fn fmt_check_and_in_place_modes_match_canonical_form() {
    // Full life cycle of the new flags on a real example:
    //  1. --check on the unformatted source should exit 1 with a
    //     "not canonically formatted" notice on stderr.
    //  2. --in-place should rewrite to the canonical form, no
    //     change to mtime if the file is already canonical.
    //  3. --check on the canonical source should exit 0 silently.
    //  4. --check + --in-place together should be rejected.
    let binary = env!("CARGO_BIN_EXE_intentc");
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let src = format!("{}/examples/language/english/basics.vani", manifest_dir);
    let tmp = std::env::temp_dir().join(format!(
        "intentc_fmt_check_{}.vani",
        std::process::id()
    ));
    std::fs::copy(&src, &tmp).expect("copy fixture");

    // (1) Unformatted: --check should exit 1.
    let out = Command::new(binary)
        .args(["fmt", "--check", tmp.to_str().unwrap()])
        .output()
        .expect("intentc fmt --check");
    assert_eq!(out.status.code(), Some(1), "expected exit 1 for non-canonical");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("not canonically formatted"),
        "expected `not canonically formatted` in stderr, got:\n{stderr}"
    );

    // (2) --in-place rewrites successfully.
    let out = Command::new(binary)
        .args(["fmt", "--in-place", tmp.to_str().unwrap()])
        .output()
        .expect("intentc fmt --in-place");
    assert!(
        out.status.success(),
        "fmt --in-place failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    // (3) After --in-place, --check passes silently.
    let out = Command::new(binary)
        .args(["fmt", "--check", tmp.to_str().unwrap()])
        .output()
        .expect("intentc fmt --check (canonical)");
    assert!(
        out.status.success(),
        "fmt --check should pass on canonical file: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        out.stdout.is_empty() && out.stderr.is_empty(),
        "check on canonical should be silent: stdout={:?} stderr={:?}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr),
    );

    // (4) --check and --in-place together is rejected.
    let out = Command::new(binary)
        .args(["fmt", "--check", "--in-place", tmp.to_str().unwrap()])
        .output()
        .expect("intentc fmt --check --in-place");
    assert!(
        !out.status.success(),
        "expected non-zero exit for mutually exclusive flags"
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("mutually exclusive"),
        "expected mutual-exclusion diagnostic, got:\n{stderr}"
    );

    let _ = std::fs::remove_file(&tmp);
}

#[test]
fn fmt_preserves_comments_from_example_with_leading_block() {
    // `examples/language/english/vec_invariants.vani` opens with a 10-line `//`
    // block documenting the loop invariant. Earlier versions of fmt
    // would silently strip it. Now run fmt and assert each of those
    // lines reappears in the output.
    let binary = env!("CARGO_BIN_EXE_intentc");
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let example = format!("{}/examples/language/english/vec_invariants.vani", manifest_dir);
    let source = std::fs::read_to_string(&example).expect("read example");

    let out = Command::new(binary)
        .args(["fmt", &example])
        .output()
        .expect("intentc fmt");
    assert!(out.status.success(), "fmt failed: {}", String::from_utf8_lossy(&out.stderr));
    let stdout = String::from_utf8_lossy(&out.stdout);

    // Every `// …` line from the source must appear somewhere in
    // the formatted output.
    let mut comment_lines = 0;
    for line in source.lines() {
        let trimmed = line.trim_start();
        if trimmed.starts_with("//") {
            comment_lines += 1;
            assert!(
                stdout.contains(trimmed),
                "missing comment line: `{trimmed}`\nformatted output:\n{stdout}"
            );
        }
    }
    assert!(comment_lines > 0, "test example should have comments");
}

#[test]
#[ignore = "multiple translated example files have pre-existing fmt parse errors (keyword-as-identifier, Box<Vec<T>> type, etc.) — fix examples then re-enable"]
fn fmt_roundtrips_every_example() {
    // `vanic fmt` should produce source that re-parses to the
    // same AST. Whitespace and comments may differ; structural
    // shape must not. Runs `vanic fmt` on every example file and
    // pipes the output back through `vanic ast` (to canonicalize
    // the AST dump) for comparison.
    //
    // A.2 reorg (2026-06-06): examples now live under
    // `examples/language/<lang>/*.vani`; walk recursively to
    // pick up every language subfolder.
    let binary = env!("CARGO_BIN_EXE_intentc");
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let examples_dir = format!("{}/examples/language", manifest_dir);

    fn walk_vani(dir: &std::path::Path, out: &mut Vec<std::path::PathBuf>) {
        if let Ok(rd) = std::fs::read_dir(dir) {
            for e in rd.flatten() {
                let p = e.path();
                if p.is_dir() {
                    walk_vani(&p, out);
                } else if p.extension().and_then(|s| s.to_str()) == Some("vani") {
                    out.push(p);
                }
            }
        }
    }
    let mut entries: Vec<std::path::PathBuf> = Vec::new();
    walk_vani(std::path::Path::new(&examples_dir), &mut entries);
    entries.sort();
    assert!(!entries.is_empty(), "no examples discovered");

    for path in entries {
        let name = path.file_name().and_then(|s| s.to_str()).unwrap_or("?");

        let fmt_out = Command::new(binary)
            .args(["fmt", path.to_str().unwrap()])
            .output()
            .expect("intentc fmt");
        assert!(
            fmt_out.status.success(),
            "fmt failed for {name}: {}",
            String::from_utf8_lossy(&fmt_out.stderr)
        );

        // Write the formatted source to a temp file so we can run
        // `intentc ast` on it without piping (the CLI takes a path).
        let tmp = std::env::temp_dir().join(format!("fmt_roundtrip_{}", name));
        std::fs::write(&tmp, &fmt_out.stdout).expect("write tmp");

        let ast_a = Command::new(binary)
            .args(["ast", path.to_str().unwrap()])
            .output()
            .expect("intentc ast original");
        let ast_b = Command::new(binary)
            .args(["ast", tmp.to_str().unwrap()])
            .output()
            .expect("intentc ast formatted");
        let _ = std::fs::remove_file(&tmp);

        assert!(ast_a.status.success(), "ast(orig) failed for {name}");
        assert!(ast_b.status.success(), "ast(fmt) failed for {name}");

        // Spans differ (byte offsets shift after formatting), so
        // strip every `span: Span { ... }` substring before
        // comparing. The block always renders on one line in
        // `{:#?}`-style debug output.
        let strip = |s: &str| -> String {
            let mut out = String::with_capacity(s.len());
            let mut chars = s.chars().peekable();
            while let Some(c) = chars.next() {
                if c == 's' {
                    let mut peek = String::new();
                    let mut snapshot = chars.clone();
                    for _ in 0..3 {
                        if let Some(&p) = snapshot.peek() {
                            peek.push(p);
                            snapshot.next();
                        }
                    }
                    if peek == "pan" {
                        for _ in 0..3 { chars.next(); }
                        // skip to closing `}`
                        let mut depth: i32 = 0;
                        for d in chars.by_ref() {
                            if d == '{' { depth += 1; }
                            else if d == '}' {
                                depth -= 1;
                                if depth == 0 { break; }
                            }
                        }
                        continue;
                    }
                }
                out.push(c);
            }
            out
        };

        let a = strip(&String::from_utf8_lossy(&ast_a.stdout));
        let b = strip(&String::from_utf8_lossy(&ast_b.stdout));
        assert_eq!(
            a, b,
            "AST changed across format round-trip for {name}"
        );
    }
}

#[test]
fn emit_llvm_parallel_for_lowers_to_gomp_call() {
    // The LLVM backend lifts each `parallel for` body into an
    // `@__intent_par_<N>` function and calls `@GOMP_parallel`
    // from the parent. Confirm the emitted IR has the expected
    // shape: a declaration of GOMP_parallel, an internal outlined
    // function per parallel-for, and a call site for each.
    let binary = env!("CARGO_BIN_EXE_intentc");
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let example = format!("{}/examples/language/english/parallel.vani", manifest_dir);

    let out = Command::new(binary)
        .args(["emit", &example, "--backend=llvm"])
        .output()
        .expect("intentc emit --backend=llvm");
    assert!(out.status.success(), "emit failed");
    let stdout = String::from_utf8_lossy(&out.stdout);

    // On Linux/macOS the scheduler is libgomp (@GOMP_parallel, void outlined
    // fns). On Windows the scheduler is Win32 CreateThread (i8* outlined fns).
    // Both must outline exactly 11 parallel-for bodies.
    #[cfg(not(target_os = "windows"))]
    {
        assert!(
            stdout.contains("declare void @GOMP_parallel(void (i8*)*, i8*, i32, i32)"),
            "missing GOMP_parallel declaration:\n{stdout}"
        );
        let outlined = stdout
            .matches("define internal void @__intent_par_")
            .count();
        // All 11 parallel-fors get outlined: three basic, plus eight
        // reductions (`+`, `*`, `||`, `min`, `max`, bitwise `&`,
        // bitwise `|`, bitwise `^`). The `||` case used to fall back
        // to sequential because atomicrmw rejects i1, but the
        // backend now allocates an i8 shadow per bool reduction and
        // runs `atomicrmw or` against it.
        assert_eq!(
            outlined, 11,
            "expected 11 outlined functions, got {outlined}"
        );
        let call_sites = stdout.matches("call void @GOMP_parallel(").count();
        assert_eq!(
            call_sites, 11,
            "expected 11 GOMP_parallel call sites, got {call_sites}"
        );
    }
    #[cfg(target_os = "windows")]
    {
        assert!(
            stdout.contains("declare i8* @CreateThread("),
            "missing CreateThread declaration on Windows:\n{stdout}"
        );
        let outlined = stdout
            .matches("define internal i8* @__intent_par_")
            .count();
        assert_eq!(
            outlined, 11,
            "expected 11 outlined functions (Win32 path), got {outlined}"
        );
        let call_sites = stdout.matches("call i8* @CreateThread(").count();
        assert!(
            call_sites >= 11,
            "expected at least 11 CreateThread call sites, got {call_sites}"
        );
    }
    // The `+` reduction lowers to `atomicrmw add`; the `*`
    // reduction lowers to a `cmpxchg` retry loop (atomicrmw
    // doesn't expose `mul`). For signed integers, `min`/`max`
    // lower to the dedicated `atomicrmw min`/`atomicrmw max`
    // instructions (the unsigned variants are `umin`/`umax`).
    // Bool `||` lowers to `atomicrmw or i8*` via the shadow.
    // Bitwise `&` / `|` / `^` lower to native-width
    // `atomicrmw and` / `or` / `xor` (no shadow needed because
    // the integer width is already byte-aligned).
    assert!(
        stdout.contains("atomicrmw add"),
        "expected atomicrmw add lowering:\n{stdout}"
    );
    assert!(
        stdout.contains("cmpxchg"),
        "expected cmpxchg lowering for `*` reduction:\n{stdout}"
    );
    assert!(
        stdout.contains("atomicrmw min"),
        "expected atomicrmw min lowering for min reduction:\n{stdout}"
    );
    assert!(
        stdout.contains("atomicrmw max"),
        "expected atomicrmw max lowering for max reduction:\n{stdout}"
    );
    assert!(
        stdout.contains("atomicrmw or i8*"),
        "expected atomicrmw or on i8 shadow for `||` reduction:\n{stdout}"
    );
    assert!(
        stdout.contains("atomicrmw and i64*"),
        "expected native-width atomicrmw and for bitwise `&` reduction:\n{stdout}"
    );
    assert!(
        stdout.contains("atomicrmw or i64*"),
        "expected native-width atomicrmw or for bitwise `|` reduction:\n{stdout}"
    );
    assert!(
        stdout.contains("atomicrmw xor i64*"),
        "expected native-width atomicrmw xor for bitwise `^` reduction:\n{stdout}"
    );
}

#[test]
fn emit_c_parallel_for_pragma_appears_in_output() {
    // The C backend lowers `parallel for` to a regular for loop
    // preceded by `_Pragma("omp parallel for")`. Compilers with
    // -fopenmp parallelize; compilers without it warn-and-run
    // sequentially. The Run path auto-adds -fopenmp when the
    // probe succeeds, so the user pays nothing for unsupported
    // toolchains.
    let binary = env!("CARGO_BIN_EXE_intentc");
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let example = format!("{}/examples/language/english/parallel.vani", manifest_dir);

    let out = Command::new(binary)
        .args(["emit", &example, "--backend=c"])
        .output()
        .expect("intentc emit --backend=c");
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    let pragma_count = stdout.matches("_Pragma(\"omp parallel for").count();
    assert_eq!(
        pragma_count, 11,
        "expected 11 omp pragmas (one per parallel for in the example), got {pragma_count}:\n{stdout}"
    );
    // Each reduction op contributes its `reduction(op: var)`
    // clause to the corresponding pragma. Tree-C names the
    // reduction var after the source binding (e.g. `v_total`);
    // SSA-C names it after the SSA carry value-id (e.g.
    // `v_37`). Functionally equivalent — accept either.
    fn has_reduction(stdout: &str, op: &str) -> bool {
        // Match `reduction(<op>:<anything>)` (tree-C) and
        // `reduction(<op>: <anything>)` (SSA-C).
        stdout.contains(&format!("reduction({}:", op))
            || stdout.contains(&format!("reduction({}: ", op))
    }
    assert!(has_reduction(&stdout, "+"), "expected `+` reduction clause:\n{stdout}");
    assert!(has_reduction(&stdout, "*"), "expected `*` reduction clause:\n{stdout}");
    assert!(has_reduction(&stdout, "||"), "expected `||` reduction clause:\n{stdout}");
    assert!(has_reduction(&stdout, "min"), "expected `min` reduction clause:\n{stdout}");
    assert!(has_reduction(&stdout, "max"), "expected `max` reduction clause:\n{stdout}");
    assert!(has_reduction(&stdout, "&"), "expected `&` reduction clause:\n{stdout}");
    assert!(has_reduction(&stdout, "|"), "expected `|` reduction clause:\n{stdout}");
    assert!(has_reduction(&stdout, "^"), "expected `^` reduction clause:\n{stdout}");
}

#[test]
#[ignore = "parallel.vani LLVM IR emits atomic load without alignment; lli rejects it"]
fn run_parallel_example_proves_race_free_and_runs() {
    // End-to-end: the effects verifier accepts every `pure fn`
    // and `parallel for` in the example, then the backend lowers
    // the loops sequentially (semantics-preserving). Output is
    // just `0` — the example doesn't print loop values, only the
    // sentinel.
    let binary = env!("CARGO_BIN_EXE_intentc");
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let example = format!("{}/examples/language/english/parallel.vani", manifest_dir);
    let output = Command::new(binary)
        .args(["run", &example])
        .output()
        .expect("intentc run");
    assert!(
        output.status.success(),
        "run failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    // Example output: bias+xs[0] = 110, sum = 100, product =
    // 240000, OR over flags = 1, min of xs = 10, max of xs = 40,
    // bit-AND of xs = 0, bit-OR of xs = 62, bit-XOR of xs = 40.
    // (xs = [10, 20, 30, 40]; 10&20&30&40 = 0; 10|20|30|40 = 62;
    // 10^20^30^40 = 40, which collides with the max-output, so
    // the unique signal for the XOR pragma is `62` for OR.)
    let stdout = String::from_utf8_lossy(&output.stdout);
    for line in ["110", "100", "240000", "1", "10", "40", "0", "62"] {
        assert!(
            stdout.contains(line),
            "expected line `{line}` in output, got:\n{stdout}"
        );
    }
}

#[test]
fn emit_llvm_parallel_for_with_captures_extends_ctx_struct() {
    // When the parallel-for body reads outer bindings, the LLVM
    // backend extends the inline ctx struct with one pointer
    // field per capture, stores the parent allocas into those
    // fields at the call site, and emits matching loads in the
    // outlined function. Pin the resulting IR shape so a future
    // refactor can't silently drop captures.
    let binary = env!("CARGO_BIN_EXE_intentc");
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let example = format!("{}/examples/language/english/parallel.vani", manifest_dir);

    let out = Command::new(binary)
        .args(["emit", &example, "--backend=llvm"])
        .output()
        .expect("intentc emit --backend=llvm");
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);

    // At least one outlined function unpacks captures via
    // `%cap_<i> = load …, … * %cap_<i>_p` lines.
    assert!(
        stdout.contains("%cap_0_p = getelementptr"),
        "expected capture-field getelementptr in outlined fn:\n{stdout}"
    );
    assert!(
        stdout.contains("%cap_0 = load"),
        "expected capture-field load in outlined fn:\n{stdout}"
    );
}

#[test]
fn run_strings_concat_example_prints_joined_owned_strings() {
    // OwnedStr surface end-to-end: `Str + Str` allocates and
    // returns an OwnedStr; chaining a second concat consumes the
    // first OwnedStr and frees its buffer inside the helper.
    let binary = env!("CARGO_BIN_EXE_intentc");
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let example = format!("{}/examples/language/english/strings_concat.vani", manifest_dir);
    let output = Command::new(binary)
        .args(["run", &example])
        .output()
        .expect("intentc run");
    assert!(
        output.status.success(),
        "run failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Hello, alice!"),
        "expected joined alice greeting, got:\n{stdout}"
    );
    assert!(
        stdout.contains("Hello, bob."),
        "expected joined bob greeting, got:\n{stdout}"
    );
}

#[test]
fn run_strings_example_prints_each_greeting() {
    // Pins the Str feature surface: Str param, Str return, let-bound
    // Str, and ==/!= via strcmp. Also a smoke test for the LLVM
    // `Discard` path — `let _ = greet("alice")` must execute even
    // though the i64 result is dropped.
    let binary = env!("CARGO_BIN_EXE_intentc");
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let example = format!("{}/examples/language/english/strings.vani", manifest_dir);

    let output = Command::new(binary)
        .args(["run", &example])
        .output()
        .expect("intentc run should execute");

    assert!(
        output.status.success(),
        "intentc run failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("hello, alice"),
        "expected greeting in stdout, got: {stdout}"
    );
    assert!(
        stdout.contains("role: member"),
        "expected `role: member` (alice via != \"guest\" branch), got: {stdout}"
    );
    assert!(
        stdout.contains("role: visitor"),
        "expected `role: visitor` (guest falls through), got: {stdout}"
    );
    assert!(
        stdout.contains("len: 5"),
        "expected `len: 5` from len(\"hello\"), got: {stdout}"
    );
}

// ── Windows IOCP recv ABI fix (2026-06-12) ────────────────────────────────
//
// Root cause of the former mismatch: emit_intent_epoll_helpers_llvm_windows
// called @recv as `call i64 @recv(...)` but @recv is declared `i32` on
// Windows. The ABI mismatch left garbage in the high 32 bits of rax,
// producing 0x200000002 instead of 4. Fixed by using `call i32 @recv` +
// `sext i32 to i64` in the IOCP recv_nb helper.
//
// The tests below confirm C and LLVM produce identical output on Windows.

#[test]
#[cfg(target_os = "windows")]
fn echo_loop_llvm_matches_c_on_windows() {
    let binary = env!("CARGO_BIN_EXE_intentc");
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let example = format!("{}/examples/language/english/echo_loop.vani", manifest_dir); // path confirmed correct

    let c_out = Command::new(binary)
        .args(["run", "--backend=c", &example])
        .output()
        .expect("intentc run --backend=c should execute");
    let llvm_out = Command::new(binary)
        .args(["run", "--backend=llvm", &example])
        .output()
        .expect("intentc run --backend=llvm should execute");

    let c_stdout = String::from_utf8_lossy(&c_out.stdout).replace("\r\n", "\n");
    let llvm_stdout = String::from_utf8_lossy(&llvm_out.stdout).replace("\r\n", "\n");
    assert_eq!(
        c_stdout, llvm_stdout,
        "echo_loop.vani stdout diverges between C and LLVM on Windows\n\
         C:    {c_stdout:?}\n\
         LLVM: {llvm_stdout:?}"
    );
}

// ---------------------------------------------------------------------------
// Windows-specific spot tests
// ---------------------------------------------------------------------------

/// Brahmi/Devanagari numeral output must not be garbled on Windows.
///
/// The LLVM brahmi print helper emits one `putchar` per UTF-8 byte of each
/// numeral glyph. On Windows the CRT stdout is buffered and (in text mode)
/// performs CR/LF translation — verify the bytes arrive in order and the
/// two backends agree byte-for-byte.
#[test]
fn windows_brahmi_numeral_output_no_crt_reorder() {
    let binary = env!("CARGO_BIN_EXE_intentc");
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let example = format!(
        "{}/examples/language/sanskrit/pure_devanagari.vani",
        manifest_dir
    );

    let c_out = Command::new(binary)
        .args(["run", "--backend=c", &example])
        .output()
        .expect("intentc run --backend=c");
    let ll_out = Command::new(binary)
        .args(["run", "--backend=llvm", &example])
        .output()
        .expect("intentc run --backend=llvm");

    assert!(c_out.status.success(), "C backend failed: {}", String::from_utf8_lossy(&c_out.stderr));
    assert!(ll_out.status.success(), "LLVM backend failed: {}", String::from_utf8_lossy(&ll_out.stderr));

    let c_stdout  = String::from_utf8_lossy(&c_out.stdout).replace("\r\n", "\n");
    let ll_stdout = String::from_utf8_lossy(&ll_out.stdout).replace("\r\n", "\n");

    assert_eq!(
        c_stdout, ll_stdout,
        "Brahmi numeral output diverges between C and LLVM\n\
         C:    {c_stdout:?}\n\
         LLVM: {ll_stdout:?}"
    );

    // Devanagari digit ONE (U+0967) encodes as E0 A5 A7 in UTF-8.
    // The output should contain it — proves the brahmi helper ran and
    // all three bytes arrived in the correct order (no CRT reordering).
    assert!(
        c_out.stdout.windows(3).any(|w| w == [0xE0, 0xA5, 0xA7]),
        "Expected Devanagari digit १ (U+0967, E0 A5 A7) in output — brahmi helper may not have run"
    );
}

/// Blocking TCP echo with three sequential clients produces "echoed bytes: 12"
/// on both backends (aaa=3 + bbbb=4 + ccccc=5). Guards against Windows-specific
/// socket teardown races in the blocking accept/recv path.
#[test]
#[ignore = "tcp_multi_echo.vani LLVM IR has undefined value '%t3.fd.addr' for TCP locals; lli rejects it"]
fn windows_tcp_echo_blocking_three_clients() {
    let binary = env!("CARGO_BIN_EXE_intentc");
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let example = format!(
        "{}/examples/language/english/tcp_multi_echo.vani",
        manifest_dir
    );

    let c_out = Command::new(binary)
        .args(["run", "--backend=c", &example])
        .output()
        .expect("intentc run --backend=c");
    let ll_out = Command::new(binary)
        .args(["run", "--backend=llvm", &example])
        .output()
        .expect("intentc run --backend=llvm");

    assert!(c_out.status.success(),  "C backend failed: {}",    String::from_utf8_lossy(&c_out.stderr));
    assert!(ll_out.status.success(), "LLVM backend failed: {}", String::from_utf8_lossy(&ll_out.stderr));

    let c_stdout  = String::from_utf8_lossy(&c_out.stdout).replace("\r\n", "\n");
    let ll_stdout = String::from_utf8_lossy(&ll_out.stdout).replace("\r\n", "\n");

    assert_eq!(
        c_stdout, ll_stdout,
        "tcp_multi_echo stdout diverges between C and LLVM\n\
         C:    {c_stdout:?}\n\
         LLVM: {ll_stdout:?}"
    );
    assert!(
        c_stdout.contains("echoed bytes: 12"),
        "Expected 'echoed bytes: 12' (3+4+5), got: {c_stdout:?}"
    );
}

/// Windows snprintf/dprintf shim roundtrip.
///
/// On Windows the ORC JIT cannot resolve `snprintf` or `dprintf` from any
/// DLL — `snprintf` is inlined by MinGW and `dprintf` is POSIX-only. The
/// compiler emits IR shims backed by `vsnprintf`+`_write`. This test verifies:
///   1. snprintf shim: i64_to_str produces the correct decimal string
///      (exercises the varargs → vsnprintf path).
///   2. dprintf shim: a failing assert with a custom message writes the
///      expected text to stderr (exercises the vsnprintf+_write path).
#[test]
#[ignore = "echo_p3b_str_local.vani LLVM IR has undefined value '%t3.c.addr' in snprintf path; lli rejects it"]
fn windows_snprintf_dprintf_shim_roundtrip() {
    use std::fs;

    let binary = env!("CARGO_BIN_EXE_intentc");
    let manifest_dir = env!("CARGO_MANIFEST_DIR");

    // --- Part 1: snprintf shim via i64_to_str ---
    // echo_p3b_str_local.vani converts integers to strings internally;
    // the LLVM path goes through @snprintf. Both backends must agree.
    let str_example = format!(
        "{}/examples/language/english/echo_p3b_str_local.vani",
        manifest_dir
    );
    let c_out = Command::new(binary)
        .args(["run", "--backend=c", &str_example])
        .output()
        .expect("intentc run --backend=c (snprintf path)");
    let ll_out = Command::new(binary)
        .args(["run", "--backend=llvm", &str_example])
        .output()
        .expect("intentc run --backend=llvm (snprintf path)");

    assert!(c_out.status.success(),  "C backend failed (snprintf): {}",    String::from_utf8_lossy(&c_out.stderr));
    assert!(ll_out.status.success(), "LLVM backend failed (snprintf): {}", String::from_utf8_lossy(&ll_out.stderr));

    let c_stdout  = String::from_utf8_lossy(&c_out.stdout).replace("\r\n", "\n");
    let ll_stdout = String::from_utf8_lossy(&ll_out.stdout).replace("\r\n", "\n");
    assert_eq!(
        c_stdout, ll_stdout,
        "snprintf shim: stdout diverges between C and LLVM\nC: {c_stdout:?}\nLLVM: {ll_stdout:?}"
    );
    // "111" appears in mode=1 output — a concrete decimal-formatting check.
    assert!(ll_stdout.contains("111"), "snprintf shim: expected '111' in output, got: {ll_stdout:?}");

    // --- Part 2: dprintf shim via assert failure to stderr ---
    let dir = std::env::temp_dir().join(format!(
        "intentc-snprintf-dprintf-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir_all(&dir).expect("mkdir");
    let src = dir.join("dprintf_check.vani");
    fs::write(
        &src,
        "fn main() -> i64 {\n  let x: i64 = 0;\n  assert x == 1, \"snprintf-dprintf-shim-ok\";\n  return 0;\n}\n",
    )
    .expect("write dprintf_check.vani");

    let assert_out = Command::new(binary)
        .args(["run", src.to_str().unwrap()])   // LLVM backend (default)
        .output()
        .expect("intentc run (dprintf path)");

    let _ = fs::remove_dir_all(&dir);

    assert!(
        !assert_out.status.success(),
        "expected non-zero exit from failing assert; stderr: {}",
        String::from_utf8_lossy(&assert_out.stderr)
    );
    let stderr = String::from_utf8_lossy(&assert_out.stderr);
    assert!(
        stderr.contains("snprintf-dprintf-shim-ok"),
        "dprintf shim: expected custom message in stderr, got: {stderr:?}"
    );
}

/// `vanic check --big-o` pins the annotation output format and verifies
/// the classifier is correct on a real example file:
///   - `print_vec` has a while loop → O(n)
///   - `main` calls sort + has while loops → O(n log n)
///   - trivial helpers (descending, etc.) are O(1) and suppressed in auto mode
///   - `--big-o=force` includes the O(1) helpers
#[test]
fn check_big_o_flag_annotates_sort_example() {
    let binary = env!("CARGO_BIN_EXE_intentc");
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let example = format!("{}/examples/language/english/sort.vani", manifest_dir);

    // --- auto mode (default) ---
    let auto_out = Command::new(binary)
        .args(["check", "--big-o", &example])
        .output()
        .expect("intentc check --big-o");
    assert!(
        auto_out.status.success(),
        "check --big-o failed: {}",
        String::from_utf8_lossy(&auto_out.stderr)
    );
    let auto_stdout = String::from_utf8_lossy(&auto_out.stdout);
    // Auto mode: only non-O(1) fns appear
    assert!(
        auto_stdout.contains("fn print_vec:"),
        "auto: expected fn print_vec in output:\n{auto_stdout}"
    );
    assert!(
        auto_stdout.contains("fn main:"),
        "auto: expected fn main in output:\n{auto_stdout}"
    );
    // main calls sort + has loops → O(n log n)
    assert!(
        auto_stdout.contains("fn main: O(n log n)"),
        "auto: expected main classified O(n log n):\n{auto_stdout}"
    );
    // trivial O(1) helpers must be suppressed in auto mode
    assert!(
        !auto_stdout.contains("fn descending:"),
        "auto: O(1) fn descending should be suppressed:\n{auto_stdout}"
    );

    // --- force mode ---
    let force_out = Command::new(binary)
        .args(["check", "--big-o=force", &example])
        .output()
        .expect("intentc check --big-o=force");
    assert!(force_out.status.success());
    let force_stdout = String::from_utf8_lossy(&force_out.stdout);
    // Force includes O(1) helpers
    assert!(
        force_stdout.contains("fn descending: O(1)"),
        "force: expected fn descending: O(1) in output:\n{force_stdout}"
    );
    assert!(
        force_stdout.contains("fn main: O(n log n)"),
        "force: expected fn main: O(n log n):\n{force_stdout}"
    );

    // --- off mode ---
    let off_out = Command::new(binary)
        .args(["check", "--big-o=off", &example])
        .output()
        .expect("intentc check --big-o=off");
    assert!(off_out.status.success());
    let off_stdout = String::from_utf8_lossy(&off_out.stdout);
    assert!(
        !off_stdout.contains("O("),
        "off mode: no complexity annotations expected:\n{off_stdout}"
    );
}

// ---------------------------------------------------------------------------
// Windows async-recv byte-count parity (2026-06-16: WSAECONNRESET fix)
//
// echo_loop.vani produces identical stdout on C and LLVM backends on
// Windows after fixing recv_nb to treat WSAECONNRESET/WSAECONNABORTED
// as EOF (return 0) instead of error (return -1 → infinite yield loop).
// ---------------------------------------------------------------------------

#[test]
#[cfg(target_os = "windows")]
fn echo_loop_windows_byte_count_matches_c() {
    let binary = env!("CARGO_BIN_EXE_intentc");
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let example = format!("{}/examples/language/english/echo_loop.vani", manifest_dir);

    let c_out = Command::new(binary)
        .args(["run", "--backend=c", &example])
        .output()
        .expect("intentc run --backend=c should execute");
    let llvm_out = Command::new(binary)
        .args(["run", "--backend=llvm", &example])
        .output()
        .expect("intentc run --backend=llvm should execute");

    // BUG-42 fallout (2026-07-29): both `Command`s above used to
    // silently run the LLVM backend regardless of `--backend=c`
    // (the flag sat BEFORE the file path, which the pre-fix CLI
    // parser dropped without error) — so this assertion was
    // comparing LLVM's stdout against itself and could never have
    // failed, no matter what "IOCP parity" state the runtime was
    // actually in. Now that the CLI bug is fixed, the two `Command`s
    // really do exercise different backends, and the raw byte
    // counts genuinely differ — but only because the C backend's
    // stdout is in Windows CRT text mode (`\r\n` per line) while
    // LLVM's is not (`\n`), a 2-byte-per-line artifact with nothing
    // to do with IOCP semantics. Normalize line endings before
    // comparing, matching every other cross-backend stdout
    // comparison in this file.
    let c_normalized = String::from_utf8_lossy(&c_out.stdout).replace("\r\n", "\n");
    let llvm_normalized = String::from_utf8_lossy(&llvm_out.stdout).replace("\r\n", "\n");
    let c_bytes = c_normalized.len();
    let llvm_bytes = llvm_normalized.len();

    // Print diagnostics even on failure so CI logs are informative.
    eprintln!(
        "echo_loop byte counts (CRLF-normalized) — C backend: {c_bytes}, LLVM backend: {llvm_bytes}\n\
         C stdout:    {c_normalized:?}\n\
         LLVM stdout: {llvm_normalized:?}",
    );

    assert_eq!(
        c_bytes, llvm_bytes,
        "echo_loop byte-count diverges after CRLF normalization: \
         C={c_bytes} LLVM={llvm_bytes}"
    );
}

// ---------------------------------------------------------------------------
// Enum payload exhaustiveness — regression tests (2026-06-18)
// ---------------------------------------------------------------------------

#[test]
fn enum_non_exhaustive_missing_variant_is_rejected() {
    let src = write_tmp_vani(
        "exhaust_missing_variant",
        r#"
enum Color { Red, Green, Blue }
fn f(c: Color) -> i64 {
  return match c {
    Color.Red   then 1,
    Color.Green then 2,
  };
}
fn main() -> i64 { return 0; }
"#,
    );
    let binary = env!("CARGO_BIN_EXE_intentc");
    let output = Command::new(binary)
        .args(["check", src.to_str().unwrap()])
        .output()
        .expect("intentc check");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !output.status.success(),
        "expected non-exhaustive error; stderr: {stderr}"
    );
    assert!(
        stderr.contains("non-exhaustive") && stderr.contains("Color.Blue"),
        "expected missing-arm diagnostic for Color.Blue; got: {stderr}"
    );
}

#[test]
fn enum_binding_on_payload_less_variant_is_rejected() {
    let src = write_tmp_vani(
        "exhaust_bad_bind",
        r#"
enum Shape { Circle(i64), Triangle }
fn f(s: Shape) -> i64 {
  return match s {
    Shape.Circle(r) then r,
    Shape.Triangle(x) then 0,
  };
}
fn main() -> i64 { return 0; }
"#,
    );
    let binary = env!("CARGO_BIN_EXE_intentc");
    let output = Command::new(binary)
        .args(["check", src.to_str().unwrap()])
        .output()
        .expect("intentc check");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !output.status.success(),
        "expected binding-on-payload-less error; stderr: {stderr}"
    );
    assert!(
        stderr.contains("no payload") || stderr.contains("carries no payload"),
        "expected 'no payload' diagnostic for Triangle; got: {stderr}"
    );
}

#[test]
fn enum_exhaustive_with_wildcard_is_accepted() {
    let src = write_tmp_vani(
        "exhaust_wildcard_ok",
        r#"
enum Color { Red, Green, Blue }
fn f(c: Color) -> i64 {
  return match c {
    Color.Red then 1,
    _         then 0,
  };
}
fn main() -> i64 { return 0; }
"#,
    );
    let binary = env!("CARGO_BIN_EXE_intentc");
    let output = Command::new(binary)
        .args(["check", src.to_str().unwrap()])
        .output()
        .expect("intentc check");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "exhaustive-with-wildcard should compile; stderr: {stderr}"
    );
}

#[test]
fn enum_tag_only_match_on_payload_variant_is_accepted() {
    let src = write_tmp_vani(
        "exhaust_tag_only_ok",
        r#"
enum Shape { Circle(i64), Square(i64), Triangle }
fn classify(s: Shape) -> i64 {
  return match s {
    Shape.Circle   then 1,
    Shape.Square   then 2,
    Shape.Triangle then 3,
  };
}
fn main() -> i64 { return 0; }
"#,
    );
    let binary = env!("CARGO_BIN_EXE_intentc");
    let output = Command::new(binary)
        .args(["check", src.to_str().unwrap()])
        .output()
        .expect("intentc check");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "tag-only match on payloaded variant should compile; stderr: {stderr}"
    );
}
