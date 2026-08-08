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

// Originally C-backend-only: at the time BUG-31 was fixed, the LLVM
// backend's counterpart of this bug (self-referential struct owning a
// Vec<Self>) was still unroot-caused and the example's own header
// comment steered users to `--backend=c`. Re-verified on 2026-08-05
// while investigating localfuzz's `docs/LOCALFUZZ_HANDOFF_2026-08-05.md`
// section 3 items: the LLVM side now works correctly (both `vanic run`,
// which uses lli, and `vanic build`, which AOT-compiles to a native
// binary) -- most likely fixed as a side effect of BUG-108/109/110's
// extensive Vec-related `backend_llvm.rs`/`ssa_backend_llvm.rs` changes
// rather than by any change targeting this specific bug. Extended to
// cover both backends to lock that in as a regression guard.
#[test]
fn self_referential_struct_vec_example_produces_correct_output_on_both_backends() {
    let binary = env!("CARGO_BIN_EXE_intentc");
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let example = format!(
        "{}/examples/language/english/self_referential_struct_vec.vani",
        manifest_dir
    );
    let expected = "1\n2\n3\n";

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
        let stdout = String::from_utf8_lossy(&output.stdout).replace("\r\n", "\n");
        assert_eq!(
            stdout, expected,
            "self-referential struct (Vec<Self> field) tree walk produced the wrong result for {:?}",
            backend_args
        );
    }
}

// BUG-112 (2026-08-05): `vanic build` (LLVM AOT native-binary compile)
// omitted `-lm` from its host-POSIX link command -- every vāṇी
// program's runtime unconditionally emits math-builtin helper
// functions (`intent_f64_normal_pdf`/`_cdf`, `intent_f64_wrap`, etc.)
// that reference libm symbols (`exp`/`erf`/`fmod`/...) whether or not
// the program actually calls them, so `cc`'s link step failed with
// "undefined reference to 'exp'" (etc.) on any host where `cc` doesn't
// already implicitly pull in libm, for literally any program -- this
// exact example included, discovered while re-verifying its LLVM-
// backend `vanic build` path above. `vanic run` (LLVM via `lli`) was
// unaffected (`lli` auto-resolves libc/libm symbols itself), which is
// why this stayed hidden despite `run` being the more common path.
// This is a real subprocess `vanic build` + execute-the-linked-binary
// test, not a string check, because the bug is specifically a LINKER
// failure that a `compile_to_llvm`/`emit` string assertion wouldn't
// exercise at all.
#[test]
fn vanic_build_links_self_referential_struct_vec_example_without_manual_lm_flag() {
    use std::fs;
    let binary = env!("CARGO_BIN_EXE_intentc");
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let example = format!(
        "{}/examples/language/english/self_referential_struct_vec.vani",
        manifest_dir
    );
    let dir = std::env::temp_dir().join(format!(
        "intentc-bug112-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir_all(&dir).expect("mkdir");
    let out_bin = dir.join("node_bin");

    let build_output = Command::new(binary)
        .args(["build", &example, "-o", out_bin.to_str().unwrap()])
        .output()
        .unwrap_or_else(|e| panic!("intentc build {} should execute: {e}", example));
    assert!(
        build_output.status.success(),
        "intentc build {} failed (no manual -lm passed) with status {:?}\nstderr: {}",
        example,
        build_output.status,
        String::from_utf8_lossy(&build_output.stderr)
    );

    let run_output = Command::new(&out_bin)
        .output()
        .unwrap_or_else(|e| panic!("built binary {:?} should execute: {e}", out_bin));
    assert!(
        run_output.status.success(),
        "built binary {:?} exited with status {:?}\nstderr: {}",
        out_bin,
        run_output.status,
        String::from_utf8_lossy(&run_output.stderr)
    );
    let stdout = String::from_utf8_lossy(&run_output.stdout).replace("\r\n", "\n");
    assert_eq!(
        stdout, "1\n2\n3\n",
        "built binary produced the wrong output"
    );
    let _ = fs::remove_dir_all(&dir);
}

// BUG-124/BUG-125 (2026-08-06), found auditing category H's own
// prediction ("run an actual `vanic build` ... on every reachable
// cell, not just JIT/interpret it"). `is_bare_metal_triple`'s
// substring heuristics (`"none"` / `"eabi"` / `"-elf"`) misclassified
// any real Linux ARM EABI target (`arm-unknown-linux-gnueabi`,
// `*-gnueabihf`, ... the Debian armel/armhf family, e.g. Raspberry Pi
// OS 32-bit) as bare-metal, since "eabi" also appears in those
// triples' ABI suffix despite them having a full OS + libc. With a
// real `arm-linux-gnueabi-gcc` + sysroot cross-toolchain, `vanic
// build --target=arm-unknown-linux-gnueabi` on ANY program failed
// with `undefined reference to 'exp'`/`'erf'`/`'fmod'` (BUG-112's
// exact class -- the bare-metal branch adds no `-lm` at all).
// Separately (BUG-125), `src/sort_runtime.c` (embedded into `vanic`
// via `include_str!`, linked into every LLVM-backend binary)
// unconditionally required `#pragma GCC target("avx512f...")` +
// `<immintrin.h>` regardless of target architecture -- on this same
// ARM cross-toolchain that failed to compile outright ("unknown
// target attribute 'avx512f'"), degrading to a non-fatal warning
// that still produced a binary, but with `intent_vec_i64__sort`
// undefined -- so ANY program actually calling `sort`/`sort_by`
// failed to LINK on a non-x86 target. Both confirmed against a real
// `arm-linux-gnueabi-gcc` + `libc6-dev-armel-cross` sysroot. Gated on
// that toolchain being present (not installed in this repo's own CI
// image, which only cross-installs `aarch64-linux-gnu-gcc` for a
// SEPARATE lib-only QEMU job) -- skips gracefully otherwise, same
// pattern as the `lli_available()`-gated LLVM tests.
fn arm_gnueabi_cross_gcc_available() -> bool {
    Command::new("arm-linux-gnueabi-gcc")
        .arg("--version")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

#[test]
fn vanic_build_cross_compiles_math_and_sort_program_for_real_arm_linux_target() {
    use std::fs;
    if !arm_gnueabi_cross_gcc_available() {
        return;
    }
    let binary = env!("CARGO_BIN_EXE_intentc");
    let dir = std::env::temp_dir().join(format!(
        "intentc-bug124-125-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir_all(&dir).expect("mkdir");
    let src_path = dir.join("math_and_sort.vani");
    fs::write(
        &src_path,
        r#"
            fn main() -> i64 {
              let m: f64 = exp(1.0);
              let xs: Vec<i64> = vec(3, 1, 2);
              let _ = sort(mut ref xs);
              print m;
              print xs[0];
              return 0;
            }
        "#,
    )
    .expect("write source");
    let out_bin = dir.join("arm_bin");

    let build_output = Command::new(binary)
        .args([
            "build",
            src_path.to_str().unwrap(),
            "--target=arm-unknown-linux-gnueabi",
            "-o",
            out_bin.to_str().unwrap(),
        ])
        .output()
        .unwrap_or_else(|e| panic!("intentc build --target=arm-unknown-linux-gnueabi should execute: {e}"));
    assert!(
        build_output.status.success(),
        "cross-build for arm-unknown-linux-gnueabi (math + sort) failed with status {:?}\nstderr: {}",
        build_output.status,
        String::from_utf8_lossy(&build_output.stderr)
    );
    assert!(
        !String::from_utf8_lossy(&build_output.stderr).contains("sort runtime compilation failed"),
        "sort_runtime.c must compile cleanly on a non-x86 cross target (BUG-125), stderr:\n{}",
        String::from_utf8_lossy(&build_output.stderr)
    );
    // Confirm a real ARM ELF binary was produced (can't execute it here
    // without a qemu-user emulator, but the LINK succeeding -- with
    // both libm-referencing math helpers AND intent_vec_i64__sort
    // actually resolving -- is exactly the failure mode BUG-124/125 had).
    let bytes = fs::read(&out_bin).expect("read output binary");
    assert_eq!(&bytes[0..4], b"\x7fELF", "expected a real ELF binary");
    assert_eq!(bytes[4], 1, "expected ELFCLASS32 (32-bit ARM)");
    let e_machine = u16::from_le_bytes([bytes[18], bytes[19]]);
    assert_eq!(e_machine, 40, "expected EM_ARM (40) in the ELF header");

    let _ = fs::remove_dir_all(&dir);
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

// Found auditing tutorials/src/advanced/02b_barrier_primer.md (2026-08-01):
// the LLVM backend's barrier_wait "last thread" wake path emitted a raw
// hex integer literal (`i32 0x7fffffff`) directly into the generated
// LLVM IR text for a @syscall FUTEX_WAKE argument -- invalid IR syntax
// for an integer constant (hex is float-only in LLVM's textual IR), so
// `lli` rejected it on 100% of programs that ever reach the last-thread
// branch, i.e. every real use of Barrier (some thread is always last).
// A compile-only check (src/lib.rs's IR-text assertion) can confirm the
// bad literal is gone, but only a real `lli`-executed multi-thread
// rendezvous proves the fix actually WORKS at runtime, not just parses --
// this mirrors two threads racing to a barrier, both proceeding past it,
// and the barrier's own "is_last" signal firing exactly once.
#[test]
fn barrier_two_threads_rendezvous_correctly_on_both_backends() {
    let src = write_tmp_vani(
        "barrier_two_thread_rendezvous",
        r#"
fn phase_one(id: i64, b: mut ref Barrier) -> i64 {
  let is_last: bool = barrier_wait(b);
  if is_last { return 1; }
  return 0;
}

fn main() -> i64 {
  let b: Barrier = barrier_new(2);
  let t1: Task<i64> = task phase_one(1, mut ref b);
  let last_count: i64 = phase_one(2, mut ref b);
  let t1_result: i64 = join t1;
  print "sum of is_last flags (must be exactly 1):", last_count + t1_result;
  return 0;
}
"#,
    );
    let binary = env!("CARGO_BIN_EXE_intentc");
    let expected = "sum of is_last flags (must be exactly 1): 1\n";

    for backend_args in [
        vec!["run", src.to_str().unwrap()],
        vec!["run", src.to_str().unwrap(), "--backend=c"],
    ] {
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
            "barrier rendezvous produced the wrong is_last accounting for {:?} -- \
             exactly one of the two threads must see is_last == true",
            backend_args
        );
    }
}

// Found auditing tutorials/src/advanced/02_parallel.md (2026-08-01): the
// LLVM backend crashed on ANY program with two different functions each
// containing their own `parallel for` (or block-form `task { ... }`, or
// `task fn(args)` spawn) -- the outlined-function id counter restarts at
// 0 per top-level function, so both functions generated the identical
// LLVM symbol `@__intent_par_0`, an "invalid redefinition of function"
// error on 100% of such programs (the tutorial's own double_all/
// dot_product pair, side by side in one file, hit this immediately). A
// compile-only check (src/lib.rs's IR-text assertion) proves the names
// no longer collide; only a real `lli`-executed run proves the fix
// doesn't just avoid the crash but produces the CORRECT values from
// each independently-outlined function.
#[test]
fn two_functions_each_with_parallel_for_run_correctly_on_both_backends() {
    let src = write_tmp_vani(
        "two_parallel_for_fns",
        r#"
fn double_all(xs: mut ref Vec<i64>) -> i64 {
  let n: u64 = len(xs);
  parallel for i from 0 to n {
    xs[i] = xs[i] * 2;
  }
  return 0;
}

fn triple_all(xs: mut ref Vec<i64>) -> i64 {
  let n: u64 = len(xs);
  parallel for i from 0 to n {
    xs[i] = xs[i] * 3;
  }
  return 0;
}

fn main() -> i64 {
  let xs: Vec<i64> = vec(1, 2, 3, 4);
  let _ = double_all(mut ref xs);
  let _ = triple_all(mut ref xs);
  print "xs =", xs[0], xs[1], xs[2], xs[3];
  return 0;
}
"#,
    );
    let binary = env!("CARGO_BIN_EXE_intentc");
    let expected = "xs = 6 12 18 24\n";

    for backend_args in [
        vec!["run", src.to_str().unwrap()],
        vec!["run", src.to_str().unwrap(), "--backend=c"],
    ] {
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
            "two functions each with their own parallel-for produced the wrong result for {:?}",
            backend_args
        );
    }
}

// Found auditing tutorials/src/advanced/03b_condvar_primer.md (2026-08-01):
// emit_intent_condvar_helpers_c's hardcoded C text for condvar_wait/
// condvar_wait_timeout referenced the stale type name intent_guard_i64
// (only defined via a legacy-alias code path the tree-C driver never
// actually calls) instead of the real intent_guard_int64_t that
// emit_mutex_bundle generates for Mutex<i64>/Guard<i64> -- any program
// calling condvar_wait/condvar_wait_timeout (not just condvar_notify_*)
// failed a REAL cc compile with "unknown type name 'intent_guard_i64'".
// A substring check on emitted C text can prove the identifier got
// renamed but can't prove cc actually accepts the result -- only an
// actual --backend=c run (through a real cc invocation) does.
#[test]
fn condvar_wait_and_wait_timeout_compile_and_run_with_real_cc() {
    let src = write_tmp_vani(
        "condvar_wait_real_cc",
        r#"
fn main() -> i64 {
  let cv: Condvar = condvar_new();
  let mx: Mutex<i64> = mutex_new(0);
  {
    let g: Guard<i64> = mutex_lock(ref mx);
    let signaled: bool = condvar_wait_timeout(ref cv, mut ref g, 10);
    if signaled {
      print "unexpected: signaled with no notifier";
    } else {
      print "wait_timeout returned false as expected";
    }
  }
  return 0;
}
"#,
    );
    let binary = env!("CARGO_BIN_EXE_intentc");
    let output = Command::new(binary)
        .args(["run", src.to_str().unwrap(), "--backend=c"])
        .output()
        .expect("intentc run --backend=c should execute");
    assert!(
        output.status.success(),
        "condvar_wait_timeout must compile and run via a real cc invocation on the C backend; stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_eq!(
        stdout.replace("\r\n", "\n"),
        "wait_timeout returned false as expected\n",
        "condvar_wait_timeout with no notifier must time out and return false"
    );
}

// BUG-54, found auditing tutorials/src/advanced/04b_cross_compile_primer.md
// (2026-08-01): the SSA LLVM backend's print-argument widening always used
// `sext` regardless of signedness -- printing an unsigned narrow type
// (u8/u16) whose high bit was set sign-extended into a negative i64. The C
// backend was unaffected, so this was a real backend-parity break: the
// SAME program printed different numbers depending on --backend. A
// compile-only IR-text check (src/lib.rs) can confirm the instruction
// changed from sext to zext, but only a REAL side-by-side execution proves
// the two backends now agree on the actual printed value.
#[test]
fn unsigned_narrow_int_prints_same_value_on_both_backends() {
    let src = write_tmp_vani(
        "unsigned_narrow_print_parity",
        r#"
fn main() -> i64 {
  let a: u8 = 200;
  let b: u8 = 50;
  let c: u8 = a + b;
  print "u8:", c;

  let g: u16 = 60000;
  let h: u16 = 5000;
  let k: u16 = g + h;
  print "u16:", k;
  return 0;
}
"#,
    );
    let binary = env!("CARGO_BIN_EXE_intentc");
    let expected = "u8: 250\nu16: 65000\n";

    for backend_args in [
        vec!["run", src.to_str().unwrap()],
        vec!["run", src.to_str().unwrap(), "--backend=c"],
    ] {
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
            "unsigned narrow-int printing must match between backends for {:?} -- \
             a negative number here means the value was sign-extended instead of zero-extended",
            backend_args
        );
    }
}

// Found auditing tutorials/src/advanced/05_simd.md (2026-08-01): two
// bugs, both C-backend-only. (1) vec_fill/vec_with_capacity's tree-C
// codegen computed the Vec bundle's struct name via an LLVM-backend
// naming helper by mistake, producing a stale name a real cc rejects
// (src/lib.rs's compile_to_c-based unit tests cover this half). (2)
// vec_with_capacity is implemented in SSA-LLVM but was never ported to
// SSA-C, and nothing routed it to the tree-C fallback -- SSA-C fell
// through to an ordinary (nonexistent) function call. Since
// compile_to_c always calls the tree backend directly, only a real CLI
// invocation (which goes through main.rs's SSA-first dispatch) can
// prove bug (2) is actually fixed.
#[test]
fn vec_with_capacity_compiles_and_runs_on_both_backends() {
    let src = write_tmp_vani(
        "vec_with_capacity_real_cc",
        r#"
fn main() -> i64 {
  let a: Vec<i64> = vec_with_capacity(4);
  let _ = push(mut ref a, 10);
  let _ = push(mut ref a, 20);
  print "len:", len(a);
  return 0;
}
"#,
    );
    let binary = env!("CARGO_BIN_EXE_intentc");
    let expected = "len: 2\n";

    for backend_args in [
        vec!["run", src.to_str().unwrap()],
        vec!["run", src.to_str().unwrap(), "--backend=c"],
    ] {
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
            "vec_with_capacity produced the wrong result for {:?}",
            backend_args
        );
    }
}

// BUG-58/BUG-59, found auditing tutorials/src/advanced/05_simd.md
// (2026-08-01): the chapter's own SAXPY and dot256/dot512 examples
// crashed for real, on both backends in SAXPY's case. (58) simd_store's
// discarded return value aliases the caller's own Vec buffer, not a
// fresh allocation -- both backends' generic Vec-discard-free codegen
// freed it anyway, a double-free the moment the caller's own Vec is
// later dropped too (compile-only checks in src/lib.rs cover this half
// via IR-text assertions; only a real execution proves the process
// doesn't actually crash). (59) simd256_load/store and simd512_load/
// store declared `align 32`/`align 64` in LLVM IR, but glibc's malloc
// on x86-64 only guarantees 16-byte alignment -- undefined behavior
// that manifested as a NON-DETERMINISTIC lli crash (same input,
// re-run several times, intermittently aborted depending on the
// buffer's actual runtime alignment). A single passing run proves
// nothing for a non-deterministic bug; this test runs the vec256 case
// repeatedly to get real confidence the fix holds.
#[test]
fn saxpy_f32_example_runs_without_double_free_on_both_backends() {
    let src = write_tmp_vani(
        "saxpy_f32_no_double_free",
        r#"
fn saxpy_f32(y: ref Vec<f32>, x: ref Vec<f32>, alpha: f32, n: i64) -> i64 {
    let splat_alpha: vec128<f32> = simd_splat(alpha);
    let i: i64 = 0;
    while i + 4 <= n {
        let xi: vec128<f32> = simd_load(x, i);
        let yi: vec128<f32> = simd_load(y, i);
        let ax: vec128<f32> = simd_mul(splat_alpha, xi);
        let res: vec128<f32> = simd_add(yi, ax);
        let _ = simd_store(y, i, res);
        i = i + 4;
    }
    return 0;
}

fn main() -> i64 {
    let n: i64 = 8;
    let x: Vec<f32> = vec_fill(n, 2.0 as f32);
    let y: Vec<f32> = vec_fill(n, 1.0 as f32);
    let _ = saxpy_f32(ref y, ref x, 3.0 as f32, n);
    print "y0:", y[0];
    return 0;
}
"#,
    );
    let binary = env!("CARGO_BIN_EXE_intentc");
    let expected = "y0: 7\n";

    for backend_args in [
        vec!["run", src.to_str().unwrap()],
        vec!["run", src.to_str().unwrap(), "--backend=c"],
    ] {
        let output = Command::new(binary)
            .args(&backend_args)
            .output()
            .unwrap_or_else(|e| panic!("intentc {:?} should execute: {e}", backend_args));
        assert!(
            output.status.success(),
            "intentc {:?} failed with status {:?} (a crash here likely means the \
             simd_store double-free regressed)\nstderr: {}",
            backend_args,
            output.status,
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert_eq!(
            stdout.replace("\r\n", "\n"),
            expected,
            "saxpy_f32 produced the wrong result for {:?}",
            backend_args
        );
    }
}

// BUG-47, found in an earlier tutorial audit and fixed here (2026-08-01):
// `format_declarator` (backend_c.rs) had no `Type::HashMap` arm in its
// bare/`Ref`/`RefMut` matches, so every `HashMap<K, V>` FUNCTION PARAMETER
// (by value, `ref`, or `mut ref`) fell through to the hardcoded
// `intent_hashmap_i64_i64` fallback regardless of its real K/V -- a real
// `cc` compile error whenever the parameter's declared type didn't match
// the type actually used in the function body (which always uses the
// correct per-(K,V) name via a completely separate, already-correct code
// path). Only a real `cc` invocation proves the fix -- a
// `compile_to_c` string-contains check (also added, in src/lib.rs) can't
// tell you the file as a whole actually compiles.
// BUG-38, found in an earlier tutorial audit and fixed here
// (2026-08-01): `clone_at()` on a `Vec<Box<T>>` element reached
// codegen with no checker-time rejection. Confirmed by testing:
// tree-LLVM panicked the COMPILER ITSELF (`internal error: entered
// unreachable code`), and the C backend was worse -- it compiled
// clean and then silently double-freed at runtime (`free(): double
// free detected in tcache 2`). Only a real CLI invocation on both
// backends can prove neither failure mode still happens -- a
// compile-only test can't observe a runtime double-free or a Rust
// panic backtrace.
#[test]
fn clone_at_on_vec_of_box_is_rejected_cleanly_not_a_panic_or_double_free() {
    let src = write_tmp_vani(
        "clone_at_vec_of_box_clean_rejection",
        r#"
fn main() -> i64 {
  let b1: Box<i64> = box(1);
  let xs: Vec<Box<i64>> = vec(b1);
  let c: Box<i64> = clone_at(xs, 0);
  return 0;
}
"#,
    );
    let binary = env!("CARGO_BIN_EXE_intentc");
    for backend_args in [
        vec!["run", src.to_str().unwrap()],
        vec!["run", src.to_str().unwrap(), "--backend=c"],
    ] {
        let output = Command::new(binary)
            .args(&backend_args)
            .output()
            .unwrap_or_else(|e| panic!("intentc {:?} should execute: {e}", backend_args));
        assert!(
            !output.status.success(),
            "clone_at(Vec<Box<i64>>, 0) must be rejected at compile time, not silently accepted; {:?}",
            backend_args
        );
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            stderr.contains("clone_at() does not support element type"),
            "expected the clean clone_at rejection diagnostic for {:?}, got stderr: {}",
            backend_args,
            stderr
        );
        assert!(
            !stderr.contains("panicked") && !stderr.contains("unreachable code"),
            "must not be a Rust panic/ICE for {:?}, got stderr: {}",
            backend_args,
            stderr
        );
        assert!(
            !stderr.contains("double free") && !output.status.code().map(|c| c < 0).unwrap_or(false),
            "must not reach codegen and double-free for {:?}, got stderr: {}",
            backend_args,
            stderr
        );
    }
}

// BUG-46, found in an earlier tutorial audit and fixed here
// (2026-08-01): with 2+ instantiations of a builtin generic enum
// (Option<T> or Result<T,E>) in the same program, EVERY constructor
// call for that enum broke -- not just the "extra" ones. Only a
// real run proves the fix produces the CORRECT runtime values, not
// just that the file compiles.
#[test]
fn result_with_three_instantiations_produces_correct_values_on_both_backends() {
    let src = write_tmp_vani(
        "result_three_instantiations_real_run",
        r#"
struct IoError { code: i64 }
struct ParseError { code: i64 }
struct ConfigError { code: i64 }

fn read_file(ok: bool) -> Result<i64, IoError> {
  if ok { return Result.Ok(42); }
  return Result.Err(IoError { code: 1 });
}
fn parse_value(ok: bool) -> Result<i64, ParseError> {
  if ok { return Result.Ok(7); }
  return Result.Err(ParseError { code: 2 });
}
fn load_config(ok: bool) -> Result<i64, ConfigError> {
  if ok { return Result.Ok(99); }
  return Result.Err(ConfigError { code: 3 });
}
fn main() -> i64 {
  let a: Result<i64, IoError> = read_file(true);
  let b: Result<i64, ParseError> = parse_value(false);
  let c: Result<i64, ConfigError> = load_config(true);
  if let Result.Ok(v) = a { print v; }
  if let Result.Err(e) = b { print e.code; }
  if let Result.Ok(v) = c { print v; }
  return 0;
}
"#,
    );
    let binary = env!("CARGO_BIN_EXE_intentc");
    for backend_args in [
        vec!["run", src.to_str().unwrap()],
        vec!["run", src.to_str().unwrap(), "--backend=c"],
    ] {
        let output = Command::new(binary)
            .args(&backend_args)
            .output()
            .unwrap_or_else(|e| panic!("intentc {:?} should execute: {e}", backend_args));
        assert!(
            output.status.success(),
            "3 differently-parameterized Result<i64, E> constructors must all compile and run \
             for {:?}; stderr: {}",
            backend_args,
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8_lossy(&output.stdout).replace("\r\n", "\n");
        assert_eq!(
            stdout, "42\n2\n99\n",
            "wrong values for {:?} -- expected read_file's Ok(42), parse_value's Err(2), \
             load_config's Ok(99)",
            backend_args
        );
    }
}

// BUG-34, found in an earlier tutorial audit and fixed here
// (2026-08-01): `if let`/`while let` rejected a direct call to a
// builtin-Option<T>-returning function (parse_int, find, ...) as
// their scrutinee, with "enum 'Option__i64' not declared" -- even
// though the identical call worked fine as a `match` scrutinee.
// Only a real run proves both control-flow forms now produce the
// correct value, not just that they compile.
#[test]
fn if_let_and_while_let_with_builtin_option_scrutinee_run_correctly_on_both_backends() {
    let src = write_tmp_vani(
        "if_while_let_option_scrutinee_real_run",
        r#"
fn main() -> i64 {
  if let Option.Some(v) = parse_int("42") {
    print v;
  } else {
    print -1;
  }
  let xs: Vec<i64> = vec(1, 2, 3);
  while let Option.Some(v) = find(ref xs, 2) {
    print v;
    break;
  }
  return 0;
}
"#,
    );
    let binary = env!("CARGO_BIN_EXE_intentc");
    for backend_args in [
        vec!["run", src.to_str().unwrap()],
        vec!["run", src.to_str().unwrap(), "--backend=c"],
    ] {
        let output = Command::new(binary)
            .args(&backend_args)
            .output()
            .unwrap_or_else(|e| panic!("intentc {:?} should execute: {e}", backend_args));
        assert!(
            output.status.success(),
            "if-let/while-let with a builtin Option<T>-returning scrutinee must compile and \
             run for {:?}; stderr: {}",
            backend_args,
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8_lossy(&output.stdout).replace("\r\n", "\n");
        assert_eq!(
            stdout, "42\n1\n",
            "wrong values for {:?} -- expected parse_int(\"42\")'s Some(42) then find's index 1",
            backend_args
        );
    }
}

// BUG-33, found in an earlier tutorial audit and fixed here
// (2026-08-01): `ensures` clauses failed to resolve a `let`-bound
// return value. Only a real run confirms the fix produces the
// correct runtime value on both backends, not just that the
// SMT proof discharges.
#[test]
fn ensures_with_let_bound_return_runs_correctly_on_both_backends() {
    let src = write_tmp_vani(
        "ensures_let_bound_return_real_run",
        r#"
fn double(n: i64) -> i64
requires n >= 0;
ensures _return == n * 2;
{
  let r: i64 = n * 2;
  return r;
}
fn main() -> i64 {
  print double(21);
  return 0;
}
"#,
    );
    let binary = env!("CARGO_BIN_EXE_intentc");
    for backend_args in [
        vec!["run", src.to_str().unwrap()],
        vec!["run", src.to_str().unwrap(), "--backend=c"],
    ] {
        let output = Command::new(binary)
            .args(&backend_args)
            .output()
            .unwrap_or_else(|e| panic!("intentc {:?} should execute: {e}", backend_args));
        assert!(
            output.status.success(),
            "ensures with a let-bound return value must compile and run for {:?}; stderr: {}",
            backend_args,
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8_lossy(&output.stdout).replace("\r\n", "\n");
        assert_eq!(stdout, "42\n", "wrong value for {:?}", backend_args);
    }
}

// BUG-45 (logged 2026-07-30, re-checked 2026-08-01 in the "fix
// documented TODO bugs" pass): reported that a function with a
// heap-owning `OwnedStr` parameter crashed (exit 116, both
// backends) the instant a `try`/`?` in the same function actually
// took its early-return path -- strongly suspected to be a missing
// drop/cleanup for the still-in-scope `OwnedStr` parameter at the
// synthesized early-return branch. Re-tested every bisected shape
// from the original report directly against the real CLI (only a
// real run can observe a runtime crash, unlike a compile-only
// test) -- none reproduce on either backend, confirmed across
// repeated runs. Not chased down to which of the ~14 intervening
// checker/backend commits fixed it; this test exists to lock in
// the now-correct behavior so a future regression is caught.
#[test]
fn owned_str_param_with_propagating_try_does_not_crash() {
    let src = write_tmp_vani(
        "owned_str_param_propagating_try_no_crash",
        r#"
fn maybe_half(x: i64) -> Option<i64> {
  if x % 2 == 0 { return Option.Some(x / 2); }
  return Option.None;
}
fn f(s: OwnedStr, x: i64) -> Option<i64> {
  let a = parse_int(s)?;
  let b = maybe_half(x)?;
  return Option.Some(a + b);
}
fn main() -> i64 {
  let r: Option<i64> = f("5" + "", 3);
  if let Option.Some(v) = r {
    print v;
  } else {
    print -1;
  }
  return 0;
}
"#,
    );
    let binary = env!("CARGO_BIN_EXE_intentc");
    for backend_args in [
        vec!["run", src.to_str().unwrap()],
        vec!["run", src.to_str().unwrap(), "--backend=c"],
    ] {
        for run_idx in 0..5 {
            let output = Command::new(binary)
                .args(&backend_args)
                .output()
                .unwrap_or_else(|e| panic!("intentc {:?} should execute: {e}", backend_args));
            assert!(
                output.status.success(),
                "run {run_idx}: an OwnedStr param + a later propagating try/? must not crash \
                 for {:?}; status {:?}, stderr: {}",
                backend_args,
                output.status,
                String::from_utf8_lossy(&output.stderr)
            );
            let stdout = String::from_utf8_lossy(&output.stdout).replace("\r\n", "\n");
            assert_eq!(
                stdout, "-1\n",
                "run {run_idx}: x=3 is odd, maybe_half's ? must propagate None for {:?}",
                backend_args
            );
        }
    }
}

// BUG-46 follow-up gap, found via tutorials/src/intermediate/
// 10c_error_patterns_primer.md's own worked example once BUG-46
// itself was fixed: `resolve_bare_enum_ctors_in_stmt`'s initial fix
// didn't recurse into `if let`/`while let` bodies, so a `return
// EnumName.Variant(...);` inside an `if let ... else if let ...`
// chain -- the single most common place a union error type
// actually gets constructed -- was still unresolved once 2+
// instantiations of the same generic enum existed. Fixed in the
// same follow-up pass as the payload-less-variant (`Option.None`)
// gap. Only a real run proves the CORRECT runtime values, not just
// that the file compiles.
#[test]
fn enum_constructor_in_if_let_chain_produces_correct_values_on_both_backends() {
    let src = write_tmp_vani(
        "enum_ctor_in_if_let_chain_real_run",
        r#"
struct ConfigError { code: i64 }

fn read_config(ok: bool) -> Result<i64, i64> {
  if ok { return Result.Ok(1); }
  return Result.Err(9);
}
fn parse_value(ok: bool) -> Result<i64, i64> {
  if ok { return Result.Ok(2); }
  return Result.Err(8);
}
fn load(a_ok: bool, b_ok: bool) -> Result<i64, ConfigError> {
  let raw: i64 = 0;
  let step1: Result<i64, i64> = read_config(a_ok);
  if let Result.Ok(v) = step1 {
    raw = v;
  } else if let Result.Err(e) = step1 {
    return Result.Err(ConfigError { code: e });
  }
  let value: i64 = 0;
  let step2: Result<i64, i64> = parse_value(b_ok);
  if let Result.Ok(v) = step2 {
    value = v;
  } else if let Result.Err(e) = step2 {
    return Result.Err(ConfigError { code: e });
  }
  return Result.Ok(raw + value);
}
fn main() -> i64 {
  let ok_outcome: Result<i64, ConfigError> = load(true, true);
  if let Result.Ok(v) = ok_outcome { print v; }

  let err_outcome: Result<i64, ConfigError> = load(false, true);
  if let Result.Err(e) = err_outcome { print e.code; }
  return 0;
}
"#,
    );
    let binary = env!("CARGO_BIN_EXE_intentc");
    for backend_args in [
        vec!["run", src.to_str().unwrap()],
        vec!["run", src.to_str().unwrap(), "--backend=c"],
    ] {
        let output = Command::new(binary)
            .args(&backend_args)
            .output()
            .unwrap_or_else(|e| panic!("intentc {:?} should execute: {e}", backend_args));
        assert!(
            output.status.success(),
            "enum constructor inside an if-let/else-if-let chain, with 2 Result<i64,i64>/\
             Result<i64,ConfigError> instantiations, must compile and run for {:?}; stderr: {}",
            backend_args,
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8_lossy(&output.stdout).replace("\r\n", "\n");
        assert_eq!(
            stdout, "3\n9\n",
            "expected load(true,true)'s Ok(1+2=3) then load(false,true)'s Err(9) for {:?}",
            backend_args
        );
    }
}

#[test]
fn hashmap_owned_str_param_compiles_and_runs_with_real_cc() {
    let src = write_tmp_vani(
        "hashmap_owned_str_param_real_cc",
        r#"
fn lookup(map: ref HashMap<OwnedStr, i64>, key: OwnedStr) -> Option<i64> {
  return hashmap_get(map, key);
}
fn insert_it(map: mut ref HashMap<OwnedStr, i64>, key: OwnedStr, v: i64) -> i64 {
  let _ = hashmap_insert(map, key, v);
  return 0;
}
fn count_ints(map: ref HashMap<i64, i64>) -> i64 {
  return hashmap_len(map);
}
fn main() -> i64 {
  let m: HashMap<OwnedStr, i64> = hashmap_new();
  let _ = insert_it(mut ref m, "a" + "", 1);
  let r: Option<i64> = lookup(ref m, "a" + "");
  if let Option.Some(v) = r { print v; }

  let m2: HashMap<i64, i64> = hashmap_new();
  let _ = hashmap_insert(mut ref m2, 5, 50);
  print count_ints(ref m2);
  return 0;
}
"#,
    );
    let binary = env!("CARGO_BIN_EXE_intentc");
    let output = Command::new(binary)
        .args(["run", src.to_str().unwrap(), "--backend=c"])
        .output()
        .expect("intentc run --backend=c should execute");
    assert!(
        output.status.success(),
        "a HashMap<OwnedStr, i64> ref/mut-ref parameter, alongside a \
         second HashMap<i64, i64> instantiation in the same program, \
         must compile and run via a real cc invocation; stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_eq!(
        stdout.replace("\r\n", "\n"),
        "1\n1\n",
        "expected the OwnedStr-keyed lookup (1) then the i64-keyed len (1)"
    );
}

#[test]
fn vec256_dot_product_runs_consistently_without_alignment_crash() {
    let src = write_tmp_vani(
        "vec256_dot_no_alignment_crash",
        r#"
fn dot256(a: ref Vec<f32>, b: ref Vec<f32>, n: i64) -> f32 {
    let acc: vec256<f32> = simd256_splat(0.0 as f32);
    let i: i64 = 0;
    while i + 8 <= n {
        let ai: vec256<f32> = simd256_load(a, i);
        let bi: vec256<f32> = simd256_load(b, i);
        acc = simd256_add(acc, simd256_mul(ai, bi));
        i = i + 8;
    }
    let s: f32 = simd256_reduce_add(acc);
    while i < n {
        s = s + a[i] * b[i];
        i = i + 1;
    }
    return s;
}

fn main() -> i64 {
    let n: i64 = 8;
    let a: Vec<f32> = vec_fill(n, 1.0 as f32);
    let b: Vec<f32> = vec_fill(n, 2.0 as f32);
    print dot256(ref a, ref b, n);
    return 0;
}
"#,
    );
    let binary = env!("CARGO_BIN_EXE_intentc");
    // LLVM only -- the alignment bug never affected the C backend
    // (its `vector_size` extension has no separate alignment
    // annotation to get wrong).
    for run_idx in 0..12 {
        let output = Command::new(binary)
            .args(["run", src.to_str().unwrap()])
            .output()
            .expect("intentc run should execute");
        assert!(
            output.status.success(),
            "run {run_idx}: vec256 dot product crashed (likely the align-32-on-a-16-byte-\
             guaranteed-buffer bug regressing) -- status {:?}\nstderr: {}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert_eq!(stdout, "16\n", "run {run_idx}: wrong dot256 result");
    }
}

// BUG-61, found running the docs/TESTING_MATRIX_TODO.md priority
// sweep (Channel<T,N> had zero end-to-end coverage): `Vec<Channel<
// T,N>>` accessed via `mut ref chans[i]` crashed the LLVM backend
// with heap corruption (a flat, hardcoded 24-byte-per-element
// malloc size instead of the real 80-byte Channel<i64,4> struct
// size) and failed to even compile under the C backend (the
// generated C referenced the `intent_channel_int64_t_4` struct
// typedef before it was declared). Fixed in both `backend_llvm.rs`
// (real GEP-null sizeof for Channel/Mutex/Guard/RwLock/ReadGuard/
// WriteGuard Vec elements) and `backend_c.rs` (channel/mutex/
// rwlock bundles now emitted before the Vec-bundle loop that
// references them by name).
#[test]
fn vec_of_channel_send_recv_via_mut_ref_index_runs_correctly_on_both_backends() {
    let src = write_tmp_vani(
        "vec_of_channel_mut_ref_index",
        r#"
fn main() -> i64 {
  let ch_a: Channel<i64, 4> = channel_new();
  let ch_b: Channel<i64, 4> = channel_new();
  let chans: Vec<Channel<i64, 4>> = vec(ch_a, ch_b);
  let total: i64 = 0;
  let i: i64 = 0;
  while i < 2 {
    let _ = channel_send(mut ref chans[i], (i + 1) * 10);
    let v: i64 = channel_recv(mut ref chans[i]);
    total = total + v;
    i = i + 1;
  }
  print total;
  return 0;
}
"#,
    );
    let binary = env!("CARGO_BIN_EXE_intentc");
    for backend_args in [
        vec!["run", src.to_str().unwrap()],
        vec!["run", src.to_str().unwrap(), "--backend=c"],
    ] {
        let output = Command::new(binary)
            .args(&backend_args)
            .output()
            .unwrap_or_else(|e| panic!("intentc {:?} should execute: {e}", backend_args));
        assert!(
            output.status.success(),
            "Vec<Channel<i64,4>> mut-ref-index send/recv must not crash for {:?}; \
             status {:?}, stderr: {}",
            backend_args,
            output.status,
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8_lossy(&output.stdout).replace("\r\n", "\n");
        assert_eq!(
            stdout, "30\n",
            "expected 10 + 20 = 30 for {:?}; got: {}",
            backend_args, stdout
        );
    }
}

/// Same shape as above but growing the Vec past its initial
/// capacity via `push` before indexing -- exercises the realloc
/// path, not just the initial `vec()`-literal malloc.
#[test]
fn vec_of_channel_push_growth_then_send_recv_runs_correctly_on_both_backends() {
    let src = write_tmp_vani(
        "vec_of_channel_push_growth",
        r#"
fn main() -> i64 {
  let ch_a: Channel<i64, 4> = channel_new();
  let chans: Vec<Channel<i64, 4>> = vec(ch_a);
  let ch_b: Channel<i64, 4> = channel_new();
  push(mut ref chans, ch_b);
  let ch_c: Channel<i64, 4> = channel_new();
  push(mut ref chans, ch_c);
  let total: i64 = 0;
  let i: i64 = 0;
  while i < 3 {
    let _ = channel_send(mut ref chans[i], (i + 1) * 100);
    let v: i64 = channel_recv(mut ref chans[i]);
    total = total + v;
    i = i + 1;
  }
  print total;
  return 0;
}
"#,
    );
    let binary = env!("CARGO_BIN_EXE_intentc");
    for backend_args in [
        vec!["run", src.to_str().unwrap()],
        vec!["run", src.to_str().unwrap(), "--backend=c"],
    ] {
        let output = Command::new(binary)
            .args(&backend_args)
            .output()
            .unwrap_or_else(|e| panic!("intentc {:?} should execute: {e}", backend_args));
        assert!(
            output.status.success(),
            "Vec<Channel<i64,4>> push-growth then send/recv must not crash for {:?}; \
             status {:?}, stderr: {}",
            backend_args,
            output.status,
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8_lossy(&output.stdout).replace("\r\n", "\n");
        assert_eq!(
            stdout, "600\n",
            "expected 100 + 200 + 300 = 600 for {:?}; got: {}",
            backend_args, stdout
        );
    }
}

/// Minimal SSA-C-path variant of the same bug: a `Vec<Channel<T,N>>`
/// that never uses `mut ref vec[i]` (so it's SSA-eligible on the C
/// side, per `main.rs`'s `ssa_path_supports` -- Channel/Mutex/
/// RwLock/Atomic route through SSA on both backends) still hit the
/// identical typedef-ordering bug in `ssa_backend_c.rs` (a separate
/// copy of the same collect-then-emit logic as tree-C's `emit_c`).
/// Only needs to compile+run without crashing; the interesting
/// assertion is the exit code, not the (unused) value.
#[test]
fn vec_of_channel_construct_only_compiles_via_ssa_c_path() {
    let src = write_tmp_vani(
        "vec_of_channel_ssa_c_construct_only",
        r#"
fn main() -> i64 {
  let ch_a: Channel<i64, 4> = channel_new();
  let ch_b: Channel<i64, 4> = channel_new();
  let chans: Vec<Channel<i64, 4>> = vec(ch_a, ch_b);
  print len(chans);
  return 0;
}
"#,
    );
    let binary = env!("CARGO_BIN_EXE_intentc");
    let output = Command::new(binary)
        .args(["run", src.to_str().unwrap(), "--backend=c"])
        .output()
        .expect("intentc run --backend=c should execute");
    assert!(
        output.status.success(),
        "SSA-C-eligible Vec<Channel<i64,4>> construction must compile+run; \
         status {:?}, stderr: {}",
        output.status,
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout).replace("\r\n", "\n");
    assert_eq!(stdout, "2\n", "expected len(chans) == 2; got: {}", stdout);
}

/// BUG-61 follow-up #1, found sweeping struct-field concurrency-
/// handle + Vec-field combinations for the testing-matrix's new
/// "container x concurrency-handle nesting" section: a struct field
/// of type `Channel<T,N>` sitting alongside a `Vec<T>` field hit
/// the identical typedef-ordering failure as the bare-Vec-of-
/// Channel case (BUG-61 proper), just one level up -- the channel
/// struct wasn't declared before the OWNING struct's own typedef.
#[test]
fn struct_field_channel_alongside_vec_field_runs_correctly_on_both_backends() {
    let src = write_tmp_vani(
        "struct_field_channel_alongside_vec",
        r#"
struct Worker { ch: Channel<i64, 4>, buf: Vec<i64> }
fn main() -> i64 {
  let ch: Channel<i64, 4> = channel_new();
  let buf: Vec<i64> = vec(1, 2, 3);
  let w: Worker = Worker { ch: ch, buf: buf };
  let _ = channel_send(ref w.ch, 55);
  let v: i64 = channel_recv(ref w.ch);
  print v;
  print w.buf[0] + w.buf[1] + w.buf[2];
  return 0;
}
"#,
    );
    let binary = env!("CARGO_BIN_EXE_intentc");
    for backend_args in [
        vec!["run", src.to_str().unwrap()],
        vec!["run", src.to_str().unwrap(), "--backend=c"],
    ] {
        let output = Command::new(binary)
            .args(&backend_args)
            .output()
            .unwrap_or_else(|e| panic!("intentc {:?} should execute: {e}", backend_args));
        assert!(
            output.status.success(),
            "struct {{ Channel field, Vec field }} must not fail to build for {:?}; \
             status {:?}, stderr: {}",
            backend_args,
            output.status,
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8_lossy(&output.stdout).replace("\r\n", "\n");
        assert_eq!(
            stdout, "55\n6\n",
            "expected channel roundtrip 55, then buf sum 1+2+3=6 for {:?}; got: {}",
            backend_args, stdout
        );
    }
}

/// BUG-61 follow-up #2: same shape, but with a `Mutex<T>` field
/// instead of `Channel<T,N>` -- exercises `c_element_storage`'s
/// missing Mutex/Guard/RwLock/ReadGuard/WriteGuard arms specifically
/// (a different code path from the Channel case, since Channel
/// already had a `c_element_storage` arm before this pass -- only
/// its EMISSION ORDER was broken; Mutex/Guard/RwLock had neither
/// the right NAME nor emission order until this fix).
#[test]
fn struct_field_mutex_alongside_vec_field_runs_correctly_on_both_backends() {
    let src = write_tmp_vani(
        "struct_field_mutex_alongside_vec",
        r#"
struct Counter { m: Mutex<i64>, history: Vec<i64> }
fn main() -> i64 {
  let m: Mutex<i64> = mutex_new(10);
  let history: Vec<i64> = vec(1, 2);
  let c: Counter = Counter { m: m, history: history };
  let g: Guard<i64> = mutex_lock(ref c.m);
  print guard_get(ref g);
  print c.history[0] + c.history[1];
  return 0;
}
"#,
    );
    let binary = env!("CARGO_BIN_EXE_intentc");
    for backend_args in [
        vec!["run", src.to_str().unwrap()],
        vec!["run", src.to_str().unwrap(), "--backend=c"],
    ] {
        let output = Command::new(binary)
            .args(&backend_args)
            .output()
            .unwrap_or_else(|e| panic!("intentc {:?} should execute: {e}", backend_args));
        assert!(
            output.status.success(),
            "struct {{ Mutex field, Vec field }} must not fail to build for {:?}; \
             status {:?}, stderr: {}",
            backend_args,
            output.status,
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8_lossy(&output.stdout).replace("\r\n", "\n");
        assert_eq!(
            stdout, "10\n3\n",
            "expected mutex value 10, then history sum 1+2=3 for {:?}; got: {}",
            backend_args, stdout
        );
    }
}

/// Vec<Mutex<T>> / Vec<RwLock<T>> specifically -- BUG-61's fix
/// covered these types in the same size/ordering code paths as
/// Channel, but only Channel itself was run end-to-end to confirm.
/// Closes that gap directly.
#[test]
fn vec_of_mutex_and_vec_of_rwlock_run_correctly_on_both_backends() {
    let src = write_tmp_vani(
        "vec_of_mutex_and_rwlock",
        r#"
fn main() -> i64 {
  let m1: Mutex<i64> = mutex_new(1);
  let m2: Mutex<i64> = mutex_new(2);
  let mutexes: Vec<Mutex<i64>> = vec(m1, m2);
  let g: Guard<i64> = mutex_lock(mut ref mutexes[0]);
  print guard_get(ref g);

  let r1: RwLock<i64> = rwlock_new(100);
  let locks: Vec<RwLock<i64>> = vec(r1);
  let rg = rwlock_read(mut ref locks[0]);
  print read_guard_get(ref rg);
  return 0;
}
"#,
    );
    let binary = env!("CARGO_BIN_EXE_intentc");
    for backend_args in [
        vec!["run", src.to_str().unwrap()],
        vec!["run", src.to_str().unwrap(), "--backend=c"],
    ] {
        let output = Command::new(binary)
            .args(&backend_args)
            .output()
            .unwrap_or_else(|e| panic!("intentc {:?} should execute: {e}", backend_args));
        assert!(
            output.status.success(),
            "Vec<Mutex<i64>> / Vec<RwLock<i64>> must not crash for {:?}; \
             status {:?}, stderr: {}",
            backend_args,
            output.status,
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8_lossy(&output.stdout).replace("\r\n", "\n");
        assert_eq!(
            stdout, "1\n100\n",
            "expected mutexes[0]==1, then locks[0]==100 for {:?}; got: {}",
            backend_args, stdout
        );
    }
}

// BUG-62, found sweeping "multi-level container nesting" for the
// testing-matrix's `Vec<Array<Struct,N>>` row: THREE independent
// bugs, all specific to a Vec whose element is a fixed-size array
// of a non-trivial (Struct) type, assembled from named array
// variables (not inline array literals):
//   1. tree-C's per-shape array typedef leaked a bare `c_leaf_type`
//      placeholder comment ("/* struct */") into the typedef body
//      instead of the real Struct_Point name.
//   2. tree-C's `vec(a1, a2)` literal construction used a plain
//      compound-literal initializer list, which C forbids
//      populating from array-typed EXPRESSIONS (only brace-literal
//      elements are legal there) -- silently producing malformed
//      flattened-field assignments for named array variables.
//   3. tree-LLVM's `vec_element_byte_size` fallback (used whenever
//      `vec_element_size_expr`'s gating didn't recognize the
//      element as needing runtime sizeof) silently under-computed
//      the size of `[Struct;N]` -- 8 bytes/element (its scalar
//      fallback) instead of the real 32, corrupting the heap.
#[test]
fn vec_of_array_of_struct_from_named_variables_runs_correctly_on_both_backends() {
    let src = write_tmp_vani(
        "vec_of_array_of_struct_named_vars",
        r#"
struct Point { x: i64, y: i64 }
fn main() -> i64 {
  let a1: [Point; 2] = [Point { x: 1, y: 1 }, Point { x: 2, y: 2 }];
  let a2: [Point; 2] = [Point { x: 3, y: 3 }, Point { x: 4, y: 4 }];
  let vs: Vec<[Point; 2]> = vec(a1, a2);
  let total: i64 = 0;
  for arr in vs {
    total = total + arr[0].x + arr[0].y + arr[1].x + arr[1].y;
  }
  print total;
  return 0;
}
"#,
    );
    let binary = env!("CARGO_BIN_EXE_intentc");
    for backend_args in [
        vec!["run", src.to_str().unwrap()],
        vec!["run", src.to_str().unwrap(), "--backend=c"],
    ] {
        let output = Command::new(binary)
            .args(&backend_args)
            .output()
            .unwrap_or_else(|e| panic!("intentc {:?} should execute: {e}", backend_args));
        assert!(
            output.status.success(),
            "Vec<[Point;2]> built from named array variables must not fail to \
             build or crash for {:?}; status {:?}, stderr: {}",
            backend_args,
            output.status,
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8_lossy(&output.stdout).replace("\r\n", "\n");
        assert_eq!(
            stdout, "20\n",
            "expected (1+1+2+2) + (3+3+4+4) = 20 for {:?}; got: {}",
            backend_args, stdout
        );
    }
}

// BUG-63, found continuing the "multi-level container nesting" sweep
// item `struct { items: Vec<(i64, OwnedStr)> }`: a Tuple shape that
// ONLY ever appears inside a struct field (never in a function
// signature/body) was never collected into tree-C's `tuple_shapes`
// at all, so its bundle was never emitted anywhere -- while the
// struct-field Vec<Tuple> bundle (wrongly treated as needing no
// deferral, since `vec_element_has_user_struct` didn't recognize
// Tuple) referenced the missing type by name regardless.
#[test]
fn struct_field_vec_of_tuple_runs_correctly_on_both_backends() {
    let src = write_tmp_vani(
        "struct_field_vec_of_tuple",
        r#"
struct Bag { items: Vec<(i64, OwnedStr)> }
fn main() -> i64 {
  let items: Vec<(i64, OwnedStr)> = vec((1, "a" + ""), (2, "b" + ""));
  let bag: Bag = Bag { items: items };
  let total: i64 = 0;
  let i: i64 = 0;
  while i < 2 {
    let pair: (i64, OwnedStr) = clone_at(ref bag.items, i);
    let (num, s) = pair;
    total = total + num;
    print s;
    i = i + 1;
  }
  print total;
  return 0;
}
"#,
    );
    let binary = env!("CARGO_BIN_EXE_intentc");
    for backend_args in [
        vec!["run", src.to_str().unwrap()],
        vec!["run", src.to_str().unwrap(), "--backend=c"],
    ] {
        let output = Command::new(binary)
            .args(&backend_args)
            .output()
            .unwrap_or_else(|e| panic!("intentc {:?} should execute: {e}", backend_args));
        assert!(
            output.status.success(),
            "struct {{ Vec<(i64,OwnedStr)> field }} must not fail to build for {:?}; \
             status {:?}, stderr: {}",
            backend_args,
            output.status,
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8_lossy(&output.stdout).replace("\r\n", "\n");
        assert_eq!(
            stdout, "a\nb\n3\n",
            "expected \"a\", \"b\", then total 1+2=3 for {:?}; got: {}",
            backend_args, stdout
        );
    }
}

// BUG-64, found sweeping "container x concurrency-handle nesting"
// for `Channel<StructWithVecField, N>`: sending a non-Copy struct
// (one owning a Vec field) through a Channel double-freed at
// runtime on both backends -- `channel_send`/`channel_recv` copy
// the payload bytewise with no move-out-of-sender or deep-clone
// machinery, so the sender's original variable AND the received
// value ended up as two independent owners of the same heap
// buffer. Now cleanly rejected at compile time instead.
#[test]
fn channel_of_non_copy_struct_is_rejected_cleanly_on_both_backends() {
    let src = write_tmp_vani(
        "channel_of_non_copy_struct_rejected",
        r#"
struct Msg { id: i64, tags: Vec<i64> }
fn main() -> i64 {
  let ch: Channel<Msg, 4> = channel_new();
  let tags: Vec<i64> = vec(10, 20, 30);
  let m: Msg = Msg { id: 7, tags: tags };
  let _ = channel_send(ref ch, m);
  let got: Msg = channel_recv(ref ch);
  print got.id;
  return 0;
}
"#,
    );
    let binary = env!("CARGO_BIN_EXE_intentc");
    for backend_args in [
        vec!["run", src.to_str().unwrap()],
        vec!["run", src.to_str().unwrap(), "--backend=c"],
    ] {
        let output = Command::new(binary)
            .args(&backend_args)
            .output()
            .unwrap_or_else(|e| panic!("intentc {:?} should execute: {e}", backend_args));
        assert!(
            !output.status.success(),
            "Channel<Msg-with-Vec-field,4> must be rejected at compile time \
             (not crash at runtime with a double-free) for {:?}",
            backend_args
        );
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            stderr.contains("Copy struct/enum"),
            "expected the Channel-element-must-be-Copy diagnostic for {:?}; \
             got stderr: {}",
            backend_args, stderr
        );
        assert!(
            !stderr.contains("double free") && !stderr.contains("free():"),
            "must be a clean diagnostic, not a crashed double-free, for {:?}; \
             got stderr: {}",
            backend_args, stderr
        );
    }
}

// BUG-65: a regression introduced by BUG-63's own fix (caught in the
// same sweep before ever shipping to a release, but still a real
// second bug -- the early-tuple-bundle partitioning didn't account
// for `dyn Iface` tuple elements needing `emit_dyn_iface_typedefs`
// to run first).
#[test]
fn tuple_of_dyn_iface_runs_correctly_on_both_backends() {
    let src = write_tmp_vani(
        "tuple_of_dyn_iface",
        r#"
struct Circle { r: i64 }
interface Shape { fn area(self: Circle) -> i64; }
implement Shape for Circle {
  fn area(self: Circle) -> i64 { return self.r * self.r; }
}
fn main() -> i64 {
  let c: Circle = Circle { r: 5 };
  let d: dyn Shape = c;
  let pair: (dyn Shape, i64) = (d, 99);
  print pair.0.area();
  print pair.1;
  return 0;
}
"#,
    );
    let binary = env!("CARGO_BIN_EXE_intentc");
    for backend_args in [
        vec!["run", src.to_str().unwrap()],
        vec!["run", src.to_str().unwrap(), "--backend=c"],
    ] {
        let output = Command::new(binary)
            .args(&backend_args)
            .output()
            .unwrap_or_else(|e| panic!("intentc {:?} should execute: {e}", backend_args));
        assert!(
            output.status.success(),
            "(dyn Shape, i64) tuple must not fail to build for {:?}; status {:?}, stderr: {}",
            backend_args,
            output.status,
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8_lossy(&output.stdout).replace("\r\n", "\n");
        assert_eq!(
            stdout, "25\n99\n",
            "expected area 5*5=25, then tag 99 for {:?}; got: {}",
            backend_args, stdout
        );
    }
}

// BUG-66, found sweeping "closure capturing a Vec/Channel by move,
// stored in a struct field, called later": a struct field of
// Closure type referenced its typedef before it was declared under
// --backend=c. Fixed by splitting the typedef emission (no
// struct-body dependency) from the trampoline/constructor emission
// (genuinely needs full env-struct bodies) the same way BUG-61/63
// split Channel/Tuple bundle emission. This test covers the
// Copy-only-capture case, which is now fully correct end-to-end;
// see docs/TODO_CURRENT.md's BUG-66 entry for the separate,
// deliberately-deferred heap-capture gap this sweep also found.
#[test]
fn struct_field_closure_with_copy_only_capture_runs_correctly_on_both_backends() {
    let src = write_tmp_vani(
        "struct_field_closure_copy_capture",
        r#"
struct Handler { cb: Closure(i64) -> i64 }
fn main() -> i64 {
  let base: i64 = 100;
  let cb = fn(extra: i64) -> i64 { return base + extra; };
  let h: Handler = Handler { cb: cb };
  let f: Closure(i64) -> i64 = h.cb;
  print f(5);
  return 0;
}
"#,
    );
    let binary = env!("CARGO_BIN_EXE_intentc");
    for backend_args in [
        vec!["run", src.to_str().unwrap()],
        vec!["run", src.to_str().unwrap(), "--backend=c"],
    ] {
        let output = Command::new(binary)
            .args(&backend_args)
            .output()
            .unwrap_or_else(|e| panic!("intentc {:?} should execute: {e}", backend_args));
        assert!(
            output.status.success(),
            "struct {{ Closure field }} with Copy-only capture must not fail to build \
             for {:?}; status {:?}, stderr: {}",
            backend_args,
            output.status,
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8_lossy(&output.stdout).replace("\r\n", "\n");
        assert_eq!(
            stdout, "105\n",
            "expected 100 + 5 = 105 for {:?}; got: {}",
            backend_args, stdout
        );
    }
}

// The remaining rows below are "swept, no bug found" pairings from
// docs/TESTING_MATRIX_TODO.md's nested-combinations sections --
// promoted to permanent regression tests per that file's own
// process note ("a clean pairing still earns a permanent regression
// test, since it's exactly the kind of coverage that was missing
// before BUG-61 was found").

#[test]
fn dyn_iface_struct_field_and_heterogeneous_vec_run_correctly_on_both_backends() {
    let src = write_tmp_vani(
        "dyn_iface_struct_field_and_vec",
        r#"
struct Circle { r: i64 }
struct Square { side: i64 }
struct Triangle { base: i64, height: i64 }
interface Shape { fn area(self: Circle) -> i64; }
interface Named { fn label(self: Circle) -> i64; }
implement Shape for Circle { fn area(self: Circle) -> i64 { return self.r * self.r; } }
implement Shape for Square { fn area(self: Square) -> i64 { return self.side * self.side; } }
implement Shape for Triangle { fn area(self: Triangle) -> i64 { return self.base * self.height / 2; } }
implement Named for Circle { fn label(self: Circle) -> i64 { return 1; } }
implement Named for Square { fn label(self: Square) -> i64 { return 2; } }
struct Widget { shape: dyn Shape, name: dyn Named }
fn area_via_ref(d: ref dyn Shape) -> i64 { return d.area(); }
fn main() -> i64 {
  let c: Circle = Circle { r: 3 };
  let s: Square = Square { side: 5 };
  let w: Widget = Widget { shape: c, name: s };
  print w.shape.area();
  print w.name.label();
  let sref: dyn Shape = c;
  print area_via_ref(ref sref);
  let mixed: Vec<dyn Shape> = vec(Circle { r: 2 }, Square { side: 4 }, Triangle { base: 6, height: 4 });
  let total: i64 = 0;
  for sh in mixed { total = total + sh.area(); }
  print total;
  return 0;
}
"#,
    );
    let binary = env!("CARGO_BIN_EXE_intentc");
    for backend_args in [
        vec!["run", src.to_str().unwrap()],
        vec!["run", src.to_str().unwrap(), "--backend=c"],
    ] {
        let output = Command::new(binary)
            .args(&backend_args)
            .output()
            .unwrap_or_else(|e| panic!("intentc {:?} should execute: {e}", backend_args));
        assert!(output.status.success(), "{:?}: status {:?}, stderr: {}", backend_args, output.status, String::from_utf8_lossy(&output.stderr));
        let stdout = String::from_utf8_lossy(&output.stdout).replace("\r\n", "\n");
        assert_eq!(stdout, "9\n2\n9\n32\n", "for {:?}; got: {}", backend_args, stdout);
    }
}

#[test]
fn fnptr_in_vec_and_struct_field_run_correctly_on_both_backends() {
    let src = write_tmp_vani(
        "fnptr_vec_and_struct_field",
        r#"
fn add1(x: i64) -> i64 { return x + 1; }
fn double(x: i64) -> i64 { return x * 2; }
fn square(x: i64) -> i64 { return x * x; }
struct Op { f: fn(i64) -> i64, label: i64 }
fn apply(op: Op, x: i64) -> i64 {
  let f: fn(i64) -> i64 = op.f;
  return f(x);
}
fn main() -> i64 {
  let fns: Vec<fn(i64) -> i64> = vec(add1, double, square);
  let total: i64 = 0;
  let i: i64 = 0;
  while i < 3 {
    let f: fn(i64) -> i64 = fns[i];
    total = total + f(5);
    i = i + 1;
  }
  let op: Op = Op { f: add1, label: 1 };
  print total;
  print apply(op, 10);
  return 0;
}
"#,
    );
    let binary = env!("CARGO_BIN_EXE_intentc");
    for backend_args in [
        vec!["run", src.to_str().unwrap()],
        vec!["run", src.to_str().unwrap(), "--backend=c"],
    ] {
        let output = Command::new(binary)
            .args(&backend_args)
            .output()
            .unwrap_or_else(|e| panic!("intentc {:?} should execute: {e}", backend_args));
        assert!(output.status.success(), "{:?}: status {:?}, stderr: {}", backend_args, output.status, String::from_utf8_lossy(&output.stderr));
        let stdout = String::from_utf8_lossy(&output.stdout).replace("\r\n", "\n");
        assert_eq!(stdout, "41\n11\n", "for {:?}; got: {}", backend_args, stdout);
    }
}

#[test]
fn vec_of_vec_of_struct_runs_correctly_on_both_backends() {
    let src = write_tmp_vani(
        "vec_of_vec_of_struct",
        r#"
struct Point { x: i64, y: i64 }
fn main() -> i64 {
  let row1: Vec<Point> = vec(Point { x: 1, y: 2 }, Point { x: 3, y: 4 });
  let row2: Vec<Point> = vec(Point { x: 5, y: 6 });
  let grid: Vec<Vec<Point>> = vec(row1, row2);
  let total: i64 = 0;
  let i: i64 = 0;
  while i < 2 {
    let row: Vec<Point> = clone_at(ref grid, i);
    let j: i64 = 0;
    while j < (len(row) as i64) {
      total = total + row[j].x + row[j].y;
      j = j + 1;
    }
    i = i + 1;
  }
  print total;
  return 0;
}
"#,
    );
    let binary = env!("CARGO_BIN_EXE_intentc");
    for backend_args in [
        vec!["run", src.to_str().unwrap()],
        vec!["run", src.to_str().unwrap(), "--backend=c"],
    ] {
        let output = Command::new(binary)
            .args(&backend_args)
            .output()
            .unwrap_or_else(|e| panic!("intentc {:?} should execute: {e}", backend_args));
        assert!(output.status.success(), "{:?}: status {:?}, stderr: {}", backend_args, output.status, String::from_utf8_lossy(&output.stderr));
        let stdout = String::from_utf8_lossy(&output.stdout).replace("\r\n", "\n");
        assert_eq!(stdout, "21\n", "for {:?}; got: {}", backend_args, stdout);
    }
}

#[test]
fn async_fn_returning_struct_and_vec_run_correctly_on_both_backends() {
    let src = write_tmp_vani(
        "async_fn_returning_struct_and_vec",
        r#"
struct Point { x: i64, y: i64 }
async fn make_point(n: i64) -> Point {
  return Point { x: n, y: n * 2 };
}
async fn make_vec(n: i64) -> Vec<i64> {
  return vec(n, n * 2, n * 3);
}
fn main() -> i64 {
  let fp: Future<Point> = make_point(5);
  if let Future.Ready(p) = fp {
    print p.x;
    print p.y;
  }
  let fv: Future<Vec<i64>> = make_vec(5);
  if let Future.Ready(v) = fv {
    print v[0] + v[1] + v[2];
  }
  return 0;
}
"#,
    );
    let binary = env!("CARGO_BIN_EXE_intentc");
    for backend_args in [
        vec!["run", src.to_str().unwrap()],
        vec!["run", src.to_str().unwrap(), "--backend=c"],
    ] {
        let output = Command::new(binary)
            .args(&backend_args)
            .output()
            .unwrap_or_else(|e| panic!("intentc {:?} should execute: {e}", backend_args));
        assert!(output.status.success(), "{:?}: status {:?}, stderr: {}", backend_args, output.status, String::from_utf8_lossy(&output.stderr));
        let stdout = String::from_utf8_lossy(&output.stdout).replace("\r\n", "\n");
        assert_eq!(stdout, "5\n10\n30\n", "for {:?}; got: {}", backend_args, stdout);
    }
}

#[test]
fn barrier_with_vec_of_mutex_shared_state_runs_correctly_on_both_backends() {
    let src = write_tmp_vani(
        "barrier_with_vec_of_mutex",
        r#"
fn worker(id: i64, m: mut ref Mutex<i64>, b: mut ref Barrier) -> i64 {
  let g: Guard<i64> = mutex_lock(m);
  let _ = guard_set(ref g, id * 10);
  let _ = barrier_wait(b);
  return 0;
}
fn main() -> i64 {
  let m0: Mutex<i64> = mutex_new(0);
  let m1: Mutex<i64> = mutex_new(0);
  let m2: Mutex<i64> = mutex_new(0);
  let mutexes: Vec<Mutex<i64>> = vec(m0, m1, m2);
  let b: Barrier = barrier_new(3);
  let t1: Task<i64> = task worker(1, mut ref mutexes[1], mut ref b);
  let t2: Task<i64> = task worker(2, mut ref mutexes[2], mut ref b);
  let _ = worker(0, mut ref mutexes[0], mut ref b);
  let _ = join t1;
  let _ = join t2;
  let total: i64 = 0;
  let i: i64 = 0;
  while i < 3 {
    let g: Guard<i64> = mutex_lock(mut ref mutexes[i]);
    total = total + guard_get(ref g);
    i = i + 1;
  }
  print total;
  return 0;
}
"#,
    );
    let binary = env!("CARGO_BIN_EXE_intentc");
    for backend_args in [
        vec!["run", src.to_str().unwrap()],
        vec!["run", src.to_str().unwrap(), "--backend=c"],
    ] {
        let output = Command::new(binary)
            .args(&backend_args)
            .output()
            .unwrap_or_else(|e| panic!("intentc {:?} should execute: {e}", backend_args));
        assert!(output.status.success(), "{:?}: status {:?}, stderr: {}", backend_args, output.status, String::from_utf8_lossy(&output.stderr));
        let stdout = String::from_utf8_lossy(&output.stdout).replace("\r\n", "\n");
        assert_eq!(stdout, "30\n", "for {:?}; got: {}", backend_args, stdout);
    }
}

// BUG-67, found writing intermediate/06a_closures_primer.md's
// worked "factory returns a closure that captured an OwnedStr"
// example against the real compiler: a factory function returning
// a closure that captured a heap-owning value both freed the
// closure's env AND returned the same now-dangling pointer bundle
// -- a genuine use-after-free/double-free. `consume_if_moved_var`
// never marked a returned Closure variable as moved (Type::Closure
// has no explicit is_copy() arm, so it fell through to that
// function's true-by-default catch-all), so the return path's
// affine-closure-drop pass always fired regardless of whether the
// variable was the thing being returned.
#[test]
fn factory_fn_returning_closure_with_owned_str_capture_runs_correctly_on_both_backends() {
    let src = write_tmp_vani(
        "factory_closure_owned_str_capture",
        r#"
fn make_greeter(name: OwnedStr) -> Closure(i64) -> i64 {
  let g = fn(x: i64) -> i64 { print "hello,", name, x; return 0; };
  return g;
}
fn main() -> i64 {
  let say_hi: Closure(i64) -> i64 = make_greeter("alice" + "");
  say_hi(5);
  return 0;
}
"#,
    );
    let binary = env!("CARGO_BIN_EXE_intentc");
    for backend_args in [
        vec!["run", src.to_str().unwrap()],
        vec!["run", src.to_str().unwrap(), "--backend=c"],
    ] {
        for run_idx in 0..5 {
            let output = Command::new(binary)
                .args(&backend_args)
                .output()
                .unwrap_or_else(|e| panic!("intentc {:?} should execute: {e}", backend_args));
            assert!(
                output.status.success(),
                "run {run_idx}: factory returning an OwnedStr-capturing closure must \
                 not crash (double-free) for {:?}; status {:?}, stderr: {}",
                backend_args,
                output.status,
                String::from_utf8_lossy(&output.stderr)
            );
            let stdout = String::from_utf8_lossy(&output.stdout).replace("\r\n", "\n");
            assert_eq!(
                stdout, "hello, alice 5\n",
                "run {run_idx}: for {:?}; got: {}",
                backend_args, stdout
            );
        }
    }
}

// Testing-matrix sweep, "container x SMT contracts": a `while` loop with
// a (scalar-only) `invariant` clause, whose body mutates a `Vec<Struct>`
// element in place via `mut ref vec[i]`. Vec<Struct> element field access
// itself isn't SMT-modeled (array theory only covers scalar elements --
// confirmed separately as a cleanly-rejected v1 limitation, not a bug),
// but this combination -- a real invariant alongside real in-place struct
// mutation through a container -- had 0 direct e2e coverage before this
// sweep. See BUG-68 in docs/TODO_CURRENT.md for the actual bug this sweep
// row led to (a silent ensures-verification gap, plus a loop-invariant
// preservation gap for FieldAssign-mutated struct fields -- both fixed;
// this test is the "container in the loop, not just scalar loop state"
// half of that finding).
#[test]
fn loop_invariant_with_vec_of_struct_mutated_in_place_runs_correctly_on_both_backends() {
    let src = write_tmp_vani(
        "invariant_vec_struct_mutate",
        r#"
struct Counter { n: i64 }
fn main() -> i64 {
  let cs: Vec<Counter> = vec(Counter { n: 0 }, Counter { n: 0 }, Counter { n: 0 }, Counter { n: 0 }, Counter { n: 0 });
  let i: i64 = 0;
  while i < 5
  invariant i >= 0;
  invariant i <= 5;
  {
    let c: mut ref Counter = mut ref cs[i];
    c.n = i * 10;
    i = i + 1;
  }
  prove i == 5;
  let j: i64 = 0;
  let sum: i64 = 0;
  while j < 5 {
    sum = sum + cs[j].n;
    j = j + 1;
  }
  print sum;
  return 0;
}
"#,
    );
    let binary = env!("CARGO_BIN_EXE_intentc");
    for backend_args in [
        vec!["run", src.to_str().unwrap()],
        vec!["run", src.to_str().unwrap(), "--backend=c"],
    ] {
        let output = Command::new(binary)
            .args(&backend_args)
            .output()
            .unwrap_or_else(|e| panic!("intentc {:?} should execute: {e}", backend_args));
        assert!(output.status.success(), "{:?}: status {:?}, stderr: {}", backend_args, output.status, String::from_utf8_lossy(&output.stderr));
        let stdout = String::from_utf8_lossy(&output.stdout).replace("\r\n", "\n");
        assert_eq!(stdout, "100\n", "for {:?}; got: {}", backend_args, stdout);
    }
}

// BUG-69, found sweeping "ensures on a function returning Option<Vec<T>>
// or Result<Struct, E>": unrelated to the SMT/generics angle the sweep row
// was about, this is a general LLVM backend crash -- `TypedStmt::If`'s
// tree emitter never updated `ctx.current_block` to the merge/cont block
// after the if (every other multi-block construct did), so `vec_fill`'s
// hand-rolled SSA loop -- the one builtin that reads `ctx.current_block`
// to name its phi's entry-edge predecessor -- wired its phi to a stale
// block whenever it was textually preceded by ANY plain `if` in the same
// function: "PHI node entries do not match predecessors!" at the LLVM
// verifier. Nothing to do with Option/Result/generics; just the first
// real program in this sweep that happened to call `vec_fill` after an
// `if`. The C backend was unaffected (no phi/SSA-block bookkeeping).
// Fixed by setting `ctx.current_block = cont_lbl` after the if. This test
// covers the actual sweep-row scenario end-to-end: Option<Vec<T>> and
// Result<Struct, E> return types, `vec_fill` called after an `if`,
// correct runtime values on both backends.
#[test]
fn option_vec_and_result_struct_with_vec_fill_after_if_run_correctly_on_both_backends() {
    let src = write_tmp_vani(
        "option_vec_result_struct_vecfill_after_if",
        r#"
struct Pt { x: i64, y: i64 }
enum MyErr { BadInput }
fn maybe_make(n: i64) -> Option<Vec<i64>> {
  if n < 0 {
    return Option.None;
  }
  let xs: Vec<i64> = vec_fill(n, 7);
  return Option.Some(xs);
}
fn make_pt(x: i64, y: i64) -> Result<Pt, MyErr>
requires x >= 0;
{
  if x == 0 {
    return Result.Err(MyErr.BadInput);
  }
  return Result.Ok(Pt { x: x, y: y });
}
fn main() -> i64 {
  let r1: Option<Vec<i64>> = maybe_make(3);
  let v1: i64 = match r1 {
    Option.Some(xs) then len(xs) as i64,
    Option.None then -1,
  };
  print v1;
  let r2: Option<Vec<i64>> = maybe_make(0 - 1);
  let v2: i64 = match r2 {
    Option.Some(xs) then len(xs) as i64,
    Option.None then -1,
  };
  print v2;
  let pr1: Result<Pt, MyErr> = make_pt(3, 4);
  let s1: i64 = match pr1 {
    Result.Ok(p) then p.x + p.y,
    Result.Err(_) then -999,
  };
  print s1;
  let pr2: Result<Pt, MyErr> = make_pt(0, 0);
  let s2: i64 = match pr2 {
    Result.Ok(p) then p.x + p.y,
    Result.Err(_) then -999,
  };
  print s2;
  return 0;
}
"#,
    );
    let binary = env!("CARGO_BIN_EXE_intentc");
    for backend_args in [
        vec!["run", src.to_str().unwrap()],
        vec!["run", src.to_str().unwrap(), "--backend=c"],
    ] {
        let output = Command::new(binary)
            .args(&backend_args)
            .output()
            .unwrap_or_else(|e| panic!("intentc {:?} should execute: {e}", backend_args));
        assert!(output.status.success(), "{:?}: status {:?}, stderr: {}", backend_args, output.status, String::from_utf8_lossy(&output.stderr));
        let stdout = String::from_utf8_lossy(&output.stdout).replace("\r\n", "\n");
        assert_eq!(stdout, "3\n-1\n7\n-999\n", "for {:?}; got: {}", backend_args, stdout);
    }
}

// Testing-matrix sweep, "container x generics / monomorphization": a
// user-defined generic struct `Box2<T> { items: Vec<T> }` instantiated at
// TWO different T (i64 and OwnedStr) in the same program. Same bug class
// as BUG-46 (built-in generic enums) but for struct construction --
// Env::resolve_struct_name's "exactly one candidate" fallback broke every
// Box2 { .. } construction site once a second instantiation existed
// anywhere in the program. Fixed via resolve_bare_struct_lits_in_stmt.
#[test]
fn generic_struct_two_instantiations_run_correctly_on_both_backends() {
    let src = write_tmp_vani(
        "generic_struct_two_instantiations",
        r#"
struct Box2<T> { items: Vec<T> }
fn main() -> i64 {
  let bi: Box2<i64> = Box2 { items: vec(1, 2, 3) };
  let bs: Box2<OwnedStr> = Box2 { items: vec("a" + "", "b" + "") };
  let total: i64 = bi.items[0] + bi.items[1] + bi.items[2];
  let s0: OwnedStr = clone_at(ref bs.items, 0);
  let s1: OwnedStr = clone_at(ref bs.items, 1);
  print total;
  print s0;
  print s1;
  print len(bs.items) as i64;
  return 0;
}
"#,
    );
    let binary = env!("CARGO_BIN_EXE_intentc");
    for backend_args in [
        vec!["run", src.to_str().unwrap()],
        vec!["run", src.to_str().unwrap(), "--backend=c"],
    ] {
        let output = Command::new(binary)
            .args(&backend_args)
            .output()
            .unwrap_or_else(|e| panic!("intentc {:?} should execute: {e}", backend_args));
        assert!(output.status.success(), "{:?}: status {:?}, stderr: {}", backend_args, output.status, String::from_utf8_lossy(&output.stderr));
        let stdout = String::from_utf8_lossy(&output.stdout).replace("\r\n", "\n");
        assert_eq!(stdout, "6\na\nb\n2\n", "for {:?}; got: {}", backend_args, stdout);
    }
}

// Testing-matrix sweep, "generic fn first<T>(xs: ref Vec<T>) -> T
// monomorphized over Struct and Tuple T": found BUG-71 (generic inference
// through `ref Vec<T>` bound T to the whole Vec instead of its element,
// for ANY T -- not container/generics-specific in a narrow sense, but a
// general generic-call inference bug) and BUG-72 (a generic fn specialized
// over a Tuple T mangled its name with literal `[`/`]` from Tuple's
// derived-Debug fallback, crashing the LLVM backend's "expected '(' in
// call" -- C backend was unaffected by this specific repro). Both fixed;
// this test exercises the full row end-to-end: scalar, Struct, and Tuple T
// through the same generic fn, correct values on both backends.
#[test]
fn generic_fn_ref_vec_t_over_scalar_struct_tuple_runs_correctly_on_both_backends() {
    let src = write_tmp_vani(
        "generic_fn_ref_vec_t_struct_tuple",
        r#"
struct Pt { x: i64, y: i64 }
fn first<T>(xs: ref Vec<T>) -> T {
  return xs[0];
}
fn main() -> i64 {
  let nums: Vec<i64> = vec(10, 20, 30);
  let n: i64 = first(ref nums);
  let pts: Vec<Pt> = vec(Pt { x: 1, y: 2 }, Pt { x: 3, y: 4 });
  let p: Pt = first(ref pts);
  let tups: Vec<(i64, i64)> = vec((5, 6), (7, 8));
  let t: (i64, i64) = first(ref tups);
  print n;
  print p.x;
  print p.y;
  print t.0;
  print t.1;
  return 0;
}
"#,
    );
    let binary = env!("CARGO_BIN_EXE_intentc");
    for backend_args in [
        vec!["run", src.to_str().unwrap()],
        vec!["run", src.to_str().unwrap(), "--backend=c"],
    ] {
        let output = Command::new(binary)
            .args(&backend_args)
            .output()
            .unwrap_or_else(|e| panic!("intentc {:?} should execute: {e}", backend_args));
        assert!(output.status.success(), "{:?}: status {:?}, stderr: {}", backend_args, output.status, String::from_utf8_lossy(&output.stderr));
        let stdout = String::from_utf8_lossy(&output.stdout).replace("\r\n", "\n");
        assert_eq!(stdout, "10\n1\n2\n5\n6\n", "for {:?}; got: {}", backend_args, stdout);
    }
}

// Testing-matrix sweep, "container x generics": Vec<GenericStruct<i64>>
// alongside Vec<GenericStruct<f64>> (here OwnedStr instead of f64, to also
// cover a non-Copy instantiation) -- two different monomorphizations of
// the same generic struct, each wrapped in its own Vec. Compounds BUG-70's
// bug class (2+ instantiations of the same generic struct) with BUG-61's
// territory (container-element codegen that's written per-shape). Checked
// 2026-08-02, not a bug -- both compile and run correctly on both
// backends.
#[test]
fn vec_of_generic_struct_two_monomorphizations_runs_correctly_on_both_backends() {
    let src = write_tmp_vani(
        "vec_of_generic_struct_two_mono",
        r#"
struct Box2<T> { val: T }
fn main() -> i64 {
  let vi: Vec<Box2<i64>> = vec(Box2 { val: 100 }, Box2 { val: 200 });
  let vs: Vec<Box2<OwnedStr>> = vec(Box2 { val: "hello" + "" }, Box2 { val: "world" + "" });
  let sum_i: i64 = vi[0].val + vi[1].val;
  let cs0: Box2<OwnedStr> = clone_at(ref vs, 0);
  let cs1: Box2<OwnedStr> = clone_at(ref vs, 1);
  print sum_i;
  print cs0.val;
  print cs1.val;
  print len(vi) as i64;
  print len(vs) as i64;
  return 0;
}
"#,
    );
    let binary = env!("CARGO_BIN_EXE_intentc");
    for backend_args in [
        vec!["run", src.to_str().unwrap()],
        vec!["run", src.to_str().unwrap(), "--backend=c"],
    ] {
        let output = Command::new(binary)
            .args(&backend_args)
            .output()
            .unwrap_or_else(|e| panic!("intentc {:?} should execute: {e}", backend_args));
        assert!(output.status.success(), "{:?}: status {:?}, stderr: {}", backend_args, output.status, String::from_utf8_lossy(&output.stderr));
        let stdout = String::from_utf8_lossy(&output.stdout).replace("\r\n", "\n");
        assert_eq!(stdout, "300\nhello\nworld\n2\n2\n", "for {:?}; got: {}", backend_args, stdout);
    }
}

// Testing-matrix sweep, "enum variant payload is Vec<Struct>". Checked
// 2026-08-02, not a bug -- construction, tag-match dispatch, and drop all
// work correctly on both backends.
#[test]
fn enum_variant_payload_vec_of_struct_runs_correctly_on_both_backends() {
    let src = write_tmp_vani(
        "enum_payload_vec_of_struct",
        r#"
struct Pt { x: i64, y: i64 }
enum Bag { Items(Vec<Pt>), Empty }
fn build(use_items: bool) -> Bag {
  if use_items {
    return Bag.Items(vec(Pt { x: 1, y: 2 }, Pt { x: 3, y: 4 }));
  }
  return Bag.Empty;
}
fn classify(b: Bag) -> i64 {
  return match b {
    Bag.Items then 1,
    Bag.Empty then 0,
  };
}
fn main() -> i64 {
  let a: Bag = build(true);
  let z: Bag = build(false);
  print classify(a);
  print classify(z);
  return 0;
}
"#,
    );
    let binary = env!("CARGO_BIN_EXE_intentc");
    for backend_args in [
        vec!["run", src.to_str().unwrap()],
        vec!["run", src.to_str().unwrap(), "--backend=c"],
    ] {
        let output = Command::new(binary)
            .args(&backend_args)
            .output()
            .unwrap_or_else(|e| panic!("intentc {:?} should execute: {e}", backend_args));
        assert!(output.status.success(), "{:?}: status {:?}, stderr: {}", backend_args, output.status, String::from_utf8_lossy(&output.stderr));
        let stdout = String::from_utf8_lossy(&output.stdout).replace("\r\n", "\n");
        assert_eq!(stdout, "1\n0\n", "for {:?}; got: {}", backend_args, stdout);
    }
}

// BUG-74: enum variant payload is a Tuple containing an Array
// (`(i64, [i64; 3])`). Three layered bugs found+fixed (checker admission
// gate + two C-backend codegen gaps -- typedef ordering and array-element
// initializer syntax); see docs/TODO_CURRENT.md for the full writeup.
// This test also covers the payload's construction actually carrying the
// right values by unpacking through a helper (destructure-binding a
// non-Copy... actually Copy here, but still routed through match).
#[test]
fn enum_variant_payload_tuple_containing_array_runs_correctly_on_both_backends() {
    let src = write_tmp_vani(
        "enum_payload_tuple_array",
        r#"
enum Rec { Full((i64, [i64; 3])), Nothing }
fn build(has: bool) -> Rec {
  if has {
    return Rec.Full((42, [1, 2, 3]));
  }
  return Rec.Nothing;
}
fn classify(r: Rec) -> i64 {
  return match r {
    Rec.Full then 1,
    Rec.Nothing then 0,
  };
}
fn main() -> i64 {
  let a: Rec = build(true);
  let z: Rec = build(false);
  print classify(a);
  print classify(z);
  return 0;
}
"#,
    );
    let binary = env!("CARGO_BIN_EXE_intentc");
    for backend_args in [
        vec!["run", src.to_str().unwrap()],
        vec!["run", src.to_str().unwrap(), "--backend=c"],
    ] {
        let output = Command::new(binary)
            .args(&backend_args)
            .output()
            .unwrap_or_else(|e| panic!("intentc {:?} should execute: {e}", backend_args));
        assert!(output.status.success(), "{:?}: status {:?}, stderr: {}", backend_args, output.status, String::from_utf8_lossy(&output.stderr));
        let stdout = String::from_utf8_lossy(&output.stdout).replace("\r\n", "\n");
        assert_eq!(stdout, "1\n0\n", "for {:?}; got: {}", backend_args, stdout);
    }
}

// BUG-74b in isolation: a plain local Tuple<Array> binding, no enum
// involved, confirming the C-backend initializer-syntax fix is general.
#[test]
fn tuple_containing_array_local_runs_correctly_on_both_backends() {
    let src = write_tmp_vani(
        "tuple_containing_array_local",
        r#"
fn main() -> i64 {
  let x: (i64, [i64; 3]) = (42, [1, 2, 3]);
  print x.0;
  return 0;
}
"#,
    );
    let binary = env!("CARGO_BIN_EXE_intentc");
    for backend_args in [
        vec!["run", src.to_str().unwrap()],
        vec!["run", src.to_str().unwrap(), "--backend=c"],
    ] {
        let output = Command::new(binary)
            .args(&backend_args)
            .output()
            .unwrap_or_else(|e| panic!("intentc {:?} should execute: {e}", backend_args));
        assert!(output.status.success(), "{:?}: status {:?}, stderr: {}", backend_args, output.status, String::from_utf8_lossy(&output.stderr));
        let stdout = String::from_utf8_lossy(&output.stdout).replace("\r\n", "\n");
        assert_eq!(stdout, "42\n", "for {:?}; got: {}", backend_args, stdout);
    }
}

// BUG-75, found sweeping "match over Vec<Enum> with 3+ variants, mixed
// Copy/non-Copy payloads": clone_at on a mixed-payload-type enum element
// silently corrupted every scalar payload on LLVM (two layered bugs --
// wrong OwnedStr-tag detection, then an LLVM type mismatch once that
// detection was fixed; see docs/TODO_CURRENT.md). This test iterates a
// Vec<Item> with 5 elements across 4 variant shapes (i64, OwnedStr, bool,
// no-payload), clone_at-ing each and matching, hand-computed expected sum.
#[test]
fn match_over_vec_enum_mixed_payloads_runs_correctly_on_both_backends() {
    let src = write_tmp_vani(
        "match_vec_enum_mixed_payloads",
        r#"
enum Item { Num(i64), Text(OwnedStr), Flag(bool), Nothing }
fn describe(it: Item) -> i64 {
  return match it {
    Item.Num(n) then n,
    Item.Text then 1000,
    Item.Flag(b) then if b { 2000 } else { 3000 },
    Item.Nothing then 0,
  };
}
fn main() -> i64 {
  let items: Vec<Item> = vec(
    Item.Num(7),
    Item.Text("hello" + ""),
    Item.Flag(true),
    Item.Flag(false),
    Item.Nothing,
  );
  let total: i64 = 0;
  let i: i64 = 0;
  let n: i64 = len(items) as i64;
  while i < n {
    let it: Item = clone_at(ref items, i);
    total = total + describe(it);
    i = i + 1;
  }
  print total;
  return 0;
}
"#,
    );
    let binary = env!("CARGO_BIN_EXE_intentc");
    for backend_args in [
        vec!["run", src.to_str().unwrap()],
        vec!["run", src.to_str().unwrap(), "--backend=c"],
    ] {
        let output = Command::new(binary)
            .args(&backend_args)
            .output()
            .unwrap_or_else(|e| panic!("intentc {:?} should execute: {e}", backend_args));
        assert!(output.status.success(), "{:?}: status {:?}, stderr: {}", backend_args, output.status, String::from_utf8_lossy(&output.stderr));
        let stdout = String::from_utf8_lossy(&output.stdout).replace("\r\n", "\n");
        // 7 (Num) + 1000 (Text) + 2000 (Flag true) + 3000 (Flag false) + 0 (Nothing) = 6007
        assert_eq!(stdout, "6007\n", "for {:?}; got: {}", backend_args, stdout);
    }
}

// Testing-matrix sweep, "nested if let 2 levels deep on a Vec element":
// a genuinely nested pattern (`if let Option.Some(Result.Ok(v)) = ...`) is
// cleanly rejected at parse time on both backends -- a real, documented
// v1 limitation ("no nested patterns", intermediate/02_enums_payloads.md),
// not a divergence bug. The flattened two-level form (two separate if-let
// statements, per that doc's own "flatten with two match levels"
// guidance) combined with an outer binding sourced from a Vec element via
// clone_at found BUG-76: `Option<UserEnum>.None`'s zero-value placeholder
// crashed the LLVM backend (see docs/TODO_CURRENT.md) -- nothing to do
// with Vec/clone_at/if-let nesting specifically, just the first repro in
// this sweep that happened to construct an Option<T> where T is a
// user-defined enum. Fixed; this test covers the full flattened-nesting
// scenario end-to-end.
#[test]
fn flattened_nested_if_let_on_vec_element_runs_correctly_on_both_backends() {
    let src = write_tmp_vani(
        "flattened_nested_iflet_vec_element",
        r#"
enum MyResult { Ok(i64), Err(i64) }
fn main() -> i64 {
  let xs: Vec<Option<MyResult>> = vec(
    Option.Some(MyResult.Ok(7)),
    Option.Some(MyResult.Err(0 - 1)),
    Option.None,
  );
  let i: i64 = 0;
  let n: i64 = len(xs) as i64;
  let total: i64 = 0;
  while i < n {
    let outer: Option<MyResult> = clone_at(ref xs, i);
    if let Option.Some(inner) = outer {
      if let MyResult.Ok(v) = inner {
        total = total + v;
      }
      if let MyResult.Err(e) = inner {
        total = total + (0 - e) * 100;
      }
    }
    i = i + 1;
  }
  print total;
  return 0;
}
"#,
    );
    let binary = env!("CARGO_BIN_EXE_intentc");
    for backend_args in [
        vec!["run", src.to_str().unwrap()],
        vec!["run", src.to_str().unwrap(), "--backend=c"],
    ] {
        let output = Command::new(binary)
            .args(&backend_args)
            .output()
            .unwrap_or_else(|e| panic!("intentc {:?} should execute: {e}", backend_args));
        assert!(output.status.success(), "{:?}: status {:?}, stderr: {}", backend_args, output.status, String::from_utf8_lossy(&output.stderr));
        let stdout = String::from_utf8_lossy(&output.stdout).replace("\r\n", "\n");
        // 7 (Ok(7)) + 1*100 (Err(-1)) = 107
        assert_eq!(stdout, "107\n", "for {:?}; got: {}", backend_args, stdout);
    }
}

// BUG-77, testing-matrix sweep "extern C fn taking/returning a Struct BY
// VALUE": a real, linked C function that TAKES a small struct by value
// already worked on both backends (Closure #288's ABI lowering), but one
// that RETURNS a small struct by value crashed the LLVM backend the
// instant it was actually called (not just declared) -- the ABI-lowered
// call result (`i64`) was handed to callers as if it were already the
// real `%Struct_X` type, an LLVM type mismatch. C backend was unaffected.
// Exercises both directions (param AND return) against a real linked C
// shim on both backends.
#[test]
fn extern_c_struct_by_value_param_and_return_runs_correctly_on_both_backends() {
    use std::fs;
    let src = write_tmp_vani(
        "extern_struct_by_value",
        r#"
struct Point { x: i32, y: i32 }
extern "C" fn make_point(x: i32, y: i32) -> Point;
extern "C" fn point_sum(p: Point) -> i32;
fn main() -> i64 {
  let p: Point = make_point(3 as i32, 4 as i32);
  let s: i32 = point_sum(p);
  print s as i64;
  return 0;
}
"#,
    );
    let dir = src.parent().unwrap().to_path_buf();
    let shim_c = dir.join("shim.c");
    fs::write(
        &shim_c,
        "#include <stdint.h>\n\
         typedef struct { int32_t x; int32_t y; } Point;\n\
         Point make_point(int32_t x, int32_t y) { Point p; p.x = x; p.y = y; return p; }\n\
         int32_t point_sum(Point p) { return p.x + p.y; }\n",
    )
    .expect("write shim.c");

    let binary = env!("CARGO_BIN_EXE_intentc");

    // LLVM backend: `vanic build` (AOT, always LLVM) + `--link-with` + `-lm`
    // (the generated runtime helpers pull in libm symbols that `cc` only
    // resolves when linked explicitly here, per the FFI tutorial's own
    // `--link-with=m` guidance).
    let llvm_bin = dir.join("prog_llvm");
    let build = Command::new(binary)
        .args([
            "build",
            src.to_str().unwrap(),
            "--link-with",
            shim_c.to_str().unwrap(),
            "-lm",
            "-o",
            llvm_bin.to_str().unwrap(),
        ])
        .output()
        .expect("intentc build runs");
    assert!(
        build.status.success(),
        "LLVM build --link-with failed:\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&build.stdout),
        String::from_utf8_lossy(&build.stderr),
    );
    let run_llvm = Command::new(&llvm_bin).output().expect("LLVM binary runs");
    assert!(
        run_llvm.status.success(),
        "LLVM binary exited non-zero: {:?} (stdout: {}, stderr: {})",
        run_llvm.status,
        String::from_utf8_lossy(&run_llvm.stdout),
        String::from_utf8_lossy(&run_llvm.stderr),
    );
    assert_eq!(
        String::from_utf8_lossy(&run_llvm.stdout).replace("\r\n", "\n"),
        "7\n",
        "LLVM backend output mismatch"
    );

    // C backend: `vanic run --backend=c --link-with` (JIT-equivalent via
    // gcc, no separate build step needed for the C path).
    let run_c = Command::new(binary)
        .args([
            "run",
            src.to_str().unwrap(),
            "--backend=c",
            "--link-with",
            shim_c.to_str().unwrap(),
        ])
        .output()
        .expect("intentc run --backend=c --link-with runs");
    assert!(
        run_c.status.success(),
        "C backend run --link-with failed:\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&run_c.stdout),
        String::from_utf8_lossy(&run_c.stderr),
    );
    assert_eq!(
        String::from_utf8_lossy(&run_c.stdout).replace("\r\n", "\n"),
        "7\n",
        "C backend output mismatch"
    );

    let _ = fs::remove_dir_all(&dir);
}

// Testing-matrix sweep, "#[no_mangle] fn with a Tuple/Array parameter".
// BUG-44's fix was only verified with scalar params. Checked 2026-08-02,
// not a bug: both a Tuple-typed and an Array-typed parameter on a
// no_mangle fn compute correctly on both backends.
#[test]
fn no_mangle_fn_with_tuple_and_array_params_runs_correctly_on_both_backends() {
    let src = write_tmp_vani(
        "no_mangle_tuple_array_params",
        r#"
#[no_mangle]
fn sum_pair(p: (i64, i64)) -> i64 { return p.0 + p.1; }
#[no_mangle]
fn sum_arr(a: [i64; 4]) -> i64 { return a[0] + a[1] + a[2] + a[3]; }
fn main() -> i64 {
  print sum_pair((10, 20));
  print sum_arr([1, 2, 3, 4]);
  return 0;
}
"#,
    );
    let binary = env!("CARGO_BIN_EXE_intentc");
    for backend_args in [
        vec!["run", src.to_str().unwrap()],
        vec!["run", src.to_str().unwrap(), "--backend=c"],
    ] {
        let output = Command::new(binary)
            .args(&backend_args)
            .output()
            .unwrap_or_else(|e| panic!("intentc {:?} should execute: {e}", backend_args));
        assert!(output.status.success(), "{:?}: status {:?}, stderr: {}", backend_args, output.status, String::from_utf8_lossy(&output.stderr));
        let stdout = String::from_utf8_lossy(&output.stdout).replace("\r\n", "\n");
        assert_eq!(stdout, "30\n10\n", "for {:?}; got: {}", backend_args, stdout);
    }
}

// Testing-matrix sweep, "partial move out of a Vec<T> struct field,
// followed by clone_at on a DIFFERENT field of the same struct instance".
// Checked 2026-08-02, not a bug -- both operations compute correctly and
// no double-free/corruption on scope exit (the moved-out field isn't
// freed twice) on either backend.
#[test]
fn partial_move_then_clone_at_different_field_runs_correctly_on_both_backends() {
    let src = write_tmp_vani(
        "partial_move_then_clone_at",
        r#"
struct Holder { xs: Vec<i64>, ys: Vec<OwnedStr> }
fn main() -> i64 {
  let h: Holder = Holder { xs: vec(1, 2, 3), ys: vec("a" + "", "b" + "", "c" + "") };
  let moved_xs: Vec<i64> = h.xs;
  let sum: i64 = moved_xs[0] + moved_xs[1] + moved_xs[2];
  let s0: OwnedStr = clone_at(ref h.ys, 0);
  let s1: OwnedStr = clone_at(ref h.ys, 1);
  print sum;
  print s0;
  print s1;
  return 0;
}
"#,
    );
    let binary = env!("CARGO_BIN_EXE_intentc");
    for backend_args in [
        vec!["run", src.to_str().unwrap()],
        vec!["run", src.to_str().unwrap(), "--backend=c"],
    ] {
        let output = Command::new(binary)
            .args(&backend_args)
            .output()
            .unwrap_or_else(|e| panic!("intentc {:?} should execute: {e}", backend_args));
        assert!(output.status.success(), "{:?}: status {:?}, stderr: {}", backend_args, output.status, String::from_utf8_lossy(&output.stderr));
        let stdout = String::from_utf8_lossy(&output.stdout).replace("\r\n", "\n");
        assert_eq!(stdout, "6\na\nb\n", "for {:?}; got: {}", backend_args, stdout);
    }
}

// Testing-matrix sweep, "clone_at chained three levels deep": clone_at
// on a Vec<Vec<Struct>>, then clone_at again on the result. Checked
// 2026-08-02, not a bug -- both levels compute correctly and outer stays
// untouched (independent of the clone) on both backends.
#[test]
fn clone_at_chained_three_levels_deep_runs_correctly_on_both_backends() {
    let src = write_tmp_vani(
        "clone_at_chained_three_levels",
        r#"
struct Pt { x: i64, y: i64 }
fn main() -> i64 {
  let outer: Vec<Vec<Pt>> = vec(
    vec(Pt { x: 1, y: 2 }, Pt { x: 3, y: 4 }),
    vec(Pt { x: 5, y: 6 }, Pt { x: 7, y: 8 }, Pt { x: 9, y: 10 }),
  );
  let middle: Vec<Pt> = clone_at(ref outer, 1);
  let inner: Pt = clone_at(ref middle, 2);
  print inner.x;
  print inner.y;
  print len(middle) as i64;
  print len(outer) as i64;
  return 0;
}
"#,
    );
    let binary = env!("CARGO_BIN_EXE_intentc");
    for backend_args in [
        vec!["run", src.to_str().unwrap()],
        vec!["run", src.to_str().unwrap(), "--backend=c"],
    ] {
        let output = Command::new(binary)
            .args(&backend_args)
            .output()
            .unwrap_or_else(|e| panic!("intentc {:?} should execute: {e}", backend_args));
        assert!(output.status.success(), "{:?}: status {:?}, stderr: {}", backend_args, output.status, String::from_utf8_lossy(&output.stderr));
        let stdout = String::from_utf8_lossy(&output.stdout).replace("\r\n", "\n");
        assert_eq!(stdout, "9\n10\n3\n2\n", "for {:?}; got: {}", backend_args, stdout);
    }
}

// Testing-matrix sweep, "struct with both a Vec<T> field and an OwnedStr
// field, partially moved (one field taken, the other read), then
// dropped". Checked 2026-08-02, not a bug -- runs correctly on both
// backends. Separately confirmed via `valgrind --leak-check=full` against
// native binaries built from this exact program: 0 leaks, balanced
// alloc/free counts on both backends (not re-run under valgrind in CI --
// this test covers the correctness half; the memory-safety half was a
// one-time manual verification recorded in docs/TODO_CURRENT.md).
#[test]
fn struct_vec_and_owned_str_fields_partial_move_runs_correctly_on_both_backends() {
    let src = write_tmp_vani(
        "struct_vec_owned_str_partial_move",
        r#"
struct Rec { xs: Vec<i64>, name: OwnedStr }
fn main() -> i64 {
  let r: Rec = Rec { xs: vec(1, 2, 3), name: "widget" + "" };
  let taken: Vec<i64> = r.xs;
  print r.name;
  print taken[0] + taken[1] + taken[2];
  return 0;
}
"#,
    );
    let binary = env!("CARGO_BIN_EXE_intentc");
    for backend_args in [
        vec!["run", src.to_str().unwrap()],
        vec!["run", src.to_str().unwrap(), "--backend=c"],
    ] {
        let output = Command::new(binary)
            .args(&backend_args)
            .output()
            .unwrap_or_else(|e| panic!("intentc {:?} should execute: {e}", backend_args));
        assert!(output.status.success(), "{:?}: status {:?}, stderr: {}", backend_args, output.status, String::from_utf8_lossy(&output.stderr));
        let stdout = String::from_utf8_lossy(&output.stdout).replace("\r\n", "\n");
        assert_eq!(stdout, "widget\n6\n", "for {:?}; got: {}", backend_args, stdout);
    }
}

// BUG-78, found sweeping the testing-matrix's Big-O row (a
// `#[complexity(...)]`-annotated -- actually just plain, since v1's
// `--big-o` flag needs no attribute -- fn operating on `Vec<Struct>`/
// `Array<Tuple,N>`, not just `Vec<i64>`). The --big-o analyzer itself
// correctly classified Vec<Struct> loops (O(n)/O(n^2)) with no crash --
// not a bug. But a function taking `Array<Tuple,N>` (or `Array<Struct,N>`)
// BY VALUE as a parameter crashed the C backend entirely unrelated to
// Big-O: format_declarator's Array arm used a leaf-only type spelling
// table instead of the correct per-shape one. This test exercises the
// real bug end-to-end.
#[test]
fn array_of_tuple_and_struct_fn_params_run_correctly_on_both_backends() {
    let src = write_tmp_vani(
        "array_of_tuple_struct_params",
        r#"
struct Pt { x: i64, y: i64 }
fn sum_array_tuple(arr: [(i64, i64); 5]) -> i64 {
  let total: i64 = 0;
  let i: i64 = 0;
  while i < 5 {
    total = total + arr[i].0 + arr[i].1;
    i = i + 1;
  }
  return total;
}
fn sum_array_struct(arr: [Pt; 3]) -> i64 {
  let total: i64 = 0;
  let i: i64 = 0;
  while i < 3 {
    total = total + arr[i].x + arr[i].y;
    i = i + 1;
  }
  return total;
}
fn main() -> i64 {
  let a: [(i64, i64); 5] = [(1,1),(2,2),(3,3),(4,4),(5,5)];
  let b: [Pt; 3] = [Pt { x: 1, y: 2 }, Pt { x: 3, y: 4 }, Pt { x: 5, y: 6 }];
  print sum_array_tuple(a);
  print sum_array_struct(b);
  return 0;
}
"#,
    );
    let binary = env!("CARGO_BIN_EXE_intentc");
    for backend_args in [
        vec!["run", src.to_str().unwrap()],
        vec!["run", src.to_str().unwrap(), "--backend=c"],
    ] {
        let output = Command::new(binary)
            .args(&backend_args)
            .output()
            .unwrap_or_else(|e| panic!("intentc {:?} should execute: {e}", backend_args));
        assert!(output.status.success(), "{:?}: status {:?}, stderr: {}", backend_args, output.status, String::from_utf8_lossy(&output.stderr));
        let stdout = String::from_utf8_lossy(&output.stdout).replace("\r\n", "\n");
        // sum_array_tuple: (1+1)+(2+2)+(3+3)+(4+4)+(5+5) = 30
        // sum_array_struct: (1+2)+(3+4)+(5+6) = 21
        assert_eq!(stdout, "30\n21\n", "for {:?}; got: {}", backend_args, stdout);
    }
}

// Testing-matrix sweep, "parallel for iterating a Vec<Struct> with a
// reduce accumulating a struct field". Checked 2026-08-02, not a bug --
// computes correctly on both backends.
#[test]
fn parallel_for_reduce_over_vec_struct_field_runs_correctly_on_both_backends() {
    let src = write_tmp_vani(
        "parallel_for_vec_struct_reduce",
        r#"
struct Pt { x: i64, y: i64 }
fn main() -> i64 {
  let pts: Vec<Pt> = vec(Pt { x: 1, y: 10 }, Pt { x: 2, y: 20 }, Pt { x: 3, y: 30 }, Pt { x: 4, y: 40 });
  let n: i64 = len(pts) as i64;
  let sum: i64 = 0;
  parallel for i from 0 to n
  reduce sum with +;
  {
    sum = sum + pts[i].x;
  }
  print sum;
  return 0;
}
"#,
    );
    let binary = env!("CARGO_BIN_EXE_intentc");
    for backend_args in [
        vec!["run", src.to_str().unwrap()],
        vec!["run", src.to_str().unwrap(), "--backend=c"],
    ] {
        let output = Command::new(binary)
            .args(&backend_args)
            .output()
            .unwrap_or_else(|e| panic!("intentc {:?} should execute: {e}", backend_args));
        assert!(output.status.success(), "{:?}: status {:?}, stderr: {}", backend_args, output.status, String::from_utf8_lossy(&output.stderr));
        let stdout = String::from_utf8_lossy(&output.stdout).replace("\r\n", "\n");
        assert_eq!(stdout, "10\n", "for {:?}; got: {}", backend_args, stdout);
    }
}

// BUG-79, testing-matrix sweep "struct with both a SIMD Vec128/Vec256
// field AND a plain Vec field": c_element_storage never had arms for
// Type::Vec128/Vec256/Vec512, so a struct field of that type declared
// itself with a c_leaf_type placeholder comment ("/* vec128<T> */"),
// invalid C. Fixed; this test exercises the full row end-to-end on both
// backends.
#[test]
fn struct_with_simd_and_plain_vec_fields_runs_correctly_on_both_backends() {
    let src = write_tmp_vani(
        "struct_simd_and_plain_vec",
        r#"
struct Combo { lane: vec128<f64>, xs: Vec<f64> }
struct Combo2 { lane: vec256<f64>, xs: Vec<f64> }
fn main() -> i64 {
  let l: vec128<f64> = simd_splat(3.0 as f64);
  let c: Combo = Combo { lane: l, xs: vec(1.0 as f64, 2.0 as f64, 3.0 as f64) };
  let s: f64 = simd_reduce_add(c.lane);
  let l2: vec256<f64> = simd256_splat(5.0 as f64);
  let c2: Combo2 = Combo2 { lane: l2, xs: vec(10.0 as f64, 20.0 as f64) };
  let s2: f64 = simd256_reduce_add(c2.lane);
  print s;
  print c.xs[0];
  print len(c.xs) as i64;
  print s2;
  print c2.xs[1];
  return 0;
}
"#,
    );
    let binary = env!("CARGO_BIN_EXE_intentc");
    for backend_args in [
        vec!["run", src.to_str().unwrap()],
        vec!["run", src.to_str().unwrap(), "--backend=c"],
    ] {
        let output = Command::new(binary)
            .args(&backend_args)
            .output()
            .unwrap_or_else(|e| panic!("intentc {:?} should execute: {e}", backend_args));
        assert!(output.status.success(), "{:?}: status {:?}, stderr: {}", backend_args, output.status, String::from_utf8_lossy(&output.stderr));
        let stdout = String::from_utf8_lossy(&output.stdout).replace("\r\n", "\n");
        // simd_reduce_add(splat(3.0)) over 2 lanes = 6; xs[0]=1; len=3;
        // simd256_reduce_add(splat(5.0)) over 4 lanes = 20; xs2[1]=20.
        assert_eq!(stdout, "6\n1\n3\n20\n20\n", "for {:?}; got: {}", backend_args, stdout);
    }
}

// BUG-80, testing-matrix sweep "Option<Array<T,N>>/Result<Tuple,E>":
// LLVM was correct throughout; C crashed on Option<[i64;3]> specifically
// (two layered bugs -- match-arm binding used the wrong C type spelling,
// then C arrays can't be copy-assigned via `=` even through a typedef;
// see docs/TODO_CURRENT.md). This test covers the full row end-to-end:
// Option over an Array payload AND Result over a Tuple payload, both
// Some/Ok and None/Err paths, on both backends.
#[test]
fn option_array_and_result_tuple_payloads_run_correctly_on_both_backends() {
    let src = write_tmp_vani(
        "option_array_result_tuple",
        r#"
fn maybe_arr(has: bool) -> Option<[i64; 3]> {
  if has { return Option.Some([1, 2, 3]); }
  return Option.None;
}
fn safe_div_pair(a: i64, b: i64) -> Result<(i64, i64), i64> {
  if b == 0 { return Result.Err(0 - 1); }
  return Result.Ok((a / b, a % b));
}
fn main() -> i64 {
  let oa: Option<[i64; 3]> = maybe_arr(true);
  let total: i64 = match oa {
    Option.Some(arr) then arr[0] + arr[1] + arr[2],
    Option.None then 0 - 999,
  };
  let on: Option<[i64; 3]> = maybe_arr(false);
  let total2: i64 = match on {
    Option.Some(arr) then arr[0] + arr[1] + arr[2],
    Option.None then 0 - 999,
  };
  let r1: Result<(i64, i64), i64> = safe_div_pair(17, 5);
  let s1: i64 = match r1 {
    Result.Ok(pair) then pair.0 * 100 + pair.1,
    Result.Err(_) then 0 - 1,
  };
  let r2: Result<(i64, i64), i64> = safe_div_pair(1, 0);
  let s2: i64 = match r2 {
    Result.Ok(pair) then pair.0 * 100 + pair.1,
    Result.Err(e) then e,
  };
  print total;
  print total2;
  print s1;
  print s2;
  return 0;
}
"#,
    );
    let binary = env!("CARGO_BIN_EXE_intentc");
    for backend_args in [
        vec!["run", src.to_str().unwrap()],
        vec!["run", src.to_str().unwrap(), "--backend=c"],
    ] {
        let output = Command::new(binary)
            .args(&backend_args)
            .output()
            .unwrap_or_else(|e| panic!("intentc {:?} should execute: {e}", backend_args));
        assert!(output.status.success(), "{:?}: status {:?}, stderr: {}", backend_args, output.status, String::from_utf8_lossy(&output.stderr));
        let stdout = String::from_utf8_lossy(&output.stdout).replace("\r\n", "\n");
        // total = 1+2+3 = 6; total2 (None) = -999;
        // s1: 17/5 = 3 rem 2 -> 3*100+2 = 302; s2 (Err(-1)) = -1
        assert_eq!(stdout, "6\n-999\n302\n-1\n", "for {:?}; got: {}", backend_args, stdout);
    }
}

// Testing-matrix sweep (final row): "Vec<Option<Struct>> -- Option of a
// non-Copy struct, stored in a Vec (three-level: Vec -> Option -> Struct)".
// The non-Copy-struct half of this row isn't reachable in v1 at all (a
// clean, general enum-payload-admission restriction, confirmed separately
// -- not a bug). The three-level nesting itself, with a Copy struct,
// compiles and computes correctly on both backends.
#[test]
fn vec_of_option_of_copy_struct_runs_correctly_on_both_backends() {
    let src = write_tmp_vani(
        "vec_option_copy_struct",
        r#"
struct Pt { x: i64, y: i64 }
fn main() -> i64 {
  let items: Vec<Option<Pt>> = vec(Option.Some(Pt { x: 1, y: 2 }), Option.None, Option.Some(Pt { x: 3, y: 4 }));
  let n: i64 = len(items) as i64;
  let i: i64 = 0;
  let total: i64 = 0;
  while i < n {
    let it: Option<Pt> = items[i];
    let v: i64 = match it {
      Option.Some(p) then p.x + p.y,
      Option.None then 0,
    };
    total = total + v;
    i = i + 1;
  }
  print total;
  return 0;
}
"#,
    );
    let binary = env!("CARGO_BIN_EXE_intentc");
    for backend_args in [
        vec!["run", src.to_str().unwrap()],
        vec!["run", src.to_str().unwrap(), "--backend=c"],
    ] {
        let output = Command::new(binary)
            .args(&backend_args)
            .output()
            .unwrap_or_else(|e| panic!("intentc {:?} should execute: {e}", backend_args));
        assert!(output.status.success(), "{:?}: status {:?}, stderr: {}", backend_args, output.status, String::from_utf8_lossy(&output.stderr));
        let stdout = String::from_utf8_lossy(&output.stdout).replace("\r\n", "\n");
        // (1+2) + 0 (None) + (3+4) = 10
        assert_eq!(stdout, "10\n", "for {:?}; got: {}", backend_args, stdout);
    }
}

// BUG-20 residual fix (2026-08-02): three adjacent guard-wiring gaps
// left over from the original BUG-20 fix (which only wired guards into
// the slice-pattern `Slice` arm). (1) A guarded `_` wildcard arm in a
// slice/array match never read `arm.guard` at all -- always behaved as
// an unconditional catch-all. (2) `check_match_str` never type-checked
// or wired `arm.guard` into its generated dispatch -- a guarded string
// match arm always behaved as if its guard were `true`. (3)
// `check_match_float` had the identical gap. All three are the
// "compiles fine but silently produces the wrong answer" class of bug
// that only a real execution test catches.
#[test]
fn slice_pattern_guarded_wildcard_produces_correct_output_on_both_backends() {
    use std::fs;
    use std::path::PathBuf;

    let binary = env!("CARGO_BIN_EXE_intentc");
    let src: PathBuf = std::env::temp_dir().join(format!(
        "intentc-guarded-wildcard-{}-{}.vani",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::write(
        &src,
        r#"
intent "slice_guarded_wildcard_e2e";
fn classify(xs: Vec<i64>, n: i64) -> i64 {
    return match xs {
        [a, b] if n > 10 then a + b,
        _ if n > 5 then 100,
        _ then -1,
    };
}
fn main() -> i64 {
    print classify(vec(1, 2), 20);
    print classify(vec(1, 2, 3), 8);
    print classify(vec(1, 2, 3), 1);
    return 0;
}
"#,
    )
    .expect("write src");

    for backend_args in [
        vec!["run", src.to_str().unwrap()],
        vec!["run", src.to_str().unwrap(), "--backend=c"],
    ] {
        let output = Command::new(binary)
            .args(&backend_args)
            .output()
            .unwrap_or_else(|e| panic!("intentc {:?} should execute: {e}", backend_args));
        assert!(
            output.status.success(),
            "{:?}: status {:?}, stderr: {}",
            backend_args,
            output.status,
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8_lossy(&output.stdout).replace("\r\n", "\n");
        assert_eq!(
            stdout, "3\n100\n-1\n",
            "guarded wildcard must be evaluated, not treated as unconditional, for {:?}; got: {}",
            backend_args, stdout
        );
    }
    let _ = fs::remove_file(&src);
}

#[test]
fn string_match_guard_produces_correct_output_on_both_backends() {
    use std::fs;
    use std::path::PathBuf;

    let binary = env!("CARGO_BIN_EXE_intentc");
    let src: PathBuf = std::env::temp_dir().join(format!(
        "intentc-str-guard-{}-{}.vani",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::write(
        &src,
        r#"
intent "string_match_guard_e2e";
fn classify(s: OwnedStr, n: i64) -> i64 {
    return match s {
        "x" if n > 10 then 1,
        "y" then 2,
        _ then 0,
    };
}
fn main() -> i64 {
    print classify("x" + "", 20);
    print classify("x" + "", 5);
    print classify("y" + "", 20);
    return 0;
}
"#,
    )
    .expect("write src");

    for backend_args in [
        vec!["run", src.to_str().unwrap()],
        vec!["run", src.to_str().unwrap(), "--backend=c"],
    ] {
        let output = Command::new(binary)
            .args(&backend_args)
            .output()
            .unwrap_or_else(|e| panic!("intentc {:?} should execute: {e}", backend_args));
        assert!(
            output.status.success(),
            "{:?}: status {:?}, stderr: {}",
            backend_args,
            output.status,
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8_lossy(&output.stdout).replace("\r\n", "\n");
        assert_eq!(
            stdout, "1\n0\n2\n",
            "string match guard must gate the arm, not be silently ignored, for {:?}; got: {}",
            backend_args, stdout
        );
    }
    let _ = fs::remove_file(&src);
}

#[test]
fn float_match_guard_produces_correct_output_on_both_backends() {
    use std::fs;
    use std::path::PathBuf;

    let binary = env!("CARGO_BIN_EXE_intentc");
    let src: PathBuf = std::env::temp_dir().join(format!(
        "intentc-float-guard-{}-{}.vani",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::write(
        &src,
        r#"
intent "float_match_guard_e2e";
fn classify(x: f64, n: i64) -> i64 {
    return match x {
        1.5 if n > 10 then 1,
        2.5 then 2,
        _ then 0,
    };
}
fn main() -> i64 {
    print classify(1.5, 20);
    print classify(1.5, 5);
    print classify(2.5, 20);
    return 0;
}
"#,
    )
    .expect("write src");

    for backend_args in [
        vec!["run", src.to_str().unwrap()],
        vec!["run", src.to_str().unwrap(), "--backend=c"],
    ] {
        let output = Command::new(binary)
            .args(&backend_args)
            .output()
            .unwrap_or_else(|e| panic!("intentc {:?} should execute: {e}", backend_args));
        assert!(
            output.status.success(),
            "{:?}: status {:?}, stderr: {}",
            backend_args,
            output.status,
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8_lossy(&output.stdout).replace("\r\n", "\n");
        assert_eq!(
            stdout, "1\n0\n2\n",
            "float match guard must gate the arm, not be silently ignored, for {:?}; got: {}",
            backend_args, stdout
        );
    }
    let _ = fs::remove_file(&src);
}

// BUG-66 residual fix (2026-08-02): a closure with a heap-owning
// capture (moved in, not `ref`-captured) stored into a struct field
// used to crash both backends at build/run time (LLVM: `lli` rejects
// the emitted IR as unsized; C: double-free at runtime). The checker
// now rejects the pattern with a clean diagnostic instead. This is an
// execution-level (real `vanic` binary) confirmation that `check`
// exits non-zero with a real diagnostic, not just an in-process
// `compile_to_c` check -- mirrors the "doesn't compile with a real
// toolchain" verification style used for BUG-22 above.
#[test]
fn closure_with_heap_owning_capture_in_struct_field_is_cleanly_rejected() {
    use std::fs;
    use std::path::PathBuf;

    let binary = env!("CARGO_BIN_EXE_intentc");
    let src: PathBuf = std::env::temp_dir().join(format!(
        "intentc-closure-aff-field-{}-{}.vani",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::write(
        &src,
        r#"
struct Handler { cb: Closure(i64) -> i64 }
fn main() -> i64 {
  let data: Vec<i64> = vec(1, 2, 3, 4);
  let cb = fn(extra: i64) -> i64 { return data[0] + extra; };
  let h: Handler = Handler { cb: cb };
  let f: Closure(i64) -> i64 = h.cb;
  print f(5);
  return 0;
}
"#,
    )
    .expect("write src");

    let output = Command::new(binary)
        .args(["check", src.to_str().unwrap()])
        .output()
        .expect("intentc check should execute");
    let _ = fs::remove_file(&src);

    assert!(
        !output.status.success(),
        "expected a clean rejection, got success (this pattern used to crash \
         the backend at build/run time instead of being caught here)"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("heap-owning") && stderr.contains("struct field"),
        "expected the BUG-66-residual diagnostic to mention the heap-owning \
         capture and struct field, got:\n{}",
        stderr
    );
}

// BUG-36 (2026-08-02): the "single mutable borrow" exclusivity rule
// ("`ref` can multiply, `mut ref` must be exclusive") was documented
// but never enforced by the checker at all. `let r: mut ref Vec<i64>
// = mut ref xs; push(r, 4); print xs[0];` used to compile and run
// cleanly on both backends with no diagnostic. Real-binary
// confirmation (not just an in-process `compile()` check) that
// `vanic check` now rejects this with the exclusivity diagnostic.
#[test]
fn mut_ref_exclusivity_violation_is_cleanly_rejected() {
    use std::fs;
    use std::path::PathBuf;

    let binary = env!("CARGO_BIN_EXE_intentc");
    let src: PathBuf = std::env::temp_dir().join(format!(
        "intentc-mut-ref-exclusivity-{}-{}.vani",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::write(
        &src,
        r#"
fn main() -> i64 {
  let xs: Vec<i64> = vec(1, 2, 3);
  let r: mut ref Vec<i64> = mut ref xs;
  push(r, 4);
  print xs[0];
  return 0;
}
"#,
    )
    .expect("write src");

    let output = Command::new(binary)
        .args(["check", src.to_str().unwrap()])
        .output()
        .expect("intentc check should execute");
    let _ = fs::remove_file(&src);

    assert!(
        !output.status.success(),
        "expected a clean rejection, got success (this exact shape used to \
         compile and run cleanly with no diagnostic at all before the fix)"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("mutably borrowed by 'r'"),
        "expected the exclusivity diagnostic, got:\n{}",
        stderr
    );
}

// Companion positive test: a named `mut ref` binding used the
// ordinary, non-aliasing way (created, used, its scope ends, THEN
// the source is read again) must still compile and run correctly on
// both backends -- confirms the fix's lexical-scope model doesn't
// over-reject legitimate, non-overlapping usage.
#[test]
fn mut_ref_used_normally_still_compiles_and_runs_on_both_backends() {
    use std::fs;
    use std::path::PathBuf;

    let binary = env!("CARGO_BIN_EXE_intentc");
    let src: PathBuf = std::env::temp_dir().join(format!(
        "intentc-mut-ref-normal-{}-{}.vani",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::write(
        &src,
        r#"
fn main() -> i64 {
  let xs: Vec<i64> = vec(1, 2, 3);
  if true {
    let r: mut ref Vec<i64> = mut ref xs;
    push(r, 4);
  }
  print xs[0];
  print len(xs) as i64;
  return 0;
}
"#,
    )
    .expect("write src");

    for backend_args in [
        vec!["run", src.to_str().unwrap()],
        vec!["run", src.to_str().unwrap(), "--backend=c"],
    ] {
        let output = Command::new(binary)
            .args(&backend_args)
            .output()
            .unwrap_or_else(|e| panic!("intentc {:?} should execute: {e}", backend_args));
        assert!(
            output.status.success(),
            "{:?}: status {:?}, stderr: {}",
            backend_args,
            output.status,
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8_lossy(&output.stdout).replace("\r\n", "\n");
        assert_eq!(
            stdout, "1\n4\n",
            "for {:?}; got: {}",
            backend_args, stdout
        );
    }
    let _ = fs::remove_file(&src);
}

// Feature-combination gap audit (2026-08-03), category 1: SIMD as a
// Vec ELEMENT. Two real bugs found and fixed here, one per backend --
// see the matching src/lib.rs test's doc comment for the full root-
// cause writeup. This is the execution-level confirmation: the C
// bug corrupted every generated identifier (a compile-time failure,
// already caught by the lib.rs test), but the LLVM bug was a runtime
// heap corruption from an under-sized malloc/realloc that only a
// real execution (not just successful compilation) can catch.
#[test]
fn vec_of_vec128_example_produces_correct_output_on_both_backends() {
    use std::fs;
    use std::path::PathBuf;

    let binary = env!("CARGO_BIN_EXE_intentc");
    let src: PathBuf = std::env::temp_dir().join(format!(
        "intentc-vec-vec128-{}-{}.vani",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::write(
        &src,
        r#"
fn main() -> i64 {
  let a: vec128<f64> = simd_splat(1.0 as f64);
  let b: vec128<f64> = simd_splat(2.0 as f64);
  let mut_v: Vec<vec128<f64>> = vec(a, b);
  let s: f64 = simd_reduce_add(mut_v[0]);
  print s;
  push(mut ref mut_v, simd_splat(3.0 as f64));
  print len(mut_v) as i64;
  let s2: f64 = simd_reduce_add(mut_v[2]);
  print s2;
  return 0;
}
"#,
    )
    .expect("write src");

    for backend_args in [
        vec!["run", src.to_str().unwrap()],
        vec!["run", src.to_str().unwrap(), "--backend=c"],
    ] {
        let output = Command::new(binary)
            .args(&backend_args)
            .output()
            .unwrap_or_else(|e| panic!("intentc {:?} should execute: {e}", backend_args));
        assert!(
            output.status.success(),
            "{:?}: status {:?}, stderr: {}",
            backend_args,
            output.status,
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8_lossy(&output.stdout).replace("\r\n", "\n");
        assert_eq!(
            stdout, "2\n3\n6\n",
            "for {:?}; got: {}",
            backend_args, stdout
        );
    }
    let _ = fs::remove_file(&src);
}

#[test]
fn array_of_vec128_and_generic_wrapper_vec128_example_produces_correct_output_on_both_backends() {
    use std::fs;
    use std::path::PathBuf;

    let binary = env!("CARGO_BIN_EXE_intentc");
    let src: PathBuf = std::env::temp_dir().join(format!(
        "intentc-array-generic-vec128-{}-{}.vani",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::write(
        &src,
        r#"
struct Wrapper<T> { v: T }
fn main() -> i64 {
  let a: vec128<f64> = simd_splat(1.0 as f64);
  let b: vec128<f64> = simd_splat(2.0 as f64);
  let arr: [vec128<f64>; 2] = [a, b];
  print simd_reduce_add(arr[0]);
  print simd_reduce_add(arr[1]);
  let w: Wrapper<vec128<f64>> = Wrapper { v: a };
  print simd_reduce_add(w.v);
  return 0;
}
"#,
    )
    .expect("write src");

    for backend_args in [
        vec!["run", src.to_str().unwrap()],
        vec!["run", src.to_str().unwrap(), "--backend=c"],
    ] {
        let output = Command::new(binary)
            .args(&backend_args)
            .output()
            .unwrap_or_else(|e| panic!("intentc {:?} should execute: {e}", backend_args));
        assert!(
            output.status.success(),
            "{:?}: status {:?}, stderr: {}",
            backend_args,
            output.status,
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8_lossy(&output.stdout).replace("\r\n", "\n");
        assert_eq!(
            stdout, "2\n4\n2\n",
            "for {:?}; got: {}",
            backend_args, stdout
        );
    }
    let _ = fs::remove_file(&src);
}

// BUG-82 (2026-08-03, same gap-audit pass as the Vec<vec128> tests
// above, LLVM-only). `Result<vec128<f64>, i64>` -- a MIXED-payload-
// type enum -- segfaulted `lli` on both construction and match-arm
// extraction, because neither site's bitcast-through-the-byte-buffer
// load/store had an explicit alignment, so LLVM assumed the SIMD
// payload's natural (16-byte) ABI alignment against a buffer that
// only guarantees 4 bytes. This is a runtime crash a compile-only
// test can't catch (the C backend, and even LLVM compilation itself,
// both succeeded before the fix -- only actually RUNNING the LLVM
// output crashed).
#[test]
fn result_of_vec128_mixed_payload_enum_produces_correct_output_on_both_backends() {
    use std::fs;
    use std::path::PathBuf;

    let binary = env!("CARGO_BIN_EXE_intentc");
    let src: PathBuf = std::env::temp_dir().join(format!(
        "intentc-result-vec128-{}-{}.vani",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::write(
        &src,
        r#"
fn make(flag: bool) -> Result<vec128<f64>, i64> {
  if flag {
    return Result.Ok(simd_splat(4.0 as f64));
  }
  return Result.Err(99);
}
fn main() -> i64 {
  let r1: Result<vec128<f64>, i64> = make(true);
  let r2: Result<vec128<f64>, i64> = make(false);
  let s: f64 = match r1 {
    Result.Ok(v) then simd_reduce_add(v),
    Result.Err(_) then 0.0 as f64,
  };
  let e: i64 = match r2 {
    Result.Ok(_) then -1,
    Result.Err(code) then code,
  };
  print s;
  print e;
  return 0;
}
"#,
    )
    .expect("write src");

    for backend_args in [
        vec!["run", src.to_str().unwrap()],
        vec!["run", src.to_str().unwrap(), "--backend=c"],
    ] {
        let output = Command::new(binary)
            .args(&backend_args)
            .output()
            .unwrap_or_else(|e| panic!("intentc {:?} should execute: {e}", backend_args));
        assert!(
            output.status.success(),
            "{:?}: status {:?}, stderr: {}",
            backend_args,
            output.status,
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8_lossy(&output.stdout).replace("\r\n", "\n");
        assert_eq!(
            stdout, "8\n99\n",
            "for {:?}; got: {}",
            backend_args, stdout
        );
    }
    let _ = fs::remove_file(&src);
}

// BUG-83/BUG-84 (2026-08-03), feature-combination gap audit category
// 2: generics x concurrency handles. See the matching src/lib.rs
// tests' doc comments for the full root-cause writeups:
// BUG-83 -- collect_mutex_specs/collect_rwlock_specs/
// collect_channel_specs never recursed into a struct's OWN field
// types, so a lock used ONLY through a struct field (never a bare
// local elsewhere) never got its bundle emitted; LLVM needed a
// SECOND fix (a cross-backend struct-fields-registry fallback) since
// the C and LLVM backends each populate their own independent copy.
// BUG-84 -- Mutex<bool>'s guard_get/guard_set never converted between
// the i1 (bool) and i8 (byte-addressable storage) representations,
// an LLVM verifier crash for ANY Mutex<bool> at all, not just this
// combination -- but found via the T=bool instantiation of the same
// generic Cache<T> struct used for BUG-83.
#[test]
fn generic_struct_mutex_field_two_instantiations_produce_correct_output_on_both_backends() {
    use std::fs;
    use std::path::PathBuf;

    let binary = env!("CARGO_BIN_EXE_intentc");
    let src: PathBuf = std::env::temp_dir().join(format!(
        "intentc-generic-mutex-cache-{}-{}.vani",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::write(
        &src,
        r#"
struct Cache<T> { lock: Mutex<T> }
fn main() -> i64 {
  let ci: Cache<i64> = Cache { lock: mutex_new(42) };
  let vi: i64 = {
    let gi = mutex_lock(ref ci.lock);
    guard_get(ref gi)
  };
  let cb: Cache<bool> = Cache { lock: mutex_new(true) };
  let vb: bool = {
    let gb = mutex_lock(ref cb.lock);
    guard_get(ref gb)
  };
  print vi;
  print vb;
  return 0;
}
"#,
    )
    .expect("write src");

    for backend_args in [
        vec!["run", src.to_str().unwrap()],
        vec!["run", src.to_str().unwrap(), "--backend=c"],
    ] {
        let output = Command::new(binary)
            .args(&backend_args)
            .output()
            .unwrap_or_else(|e| panic!("intentc {:?} should execute: {e}", backend_args));
        assert!(
            output.status.success(),
            "{:?}: status {:?}, stderr: {}",
            backend_args,
            output.status,
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8_lossy(&output.stdout).replace("\r\n", "\n");
        assert_eq!(
            stdout, "42\ntrue\n",
            "for {:?}; got: {}",
            backend_args, stdout
        );
    }
    let _ = fs::remove_file(&src);
}

// Found by the local-model differential-fuzzing harness (tools/localfuzz/),
// 2026-08-03: a non-ASCII (Devanagari) local variable name crashed the LLVM
// backend but not the C backend. Root cause: local variable/register names
// (`%name.addr`, `%arg_name`, etc.) were built directly from the raw source
// identifier at ~12 call sites in backend_llvm.rs, unlike global function
// symbols (`@fn_name`), which already routed through the existing
// `llvm_mangle_ident` helper (non-ASCII chars -> `_uHHHH` hex escapes,
// producing a valid *bare* LLVM identifier). Fixed by routing all ~12 local
// binding sites through the same helper. Confirmed general across three
// unrelated scripts (Devanagari, Hangul, Cyrillic), not Nepali-specific --
// the checker's lexer explicitly supports arbitrary Unicode identifiers
// (see `lex_unicode_ident` in lexer.rs), so this could affect any dialect.
//
// NOT fixed in the same pass: struct/enum TYPE names (`%Struct_<Name>`,
// `%Enum_<Name>`) have the same unmangled-identifier gap, at ~28 separate
// call sites with no shared helper to fix centrally, and no confirmed
// crashing repro (existing dialect examples all use Latin struct/fn names
// even when using native-script local variables). Logged, not fixed --
// needs its own dedicated pass; see docs/TODO_CURRENT.md.
#[test]
fn non_ascii_local_variable_name_produces_correct_output_on_both_backends() {
    use std::fs;
    use std::path::PathBuf;

    let binary = env!("CARGO_BIN_EXE_intentc");

    // (source, expected stdout) pairs -- one per script, each using a
    // non-ASCII local variable name in an otherwise-trivial program.
    let cases: [(&str, &str); 3] = [
        // Devanagari (Nepali dialect) -- the original repro.
        (
            r#"
उद्देश्य "Devanagari local variable identifier smoke-test";

कार्य main() -> i64 {
  माना थैला: i64 = 41;
  लिखो थैला + 1;
  लौटाओ 0;
}
"#,
            "42\n",
        ),
        // Hangul (Korean dialect).
        (
            r#"
목적 "Korean local variable identifier smoke-test";

함수 main() -> i64 {
  정의 숫자: i64 = 41;
  확인 숫자 >= 0;
  반환 0;
}
"#,
            "",
        ),
        // Cyrillic, English keywords -- proves this isn't tied to any one
        // dialect's keyword set, just the identifier itself.
        (
            r#"
intent "Cyrillic local variable identifier smoke-test";

fn main() -> i64 {
  let число: i64 = 41;
  print число + 1;
  return 0;
}
"#,
            "42\n",
        ),
    ];

    for (i, (source, expected_stdout)) in cases.iter().enumerate() {
        let src: PathBuf = std::env::temp_dir().join(format!(
            "intentc-non-ascii-local-{}-{}-{}.vani",
            i,
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::write(&src, source).expect("write src");

        for backend_args in [
            vec!["run", src.to_str().unwrap()],
            vec!["run", src.to_str().unwrap(), "--backend=c"],
        ] {
            let output = Command::new(binary)
                .args(&backend_args)
                .output()
                .unwrap_or_else(|e| panic!("intentc {:?} should execute: {e}", backend_args));
            assert!(
                output.status.success(),
                "case {i}, {:?}: status {:?}, stderr: {}",
                backend_args,
                output.status,
                String::from_utf8_lossy(&output.stderr)
            );
            let stdout = String::from_utf8_lossy(&output.stdout).replace("\r\n", "\n");
            assert_eq!(
                stdout, *expected_stdout,
                "case {i}, for {:?}; got: {}",
                backend_args, stdout
            );
        }
        let _ = fs::remove_file(&src);
    }
}

// BUG-85/BUG-86 (2026-08-03), feature-combination gap audit category
// 3. See the matching src/lib.rs tests' doc comments for the full
// root-cause writeups. BUG-85 (SSA-C only): a bare, SSA-eligible
// Mutex<i64> failed to compile at all (stale hardcoded naming).
// BUG-86 (tree-C only): once BUG-85 was fixed, a program with two
// SEQUENTIAL, non-overlapping lock/unlock cycles on the same mutex
// (through a block-expression, the tutorial's own idiom) HUNG
// FOREVER on the second lock -- a real, silent deadlock, not a
// compile error. This test wraps the invocation in the `timeout`
// command specifically so a FUTURE regression of BUG-86 fails this
// test (and CI) instead of hanging the test suite itself forever.
#[test]
fn sequential_mutex_lock_unlock_via_block_expr_does_not_deadlock_on_either_backend() {
    use std::fs;
    use std::path::PathBuf;

    let binary = env!("CARGO_BIN_EXE_intentc");
    let src: PathBuf = std::env::temp_dir().join(format!(
        "intentc-mutex-sequential-{}-{}.vani",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::write(
        &src,
        r#"
fn main() -> i64 {
  let m: Mutex<i64> = mutex_new(0);
  let v1: i64 = {
    let g = mutex_lock(ref m);
    guard_get(ref g)
  };
  {
    let g = mutex_lock(ref m);
    guard_set(mut ref g, 99);
  }
  let v2: i64 = {
    let g = mutex_lock(ref m);
    guard_get(ref g)
  };
  print v1;
  print v2;
  return 0;
}
"#,
    )
    .expect("write src");

    for backend_args in [
        vec!["run", src.to_str().unwrap()],
        vec!["run", src.to_str().unwrap(), "--backend=c"],
    ] {
        // Wrapped in the real `timeout` command: if a future
        // regression reintroduces the deadlock, this test fails
        // (non-zero / killed exit) after 20s instead of hanging the
        // whole suite (and CI) forever.
        let mut cmd_args = vec!["20", binary];
        cmd_args.extend(backend_args.iter().copied());
        let output = Command::new("timeout")
            .args(&cmd_args)
            .output()
            .unwrap_or_else(|e| panic!("timeout+intentc {:?} should execute: {e}", backend_args));
        assert!(
            output.status.success(),
            "{:?}: status {:?} (124 = timeout/deadlock), stderr: {}",
            backend_args,
            output.status,
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8_lossy(&output.stdout).replace("\r\n", "\n");
        assert_eq!(
            stdout, "0\n99\n",
            "for {:?}; got: {}",
            backend_args, stdout
        );
    }
    let _ = fs::remove_file(&src);
}

#[test]
fn tutorial_verbatim_mutex_example_produces_correct_output_on_both_backends() {
    use std::fs;
    use std::path::PathBuf;

    let binary = env!("CARGO_BIN_EXE_intentc");
    let src: PathBuf = std::env::temp_dir().join(format!(
        "intentc-mutex-tutorial-verbatim-{}-{}.vani",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::write(
        &src,
        r#"
fn main() -> i64 {
  let m: Mutex<i64> = mutex_new(0);
  {
    let g: Guard<i64> = mutex_lock(ref m);
    guard_set(mut ref g, 42);
    let v: i64 = guard_get(ref g);
    print v;
  }
  return 0;
}
"#,
    )
    .expect("write src");

    for backend_args in [
        vec!["run", src.to_str().unwrap()],
        vec!["run", src.to_str().unwrap(), "--backend=c"],
    ] {
        let output = Command::new(binary)
            .args(&backend_args)
            .output()
            .unwrap_or_else(|e| panic!("intentc {:?} should execute: {e}", backend_args));
        assert!(
            output.status.success(),
            "{:?}: status {:?}, stderr: {}",
            backend_args,
            output.status,
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8_lossy(&output.stdout).replace("\r\n", "\n");
        assert_eq!(
            stdout, "42\n",
            "for {:?}; got: {}",
            backend_args, stdout
        );
    }
    let _ = fs::remove_file(&src);
}

// BUG-89 (2026-08-03), feature-combination gap audit category 5: dyn
// dispatch x generics. `Vec<dyn Iface>` holding two different
// monomorphizations of the same blanket-impl'd generic struct crashed
// both backends because `expand_blanket_impls` never removed the
// original blanket impl template from `program.impls`, so vtable
// generation produced a bogus extra trampoline for the unresolved
// template. See the matching src/lib.rs test's doc comment for the
// full root-cause writeup.
#[test]
fn vec_of_dyn_iface_two_blanket_impl_monomorphizations_produces_correct_output_on_both_backends() {
    use std::fs;
    use std::path::PathBuf;

    let binary = env!("CARGO_BIN_EXE_intentc");
    let src: PathBuf = std::env::temp_dir().join(format!(
        "intentc-dyn-blanket-mono-{}-{}.vani",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::write(
        &src,
        r#"
interface Printable {
  fn print_it(self: Self) -> i64;
}
struct Wrapper<T> { inner: T }
implement<T> Printable for Wrapper<T> where T is Printable {
  fn print_it(self: Wrapper<T>) -> i64 {
    return self.inner.print_it();
  }
}
struct Dog { name: i64 }
implement Printable for Dog {
  fn print_it(self: Dog) -> i64 { return 111; }
}
struct Cat { name: i64 }
implement Printable for Cat {
  fn print_it(self: Cat) -> i64 { return 222; }
}
fn main() -> i64 {
  let wd: Wrapper<Dog> = Wrapper { inner: Dog { name: 1 } };
  let wc: Wrapper<Cat> = Wrapper { inner: Cat { name: 2 } };
  let items: Vec<dyn Printable> = vec(wd as dyn Printable, wc as dyn Printable);
  let i: u64 = 0;
  while i < len(items) {
    print items[i].print_it();
    i = i + 1;
  }
  return 0;
}
"#,
    )
    .expect("write src");

    for backend_args in [
        vec!["run", src.to_str().unwrap()],
        vec!["run", src.to_str().unwrap(), "--backend=c"],
    ] {
        let output = Command::new(binary)
            .args(&backend_args)
            .output()
            .unwrap_or_else(|e| panic!("intentc {:?} should execute: {e}", backend_args));
        assert!(
            output.status.success(),
            "{:?}: status {:?}, stderr: {}",
            backend_args,
            output.status,
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8_lossy(&output.stdout).replace("\r\n", "\n");
        assert_eq!(
            stdout, "111\n222\n",
            "for {:?}; got: {}",
            backend_args, stdout
        );
    }
    let _ = fs::remove_file(&src);
}

// BUG-90 (2026-08-03), feature-combination gap audit category 6:
// try/? x containers/generics. `try EXPR` calling a GENERIC function
// failed with "generic function 'wrap' is declared but never called
// with concrete types" -- four compounding gaps in the generics/
// monomorphization pipeline, all the same "sibling walker never
// learned about a syntax-sugar shape" pattern this session kept
// hitting: `collect_generic_calls_in_expr` and its sibling
// `rewrite_generic_calls_in_expr` had no `Match`/`Block` arms (the
// try-desugar runs BEFORE fn-generics monomorphization, so every
// `try wrap(n)` is already a `Match{scrutinee: Call(wrap,...)}` by
// the time these walkers run); `substitute_type_param`'s
// `Type::Apply` collapse always produced `Type::Struct`, never
// `Type::Enum`, for a newly-concrete generic return type; and
// `collect_apply_in_stmt`/`rewrite_apply_in_stmt` never recursed
// into `Return`/`Assign` exprs to find the try-desugar's nested
// synthesized `Let` annotations. See the matching src/lib.rs tests'
// doc comments for the full root-cause writeup.
#[test]
fn try_inside_generic_function_call_produces_correct_output_on_both_backends() {
    use std::fs;
    use std::path::PathBuf;

    let binary = env!("CARGO_BIN_EXE_intentc");
    let src: PathBuf = std::env::temp_dir().join(format!(
        "intentc-try-generic-fn-{}-{}.vani",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::write(
        &src,
        r#"
fn lookup(x: i64) -> Option<i64> {
  if x < 0 { return Option.None; }
  return Option.Some(x * 2);
}
fn wrap<T>(x: T) -> Option<T> {
  return Option.Some(x);
}
fn compute(n: i64) -> Option<i64> {
  let v: i64 = try lookup(n);
  let w: i64 = try wrap(v);
  return Option.Some(w + 1);
}
fn main() -> i64 {
  let r1: i64 = match compute(5) {
    Option.Some(x) then x,
    Option.None then -1,
  };
  let r2: i64 = match compute(0 - 1) {
    Option.Some(x) then x,
    Option.None then -1,
  };
  print r1;
  print r2;
  return 0;
}
"#,
    )
    .expect("write src");

    for backend_args in [
        vec!["run", src.to_str().unwrap()],
        vec!["run", src.to_str().unwrap(), "--backend=c"],
    ] {
        let output = Command::new(binary)
            .args(&backend_args)
            .output()
            .unwrap_or_else(|e| panic!("intentc {:?} should execute: {e}", backend_args));
        assert!(
            output.status.success(),
            "{:?}: status {:?}, stderr: {}",
            backend_args,
            output.status,
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8_lossy(&output.stdout).replace("\r\n", "\n");
        assert_eq!(
            stdout, "11\n-1\n",
            "for {:?}; got: {}",
            backend_args, stdout
        );
    }
    let _ = fs::remove_file(&src);
}

// BUG-90 continued: category 6 row 3, `try` propagating through a
// nested `Option<Result<T, E>>`. Needed the fourth sub-fix above
// (the nested-Let-annotation walk gap in collect_apply_in_stmt /
// rewrite_apply_in_stmt).
#[test]
fn try_propagates_through_nested_option_result_enum_on_both_backends() {
    use std::fs;
    use std::path::PathBuf;

    let binary = env!("CARGO_BIN_EXE_intentc");
    let src: PathBuf = std::env::temp_dir().join(format!(
        "intentc-try-nested-option-result-{}-{}.vani",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::write(
        &src,
        r#"
fn safe_div(a: i64, b: i64) -> Result<i64, i64> {
  if b == 0 { return Result.Err(-1); }
  return Result.Ok(a / b);
}
fn lookup(x: i64) -> Option<Result<i64, i64>> {
  if x < 0 { return Option.None; }
  return Option.Some(safe_div(100, x));
}
fn compute(x: i64) -> Option<Result<i64, i64>> {
  let r: Result<i64, i64> = try lookup(x);
  return Option.Some(r);
}
fn main() -> i64 {
  let r1: Result<i64, i64> = match compute(4) {
    Option.Some(v) then v,
    Option.None then Result.Err(-99),
  };
  let out1: i64 = match r1 {
    Result.Ok(v) then v,
    Result.Err(e) then e,
  };
  let r2: Result<i64, i64> = match compute(0 - 1) {
    Option.Some(v) then v,
    Option.None then Result.Err(-99),
  };
  let out2: i64 = match r2 {
    Result.Ok(v) then v,
    Result.Err(e) then e,
  };
  print out1;
  print out2;
  return 0;
}
"#,
    )
    .expect("write src");

    for backend_args in [
        vec!["run", src.to_str().unwrap()],
        vec!["run", src.to_str().unwrap(), "--backend=c"],
    ] {
        let output = Command::new(binary)
            .args(&backend_args)
            .output()
            .unwrap_or_else(|e| panic!("intentc {:?} should execute: {e}", backend_args));
        assert!(
            output.status.success(),
            "{:?}: status {:?}, stderr: {}",
            backend_args,
            output.status,
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8_lossy(&output.stdout).replace("\r\n", "\n");
        assert_eq!(
            stdout, "25\n-99\n",
            "for {:?}; got: {}",
            backend_args, stdout
        );
    }
    let _ = fs::remove_file(&src);
}

// Category 6 row 1: `try`/`?` inside a function holding a live
// LOCAL `Vec<Struct>` binding across the early-return path. Checked
// clean (not a bug); regression-tested for output correctness here.
// Verified separately with `valgrind --leak-check=full` on native
// AOT builds of both backends: 0 errors, all heap blocks freed.
#[test]
fn try_with_live_local_vec_struct_across_early_return_on_both_backends() {
    use std::fs;
    use std::path::PathBuf;

    let binary = env!("CARGO_BIN_EXE_intentc");
    let src: PathBuf = std::env::temp_dir().join(format!(
        "intentc-try-vec-struct-dropseq-{}-{}.vani",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::write(
        &src,
        r#"
struct Item { id: i64, tag: OwnedStr }
fn find_positive(x: i64) -> Option<i64> {
  if x < 0 { return Option.None; }
  return Option.Some(x * 2);
}
fn compute(n: i64) -> Option<i64> {
  let items: Vec<Item> = vec(
    Item { id: 1, tag: "a" + "" },
    Item { id: 2, tag: "b" + "" },
  );
  let doubled: i64 = try find_positive(n);
  let first: Item = clone_at(ref items, 0);
  let second: Item = clone_at(ref items, 1);
  let total: i64 = doubled + first.id + second.id;
  return Option.Some(total);
}
fn main() -> i64 {
  let r1: i64 = match compute(5) {
    Option.Some(x) then x,
    Option.None then -1,
  };
  let r2: i64 = match compute(0 - 5) {
    Option.Some(x) then x,
    Option.None then -1,
  };
  print r1;
  print r2;
  return 0;
}
"#,
    )
    .expect("write src");

    for backend_args in [
        vec!["run", src.to_str().unwrap()],
        vec!["run", src.to_str().unwrap(), "--backend=c"],
    ] {
        let output = Command::new(binary)
            .args(&backend_args)
            .output()
            .unwrap_or_else(|e| panic!("intentc {:?} should execute: {e}", backend_args));
        assert!(
            output.status.success(),
            "{:?}: status {:?}, stderr: {}",
            backend_args,
            output.status,
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8_lossy(&output.stdout).replace("\r\n", "\n");
        assert_eq!(
            stdout, "13\n-1\n",
            "for {:?}; got: {}",
            backend_args, stdout
        );
    }
    let _ = fs::remove_file(&src);
}

// Feature-combination gap audit (2026-08-03), category 7 row 1:
// iterator-style Vec builtins chained together via named `let`s
// between each step (the v1-supported pattern -- direct one-
// expression chaining is rejected per docs, see the matching
// src/lib.rs test). Verifies actual output VALUES, not just that
// it compiles.
#[test]
fn iterator_combinators_chained_via_named_lets_produce_correct_output_on_both_backends() {
    use std::fs;
    use std::path::PathBuf;

    let binary = env!("CARGO_BIN_EXE_intentc");
    let src: PathBuf = std::env::temp_dir().join(format!(
        "intentc-iter-combinator-chain-{}-{}.vani",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::write(
        &src,
        r#"
fn is_even(x: i64) -> bool { return x % 2 == 0; }
fn double_it(x: i64) -> i64 { return x * 2; }
fn add(a: i64, b: i64) -> i64 { return a + b; }
fn mul(a: i64, b: i64) -> i64 { return a * b; }
fn main() -> i64 {
  let xs: Vec<i64> = vec(1, 2, 3, 4, 5, 6, 7, 8, 9, 10);
  let ys: Vec<i64> = vec(10, 20, 30, 40, 50, 60, 70, 80, 90, 100);
  let evens: Vec<i64> = xs.filter(is_even);
  let doubled: Vec<i64> = evens.map(double_it);
  let total: i64 = doubled.fold(0, add);
  print total;
  let zipped: Vec<i64> = vec_zip_with(ref xs, ref ys, mul);
  let zipped_evens: Vec<i64> = zipped.filter(is_even);
  let zipped_sum: i64 = zipped_evens.fold(0, add);
  print zipped_sum;
  let taken: Vec<i64> = doubled.take(2);
  print taken[0];
  print taken[1];
  return 0;
}
"#,
    )
    .expect("write src");

    for backend_args in [
        vec!["run", src.to_str().unwrap()],
        vec!["run", src.to_str().unwrap(), "--backend=c"],
    ] {
        let output = Command::new(binary)
            .args(&backend_args)
            .output()
            .unwrap_or_else(|e| panic!("intentc {:?} should execute: {e}", backend_args));
        assert!(
            output.status.success(),
            "{:?}: status {:?}, stderr: {}",
            backend_args,
            output.status,
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8_lossy(&output.stdout).replace("\r\n", "\n");
        assert_eq!(
            stdout, "60\n3850\n4\n8\n",
            "for {:?}; got: {}",
            backend_args, stdout
        );
    }
    let _ = fs::remove_file(&src);
}

// Feature-combination gap audit (2026-08-03), category 7 row 2:
// `task`/`join` call-form with a genuinely multi-block callee body
// (nested if/else inside a while loop) -- main.rs flags multi-block
// task bodies as an SSA-LLVM-reject/tree-LLVM-fallback edge case.
// Verified by hand: worker(10) = 37, worker(20) = 107.
#[test]
fn task_join_callform_multiblock_body_produces_correct_output_on_both_backends() {
    use std::fs;
    use std::path::PathBuf;

    let binary = env!("CARGO_BIN_EXE_intentc");
    let src: PathBuf = std::env::temp_dir().join(format!(
        "intentc-task-multiblock-{}-{}.vani",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::write(
        &src,
        r#"
fn worker(n: i64) -> i64 {
  let acc: i64 = 0;
  let i: i64 = 0;
  while i < n {
    if i % 3 == 0 {
      acc = acc + i * 2;
    } else {
      if i % 2 == 0 {
        acc = acc + i;
      } else {
        acc = acc - i;
      }
    }
    i = i + 1;
  }
  return acc;
}
fn main() -> i64 {
  let t1: Task<i64> = task worker(10);
  let t2: Task<i64> = task worker(20);
  let r1: i64 = join t1;
  let r2: i64 = join t2;
  print r1;
  print r2;
  return 0;
}
"#,
    )
    .expect("write src");

    for backend_args in [
        vec!["run", src.to_str().unwrap()],
        vec!["run", src.to_str().unwrap(), "--backend=c"],
    ] {
        let output = Command::new(binary)
            .args(&backend_args)
            .output()
            .unwrap_or_else(|e| panic!("intentc {:?} should execute: {e}", backend_args));
        assert!(
            output.status.success(),
            "{:?}: status {:?}, stderr: {}",
            backend_args,
            output.status,
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8_lossy(&output.stdout).replace("\r\n", "\n");
        assert_eq!(
            stdout, "37\n107\n",
            "for {:?}; got: {}",
            backend_args, stdout
        );
    }
    let _ = fs::remove_file(&src);
}

// Feature-combination gap audit (2026-08-03), category 7 row 4:
// Graph/Bst/Trie/SkipList/UnionFind/BloomFilter actually running
// end-to-end together, matching advanced/05b_advanced_collections.md's
// own documented expected values for every call.
#[test]
fn advanced_collections_run_correctly_on_both_backends() {
    use std::fs;
    use std::path::PathBuf;

    let binary = env!("CARGO_BIN_EXE_intentc");
    let src: PathBuf = std::env::temp_dir().join(format!(
        "intentc-advanced-collections-{}-{}.vani",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::write(
        &src,
        r#"
fn main() -> i64 {
  let g: Graph = graph_new(5);
  let _ = g.add_edge(0, 1, 4);
  let _ = g.add_edge(0, 2, 1);
  let _ = g.add_edge(2, 1, 2);
  let _ = g.add_edge(1, 3, 1);
  let _ = g.add_edge(3, 4, 3);
  print g.num_nodes();
  print g.num_edges();
  print g.bfs_reach(0);
  print g.dfs_reach(0);
  let dist: Option<i64> = g.dijkstra(0, 4);
  print option_unwrap_or(dist, -1);

  let b: Bst<i64> = bst_new();
  let _ = b.insert(5);
  let _ = b.insert(3);
  let _ = b.insert(7);
  let _ = b.insert(1);
  print b.contains(3);
  print b.contains(6);
  print b.len();
  print option_unwrap_or(b.min(), -1);
  print option_unwrap_or(b.max(), -1);
  let _ = b.remove(3);
  print b.len();

  let t: Trie = trie_new();
  let _ = t.insert("hello");
  let _ = t.insert("help");
  let _ = t.insert("world");
  print t.contains("hello");
  print t.contains("hell");
  print t.starts_with("hel");
  print t.starts_with("wor");
  print t.len();

  let sl: SkipList = skiplist_new();
  let _ = sl.insert(10);
  let _ = sl.insert(5);
  let _ = sl.insert(20);
  let _ = sl.insert(5);
  print sl.len();
  print sl.contains(5);
  print sl.contains(7);
  print option_unwrap_or(sl.min(), -1);
  print option_unwrap_or(sl.max(), -1);

  let uf: UnionFind = union_find_new(6);
  let _ = union_find_union(mut ref uf, 0, 1);
  let _ = union_find_union(mut ref uf, 1, 2);
  let _ = union_find_union(mut ref uf, 3, 4);
  print union_find_count(ref uf);
  print union_find_connected(mut ref uf, 0, 2);
  print union_find_connected(mut ref uf, 0, 3);

  let bf: BloomFilter = bloom_filter_new(1024, 4);
  let _ = bf.insert(42);
  let _ = bf.insert(100);
  let _ = bf.insert(7);
  print bf.contains(42);
  print bf.contains(99);
  print bf.len();
  print bf.count();
  return 0;
}
"#,
    )
    .expect("write src");

    let expected = "5\n5\n5\n5\n7\ntrue\nfalse\n4\n1\n7\n3\ntrue\nfalse\ntrue\ntrue\n3\n3\ntrue\nfalse\n5\n20\n3\ntrue\nfalse\ntrue\nfalse\n1024\n3\n";

    for backend_args in [
        vec!["run", src.to_str().unwrap()],
        vec!["run", src.to_str().unwrap(), "--backend=c"],
    ] {
        let output = Command::new(binary)
            .args(&backend_args)
            .output()
            .unwrap_or_else(|e| panic!("intentc {:?} should execute: {e}", backend_args));
        assert!(
            output.status.success(),
            "{:?}: status {:?}, stderr: {}",
            backend_args,
            output.status,
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8_lossy(&output.stdout).replace("\r\n", "\n");
        assert_eq!(
            stdout, expected,
            "for {:?}; got: {}",
            backend_args, stdout
        );
    }
    let _ = fs::remove_file(&src);
}

// Feature-combination gap audit (2026-08-03), category 7 row 3
// neighborhood (the BUG-44 `--target`/`no_std`/`#[no_mangle]`
// intersection was re-audited for OTHER bugs nearby): found that
// `examples/language/english/bare_metal.vani` -- the EXACT shipped
// example BUG-44 fixed -- had never actually been run through
// `vanic build`/`vanic run` (only its emitted TEXT was grepped for
// the bare symbol name). Doing so crashes `opt`/`llc` with ill-typed
// IR from `mmio_read_u8`/`mmio_read_u16` (internally zext'd their
// narrow load to i64, contradicting the narrow-stays-narrow-until-
// cast convention SSA-LLVM already follows -- storing that i64 value
// into a `u8`/`u16`-typed `let`'s i8/i16 alloca produced "defined
// with type 'i64' but expected 'i8/i16'"). Fixing the read side
// exposed a SECOND bug in the same builtin family: `mmio_write_u8`/
// `mmio_write_u16` unconditionally emitted `trunc i64 {val} to i8/
// i16` assuming `val` was always i64-typed, which is wrong whenever
// the value being written is ALREADY narrow (any `u8`/`u16`
// parameter or local, or `mmio_read_u8/u16` after the first fix) --
// "defined with type 'i8' but expected 'i64'" the other way around.
// Both only affect tree-LLVM (reached whenever a program contains a
// `#[no_mangle]` fn anywhere, which routes the WHOLE program there
// per BUG-44's own fix) -- SSA-LLVM already had the correct
// convention for all four builtins, used as the reference here.
// This test builds+links+runs (full opt/llc/cc pipeline) a program
// combining all four builtins with a `#[no_mangle]` fn to force
// tree-LLVM routing; `valgrind --leak-check=full` on the resulting
// native binary: 0 errors.
#[test]
fn mmio_narrow_read_write_builtins_build_and_run_correctly_under_no_mangle() {
    use std::fs;
    use std::path::PathBuf;

    let binary = env!("CARGO_BIN_EXE_intentc");
    let dir = std::env::temp_dir();
    let src: PathBuf = dir.join(format!(
        "intentc-mmio-narrow-{}-{}.vani",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::write(
        &src,
        r#"
#[no_mangle]
fn dummy_export() -> i64 {
  return 0;
}
fn uart_tx_ready() -> bool {
  let sr: u16 = mmio_read_u16(0x40011000);
  return (sr as i64) & 0x80 != 0;
}
fn uart_send(byte: u8) -> i64 {
  let _ = mmio_write_u8(0x40011004, byte);
  return 0;
}
fn set_ctrl_reg(v: u16) -> i64 {
  let _ = mmio_write_u16(0x40011008, v);
  return 0;
}
fn main() -> i64 {
  print "ok";
  return 0;
}
"#,
    )
    .expect("write src");

    let bin_path = dir.join(format!(
        "intentc-mmio-narrow-bin-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let build = Command::new(binary)
        .args([
            "build",
            src.to_str().unwrap(),
            "-lm",
            "-o",
            bin_path.to_str().unwrap(),
        ])
        .output()
        .expect("intentc build runs");
    assert!(
        build.status.success(),
        "vanic build failed (mmio narrow read/write under #[no_mangle]):\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&build.stdout),
        String::from_utf8_lossy(&build.stderr),
    );
    let run = Command::new(&bin_path).output().expect("binary runs");
    assert!(
        run.status.success(),
        "binary exited non-zero: {:?} (stdout: {}, stderr: {})",
        run.status,
        String::from_utf8_lossy(&run.stdout),
        String::from_utf8_lossy(&run.stderr),
    );
    assert_eq!(
        String::from_utf8_lossy(&run.stdout).replace("\r\n", "\n"),
        "ok\n",
    );
    let _ = fs::remove_file(&src);
    let _ = fs::remove_file(&bin_path);
}

// Feature-combination gap audit (2026-08-03), category 8 row 1:
// `extern "C"` fn taking/returning a MONOMORPHIZED GENERIC struct
// (`Wrapper<i32>`, mangled to `Wrapper__i32`) by value, verified
// against a REAL linked C shim on both backends (matching BUG-77's
// verification style, extended to a generic struct's monomorphized
// shape). 3 + 4 = 7.
#[test]
fn extern_c_monomorphized_generic_struct_by_value_runs_correctly_on_both_backends() {
    use std::fs;

    let src = write_tmp_vani(
        "extern_generic_struct_by_value",
        r#"
struct Wrapper<T> { x: T, y: T }
extern "C" fn make_wrapper(x: i32, y: i32) -> Wrapper<i32>;
extern "C" fn wrapper_sum(w: Wrapper<i32>) -> i32;
fn main() -> i64 {
  let w: Wrapper<i32> = make_wrapper(3 as i32, 4 as i32);
  let s: i32 = wrapper_sum(w);
  print s as i64;
  return 0;
}
"#,
    );
    let dir = src.parent().unwrap().to_path_buf();
    let shim_c = dir.join("wrapper_shim.c");
    fs::write(
        &shim_c,
        "#include <stdint.h>\n\
         typedef struct { int32_t x; int32_t y; } Wrapper_i32;\n\
         Wrapper_i32 make_wrapper(int32_t x, int32_t y) { Wrapper_i32 w; w.x = x; w.y = y; return w; }\n\
         int32_t wrapper_sum(Wrapper_i32 w) { return w.x + w.y; }\n",
    )
    .expect("write wrapper_shim.c");

    let binary = env!("CARGO_BIN_EXE_intentc");

    let llvm_bin = dir.join("wrapper_prog_llvm");
    let build = Command::new(binary)
        .args([
            "build",
            src.to_str().unwrap(),
            "--link-with",
            shim_c.to_str().unwrap(),
            "-lm",
            "-o",
            llvm_bin.to_str().unwrap(),
        ])
        .output()
        .expect("intentc build runs");
    assert!(
        build.status.success(),
        "LLVM build --link-with failed:\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&build.stdout),
        String::from_utf8_lossy(&build.stderr),
    );
    let run_llvm = Command::new(&llvm_bin).output().expect("LLVM binary runs");
    assert!(
        run_llvm.status.success(),
        "LLVM binary exited non-zero: {:?} (stdout: {}, stderr: {})",
        run_llvm.status,
        String::from_utf8_lossy(&run_llvm.stdout),
        String::from_utf8_lossy(&run_llvm.stderr),
    );
    assert_eq!(
        String::from_utf8_lossy(&run_llvm.stdout).replace("\r\n", "\n"),
        "7\n",
        "LLVM backend output mismatch"
    );

    let run_c = Command::new(binary)
        .args([
            "run",
            src.to_str().unwrap(),
            "--backend=c",
            "--link-with",
            shim_c.to_str().unwrap(),
        ])
        .output()
        .expect("intentc run --backend=c --link-with runs");
    assert!(
        run_c.status.success(),
        "C backend run --link-with failed:\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&run_c.stdout),
        String::from_utf8_lossy(&run_c.stderr),
    );
    assert_eq!(
        String::from_utf8_lossy(&run_c.stdout).replace("\r\n", "\n"),
        "7\n",
        "C backend output mismatch"
    );

    let _ = fs::remove_dir_all(&dir);
}

// Feature-combination gap audit (2026-08-03), category 8 row 3: the
// documented escape hatch (`pure extern "C" fn`) for calling foreign
// code inside a `task` body genuinely works end-to-end, on both
// backends, against a real linked C shim.
#[test]
fn pure_extern_c_call_inside_task_body_runs_correctly_on_both_backends() {
    use std::fs;

    let src = write_tmp_vani(
        "pure_extern_in_task",
        r#"
pure extern "C" fn c_add(a: i64, b: i64) -> i64;
fn main() -> i64 {
  task worker {
    let x: i64 = c_add(3, 4);
  }
  join worker;
  print "done";
  return 0;
}
"#,
    );
    let dir = src.parent().unwrap().to_path_buf();
    let shim_c = dir.join("cadd_shim.c");
    fs::write(
        &shim_c,
        "#include <stdint.h>\nint64_t c_add(int64_t a, int64_t b) { return a + b; }\n",
    )
    .expect("write cadd_shim.c");

    let binary = env!("CARGO_BIN_EXE_intentc");

    let llvm_bin = dir.join("pure_extern_task_llvm");
    let build = Command::new(binary)
        .args([
            "build",
            src.to_str().unwrap(),
            "--link-with",
            shim_c.to_str().unwrap(),
            "-lm",
            "-o",
            llvm_bin.to_str().unwrap(),
        ])
        .output()
        .expect("intentc build runs");
    assert!(
        build.status.success(),
        "LLVM build --link-with failed:\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&build.stdout),
        String::from_utf8_lossy(&build.stderr),
    );
    let run_llvm = Command::new(&llvm_bin).output().expect("LLVM binary runs");
    assert!(
        run_llvm.status.success(),
        "LLVM binary exited non-zero: {:?} (stdout: {}, stderr: {})",
        run_llvm.status,
        String::from_utf8_lossy(&run_llvm.stdout),
        String::from_utf8_lossy(&run_llvm.stderr),
    );
    assert_eq!(
        String::from_utf8_lossy(&run_llvm.stdout).replace("\r\n", "\n"),
        "done\n",
        "LLVM backend output mismatch"
    );

    let run_c = Command::new(binary)
        .args([
            "run",
            src.to_str().unwrap(),
            "--backend=c",
            "--link-with",
            shim_c.to_str().unwrap(),
        ])
        .output()
        .expect("intentc run --backend=c --link-with runs");
    assert!(
        run_c.status.success(),
        "C backend run --link-with failed:\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&run_c.stdout),
        String::from_utf8_lossy(&run_c.stderr),
    );
    assert_eq!(
        String::from_utf8_lossy(&run_c.stdout).replace("\r\n", "\n"),
        "done\n",
        "C backend output mismatch"
    );

    let _ = fs::remove_dir_all(&dir);
}

// BUG-93 (2026-08-03), feature-combination gap audit category 9 row
// 2: a recursive GENERIC struct (`struct Node<T> { value: T, next:
// Option<Box<Node<T>>> }`) failed to compile at all -- five
// compounding gaps in the generics-monomorphization pipeline (four
// missing Type::Box arms across four separate "walk a Type for
// nested Apply" copies, plus a single-pass-instead-of-fixed-point
// generation loop that silently discarded newly-discovered needs
// from a freshly-monomorphized struct's own fields). See the
// matching src/lib.rs test's doc comment for the full root-cause
// writeup, including three separate deferred findings surfaced
// along the way (enum-ctor-in-struct-literal ambiguity with a
// working workaround, no field access through a bare Box<T>, and a
// pre-existing C-backend memory leak in the already-shipped BUG-35
// example independent of generics).
#[test]
fn recursive_generic_struct_node_produces_correct_output_on_both_backends() {
    use std::fs;
    use std::path::PathBuf;

    let binary = env!("CARGO_BIN_EXE_intentc");
    let src: PathBuf = std::env::temp_dir().join(format!(
        "intentc-recursive-generic-node-{}-{}.vani",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::write(
        &src,
        r#"
struct Node<T> { value: T, next: Option<Box<Node<T>>> }
fn main() -> i64 {
  let tail: Node<i64> = Node { value: 3, next: Option.None };
  let tail_next: Option<Box<Node<i64>>> = Option.Some(box(tail));
  let mid: Node<i64> = Node { value: 2, next: tail_next };
  let mid_next: Option<Box<Node<i64>>> = Option.Some(box(mid));
  let head: Node<i64> = Node { value: 1, next: mid_next };
  print head.value;
  return 0;
}
"#,
    )
    .expect("write src");

    for backend_args in [
        vec!["run", src.to_str().unwrap()],
        vec!["run", src.to_str().unwrap(), "--backend=c"],
    ] {
        let output = Command::new(binary)
            .args(&backend_args)
            .output()
            .unwrap_or_else(|e| panic!("intentc {:?} should execute: {e}", backend_args));
        assert!(
            output.status.success(),
            "{:?}: status {:?}, stderr: {}",
            backend_args,
            output.status,
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8_lossy(&output.stdout).replace("\r\n", "\n");
        assert_eq!(
            stdout, "1\n",
            "for {:?}; got: {}",
            backend_args, stdout
        );
    }
    let _ = fs::remove_file(&src);
}

// BUG-95 (2026-08-03). Direct-struct-literal variant of the BUG-93
// test above -- same recursive generic struct `Node<T>`, but every
// `Option.Some(box(...))` enum constructor is written INLINE inside
// the enclosing struct literal's field, with no intermediate `let`
// to give the monomorphizer's discovery walk a literal, un-collapsed
// `Type::Apply` node to find. See the matching src/lib.rs test's doc
// comment for the full root-cause writeup (two ordering/lookup bugs
// plus a third, deeper bug where `substitute_type_param`'s eager
// Type::Apply -> Type::Enum/Struct collapse discarded the very
// information the discovery worklist needed).
#[test]
fn recursive_generic_struct_node_direct_struct_literal_produces_correct_output_on_both_backends() {
    let src = write_tmp_vani(
        "recursive-generic-node-direct-structlit",
        r#"
struct Node<T> { value: T, next: Option<Box<Node<T>>> }
fn main() -> i64 {
  let tail: Node<i64> = Node { value: 3, next: Option.None };
  let mid: Node<i64> = Node { value: 2, next: Option.Some(box(tail)) };
  let head: Node<i64> = Node { value: 1, next: Option.Some(box(mid)) };
  print head.value;
  return 0;
}
"#,
    );
    let binary = env!("CARGO_BIN_EXE_intentc");
    for backend_args in [
        vec!["run", src.to_str().unwrap()],
        vec!["run", src.to_str().unwrap(), "--backend=c"],
    ] {
        let output = Command::new(binary)
            .args(&backend_args)
            .output()
            .unwrap_or_else(|e| panic!("intentc {:?} should execute: {e}", backend_args));
        assert!(
            output.status.success(),
            "{:?}: status {:?}, stderr: {}",
            backend_args,
            output.status,
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8_lossy(&output.stdout).replace("\r\n", "\n");
        assert_eq!(stdout, "1\n", "for {:?}; got: {}", backend_args, stdout);
    }
}

// BUG-96 (2026-08-03). Deferred finding from BUG-93: field access
// through a bare `Box<T>` (`boxed.x` where `boxed: Box<Point>`) was
// rejected outright. `Box<T>` (T != dyn Iface) lowers to a bare
// `T*` in both backends, bit-identical to Ref/RefMut, so field
// access now peels it the same way. Exercises both a top-level
// `Box<Point>` binding and a `Box<Point>` struct FIELD (`n.next.x`)
// to cover the lvalue-chaining path too. See the matching
// src/lib.rs test's doc comment for the full root-cause writeup.
#[test]
fn field_access_through_bare_box_produces_correct_output_on_both_backends() {
    let src = write_tmp_vani(
        "box-field-access",
        r#"
struct Point { x: i64, y: i64 }
struct Node { value: Point, next: Box<Point> }
fn main() -> i64 {
  let p: Point = Point { x: 3, y: 4 };
  let boxed: Box<Point> = box(p);
  print boxed.x;
  print boxed.y;
  let n: Node = Node { value: Point { x: 1, y: 2 }, next: box(Point { x: 9, y: 8 }) };
  print n.next.x;
  print n.next.y;
  return 0;
}
"#,
    );
    let binary = env!("CARGO_BIN_EXE_intentc");
    for backend_args in [
        vec!["run", src.to_str().unwrap()],
        vec!["run", src.to_str().unwrap(), "--backend=c"],
    ] {
        let output = Command::new(binary)
            .args(&backend_args)
            .output()
            .unwrap_or_else(|e| panic!("intentc {:?} should execute: {e}", backend_args));
        assert!(
            output.status.success(),
            "{:?}: status {:?}, stderr: {}",
            backend_args,
            output.status,
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8_lossy(&output.stdout).replace("\r\n", "\n");
        assert_eq!(
            stdout, "3\n4\n9\n8\n",
            "for {:?}; got: {}",
            backend_args, stdout
        );
    }
}

// BUG-97 (2026-08-03). Deferred finding from BUG-93/task #39: the
// canonical `Node { next: Option<Box<Node>> }` recursive-struct
// shape (the one the Box<T>/RAII tutorial itself demonstrates)
// leaked on the C backend -- the checker never even emitted a
// scope-exit Drop for a `Node` local at all, let alone one that
// recursed into the Box chain. See the matching src/lib.rs test's
// doc comment for the full root-cause writeup. This test exercises
// a longer (3-node) chain where each node ALSO owns a plain
// `OwnedStr` field alongside the recursive `Box<Self>` edge, to
// cover both the generated deep-drop helper's non-recursive-field
// pass and its worklist-push pass in the same run. Correctness
// (not leak-freedom -- that's separately valgrind-verified, see
// docs/TODO_CURRENT.md) is what this e2e test actually asserts.
#[test]
fn recursive_struct_box_deep_drop_produces_correct_output_on_both_backends() {
    let src = write_tmp_vani(
        "recursive-struct-box-deep-drop",
        r#"
struct Node { value: i64, name: OwnedStr, next: Option<Box<Node>> }
fn main() -> i64 {
  let n0: Node = Node { value: 0, name: "n0" + "", next: Option.None };
  let n1: Node = Node { value: 1, name: "n1" + "", next: Option.Some(box(n0)) };
  let n2: Node = Node { value: 2, name: "n2" + "", next: Option.Some(box(n1)) };
  print n2.value;
  print n2.name;
  return 0;
}
"#,
    );
    let binary = env!("CARGO_BIN_EXE_intentc");
    for backend_args in [
        vec!["run", src.to_str().unwrap()],
        vec!["run", src.to_str().unwrap(), "--backend=c"],
    ] {
        let output = Command::new(binary)
            .args(&backend_args)
            .output()
            .unwrap_or_else(|e| panic!("intentc {:?} should execute: {e}", backend_args));
        assert!(
            output.status.success(),
            "{:?}: status {:?}, stderr: {}",
            backend_args,
            output.status,
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8_lossy(&output.stdout).replace("\r\n", "\n");
        assert_eq!(
            stdout, "2\nn2\n",
            "for {:?}; got: {}",
            backend_args, stdout
        );
    }
}

// BUG-91 (found 2026-08-03, fixed 2026-08-04, task #40). A bare call
// to a generic function returning `Option<T>`, used directly as a
// `match` scrutinee with no intermediate `let` binding, failed with
// "enum 'Option__i64' is not declared" -- the concrete `Option<i64>`
// instantiation was only ever discoverable through fn-generics' own
// type inference at this call site, and by the time that ran, the
// earlier struct/enum decl-monomorphization pass had already
// finished and dropped the generic template it would have needed to
// re-specialize from. See the matching src/lib.rs test's doc comment
// for the full root-cause writeup and fix description.
#[test]
fn bare_generic_call_as_match_scrutinee_produces_correct_output_on_both_backends() {
    let src = write_tmp_vani(
        "bare-generic-call-match-scrutinee",
        r#"
fn foo<T>(a: T) -> Option<T> {
  return Option.Some(a);
}
fn main() -> i64 {
  let r1: i64 = match foo(7) {
    Option.Some(x) then x,
    Option.None then -1,
  };
  print r1;
  return 0;
}
"#,
    );
    let binary = env!("CARGO_BIN_EXE_intentc");
    for backend_args in [
        vec!["run", src.to_str().unwrap()],
        vec!["run", src.to_str().unwrap(), "--backend=c"],
    ] {
        let output = Command::new(binary)
            .args(&backend_args)
            .output()
            .unwrap_or_else(|e| panic!("intentc {:?} should execute: {e}", backend_args));
        assert!(
            output.status.success(),
            "{:?}: status {:?}, stderr: {}",
            backend_args,
            output.status,
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8_lossy(&output.stdout).replace("\r\n", "\n");
        assert_eq!(stdout, "7\n", "for {:?}; got: {}", backend_args, stdout);
    }
}

// BUG-98 (found+fixed 2026-08-04, task #41). A bare enum
// constructor INSIDE a generic function's OWN body failed to
// resolve once 2+ distinct concrete instantiations of the same
// generic enum existed anywhere in the program (here, two
// DIFFERENT generic fns each internally constructing
// `Option.Some(a)`, specialized to `Option__i64` and `Option__bool`
// respectively). See the matching src/lib.rs test's doc comment for
// the full root-cause writeup.
#[test]
fn bare_enum_ctor_inside_generic_fn_body_with_two_instantiations_produces_correct_output_on_both_backends()
{
    let src = write_tmp_vani(
        "bare-enum-ctor-inside-generic-fn-body",
        r#"
fn foo1<T>(a: T) -> Option<T> {
  return Option.Some(a);
}
fn foo2<T>(a: T) -> Option<T> {
  return Option.Some(a);
}
fn main() -> i64 {
  let r1: i64 = match foo1(7) {
    Option.Some(x) then x,
    Option.None then -1,
  };
  let r2: i64 = match foo2(true) {
    Option.Some(x) then 1,
    Option.None then -1,
  };
  print r1;
  print r2;
  return 0;
}
"#,
    );
    let binary = env!("CARGO_BIN_EXE_intentc");
    for backend_args in [
        vec!["run", src.to_str().unwrap()],
        vec!["run", src.to_str().unwrap(), "--backend=c"],
    ] {
        let output = Command::new(binary)
            .args(&backend_args)
            .output()
            .unwrap_or_else(|e| panic!("intentc {:?} should execute: {e}", backend_args));
        assert!(
            output.status.success(),
            "{:?}: status {:?}, stderr: {}",
            backend_args,
            output.status,
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8_lossy(&output.stdout).replace("\r\n", "\n");
        assert_eq!(
            stdout, "7\n1\n",
            "for {:?}; got: {}",
            backend_args, stdout
        );
    }
}

// BUG-87 rows 1-2 (found+fixed, task #42, 2026-08-04). `async fn`
// combined with generics or a built-in generic enum return type was
// broken. Row 1 (a generic async fn called directly inside
// `await(...)`) turned out to already be fixed as a side effect of
// an earlier gap-audit fix; row 2 (an async fn returning
// `Option<i64>`, awaited then matched) needed a real fix -- see the
// matching src/lib.rs tests' doc comment for the full writeup.
#[test]
fn async_fn_generic_call_inside_await_produces_correct_output_on_both_backends() {
    let src = write_tmp_vani(
        "async-fn-generic-call-inside-await",
        r#"
async fn identity<T>(x: T) -> T {
  return x;
}
fn main() -> i64 {
  let r: i64 = await(identity(42));
  print r;
  return 0;
}
"#,
    );
    let binary = env!("CARGO_BIN_EXE_intentc");
    for backend_args in [
        vec!["run", src.to_str().unwrap()],
        vec!["run", src.to_str().unwrap(), "--backend=c"],
    ] {
        let output = Command::new(binary)
            .args(&backend_args)
            .output()
            .unwrap_or_else(|e| panic!("intentc {:?} should execute: {e}", backend_args));
        assert!(
            output.status.success(),
            "{:?}: status {:?}, stderr: {}",
            backend_args,
            output.status,
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8_lossy(&output.stdout).replace("\r\n", "\n");
        assert_eq!(stdout, "42\n", "for {:?}; got: {}", backend_args, stdout);
    }
}

#[test]
fn async_fn_returning_generic_enum_awaited_and_matched_produces_correct_output_on_both_backends()
{
    let src = write_tmp_vani(
        "async-fn-returning-option-awaited-matched",
        r#"
async fn maybe_get(n: i64) -> Option<i64> {
  if n > 0 {
    return Option.Some(n);
  }
  return Option.None;
}
fn main() -> i64 {
  let r1: i64 = match await(maybe_get(5)) {
    Option.Some(x) then x,
    Option.None then -1,
  };
  let r2: i64 = match await(maybe_get(-3)) {
    Option.Some(x) then x,
    Option.None then -1,
  };
  print r1;
  print r2;
  return 0;
}
"#,
    );
    let binary = env!("CARGO_BIN_EXE_intentc");
    for backend_args in [
        vec!["run", src.to_str().unwrap()],
        vec!["run", src.to_str().unwrap(), "--backend=c"],
    ] {
        let output = Command::new(binary)
            .args(&backend_args)
            .output()
            .unwrap_or_else(|e| panic!("intentc {:?} should execute: {e}", backend_args));
        assert!(
            output.status.success(),
            "{:?}: status {:?}, stderr: {}",
            backend_args,
            output.status,
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8_lossy(&output.stdout).replace("\r\n", "\n");
        assert_eq!(
            stdout, "5\n-1\n",
            "for {:?}; got: {}",
            backend_args, stdout
        );
    }
}

// BUG-99 (found+fixed 2026-08-04). `collect_generic_calls_in_stmt`/
// `rewrite_generic_calls_in_stmt` were missing arms for most `Stmt`
// variants beyond `Let`/`Assign`/`Return`/`Print`/`If`/`While` --
// found via a generic call used as an `if let` scrutinee. See the
// matching src/lib.rs test's doc comment for the full writeup.
#[test]
fn generic_call_as_if_let_scrutinee_produces_correct_output_on_both_backends() {
    let src = write_tmp_vani(
        "generic-call-as-if-let-scrutinee",
        r#"
fn foo1<T>(a: T) -> Option<T> {
  return Option.Some(a);
}
fn foo2<T>(a: T) -> Option<T> {
  return Option.Some(a);
}
fn main() -> i64 {
  if let Option.Some(x) = foo1(5) {
    print x;
  }
  if let Option.Some(y) = foo2(true) {
    print 1;
  }
  return 0;
}
"#,
    );
    let binary = env!("CARGO_BIN_EXE_intentc");
    for backend_args in [
        vec!["run", src.to_str().unwrap()],
        vec!["run", src.to_str().unwrap(), "--backend=c"],
    ] {
        let output = Command::new(binary)
            .args(&backend_args)
            .output()
            .unwrap_or_else(|e| panic!("intentc {:?} should execute: {e}", backend_args));
        assert!(
            output.status.success(),
            "{:?}: status {:?}, stderr: {}",
            backend_args,
            output.status,
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8_lossy(&output.stdout).replace("\r\n", "\n");
        assert_eq!(
            stdout, "5\n1\n",
            "for {:?}; got: {}",
            backend_args, stdout
        );
    }
}

// BUG-100 (found+fixed 2026-08-04). Sibling gap to BUG-99:
// `collect_generic_calls_in_expr`/`rewrite_generic_calls_in_expr`
// were missing arms for most `ExprKind` variants -- found via a
// generic call nested inside a `FieldAccess` (`print make(3, 4).a;`
// where `make<T>` returns a generic struct). See the matching
// src/lib.rs test's doc comment for the full writeup.
#[test]
fn generic_call_nested_in_field_access_produces_correct_output_on_both_backends() {
    let src = write_tmp_vani(
        "generic-call-nested-in-field-access",
        r#"
struct Pair<T> { a: T, b: T }
fn make<T>(x: T, y: T) -> Pair<T> {
  return Pair { a: x, b: y };
}
fn main() -> i64 {
  print make(3, 4).a;
  return 0;
}
"#,
    );
    let binary = env!("CARGO_BIN_EXE_intentc");
    for backend_args in [
        vec!["run", src.to_str().unwrap()],
        vec!["run", src.to_str().unwrap(), "--backend=c"],
    ] {
        let output = Command::new(binary)
            .args(&backend_args)
            .output()
            .unwrap_or_else(|e| panic!("intentc {:?} should execute: {e}", backend_args));
        assert!(
            output.status.success(),
            "{:?}: status {:?}, stderr: {}",
            backend_args,
            output.status,
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8_lossy(&output.stdout).replace("\r\n", "\n");
        assert_eq!(stdout, "3\n", "for {:?}; got: {}", backend_args, stdout);
    }
}

// BUG-101 (found+fixed 2026-08-04). `substitute_type_param`'s
// `Type::Apply` collapse mangled a nested generic-enum type argument
// with the wrong naming convention (`type_mangle`, which prefixes
// nominal types with "Enum_"/"Struct_", instead of `type_mangle_for_
// decl`, which the decl-generation worklist actually uses). See the
// matching src/lib.rs test's doc comment for the full writeup.
#[test]
fn nested_generic_option_of_option_produces_correct_output_on_both_backends() {
    let src = write_tmp_vani(
        "nested-generic-option-of-option",
        r#"
fn wrap<T>(x: T) -> Option<Option<T>> {
  return Option.Some(Option.Some(x));
}
fn main() -> i64 {
  let r: i64 = match wrap(9) {
    Option.Some(inner) then match inner {
      Option.Some(v) then v,
      Option.None then -1,
    },
    Option.None then -2,
  };
  print r;
  return 0;
}
"#,
    );
    let binary = env!("CARGO_BIN_EXE_intentc");
    for backend_args in [
        vec!["run", src.to_str().unwrap()],
        vec!["run", src.to_str().unwrap(), "--backend=c"],
    ] {
        let output = Command::new(binary)
            .args(&backend_args)
            .output()
            .unwrap_or_else(|e| panic!("intentc {:?} should execute: {e}", backend_args));
        assert!(
            output.status.success(),
            "{:?}: status {:?}, stderr: {}",
            backend_args,
            output.status,
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8_lossy(&output.stdout).replace("\r\n", "\n");
        assert_eq!(stdout, "9\n", "for {:?}; got: {}", backend_args, stdout);
    }
}

// BUG-102 (found+fixed 2026-08-04). One layer deeper than BUG-101:
// `resolve_bare_enum_ctor_receiver` only ever rewrote the outermost
// bare enum-constructor receiver, never recursing into a payloaded
// variant's own payload argument. Exercised here through `await` too,
// confirming it composes with BUG-87's own fix. See the matching
// src/lib.rs test's doc comment for the full writeup.
#[test]
fn nested_generic_option_two_instantiations_via_async_produces_correct_output_on_both_backends() {
    let src = write_tmp_vani(
        "nested-generic-option-two-instantiations-async",
        r#"
async fn foo1<T>(a: T) -> Option<T> {
  return Option.Some(a);
}
async fn foo2<T>(a: T) -> Option<T> {
  return Option.Some(a);
}
fn main() -> i64 {
  let r1: i64 = match await(foo1(7)) {
    Option.Some(x) then x,
    Option.None then -1,
  };
  let r2: i64 = match await(foo2(true)) {
    Option.Some(x) then 1,
    Option.None then -1,
  };
  print r1;
  print r2;
  return 0;
}
"#,
    );
    let binary = env!("CARGO_BIN_EXE_intentc");
    for backend_args in [
        vec!["run", src.to_str().unwrap()],
        vec!["run", src.to_str().unwrap(), "--backend=c"],
    ] {
        let output = Command::new(binary)
            .args(&backend_args)
            .output()
            .unwrap_or_else(|e| panic!("intentc {:?} should execute: {e}", backend_args));
        assert!(
            output.status.success(),
            "{:?}: status {:?}, stderr: {}",
            backend_args,
            output.status,
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8_lossy(&output.stdout).replace("\r\n", "\n");
        assert_eq!(
            stdout, "7\n1\n",
            "for {:?}; got: {}",
            backend_args, stdout
        );
    }
}

// BUG-103 (found+fixed 2026-08-04, task #44). Three levels of nested
// generic enum ran correctly on LLVM but failed to COMPILE on the C
// backend: the "unified topological emit" loop's deferred-payloaded-
// enum sub-loop checked struct/Vec-bundle dependencies but never
// enum-depends-on-enum dependencies (unlike the sibling struct sub-
// loop, which already did), so the outer enum's typedef could be
// emitted before the enum it depends on. See the matching
// src/lib.rs test's doc comment for the full writeup.
#[test]
fn triple_nested_generic_option_produces_correct_output_on_both_backends() {
    let src = write_tmp_vani(
        "triple-nested-generic-option",
        r#"
fn wrap<T>(x: T) -> Option<Option<Option<T>>> {
  return Option.Some(Option.Some(Option.Some(x)));
}
fn main() -> i64 {
  let r: i64 = match wrap(5) {
    Option.Some(mid) then match mid {
      Option.Some(inner) then match inner {
        Option.Some(v) then v,
        Option.None then -1,
      },
      Option.None then -2,
    },
    Option.None then -3,
  };
  print r;
  return 0;
}
"#,
    );
    let binary = env!("CARGO_BIN_EXE_intentc");
    for backend_args in [
        vec!["run", src.to_str().unwrap()],
        vec!["run", src.to_str().unwrap(), "--backend=c"],
    ] {
        let output = Command::new(binary)
            .args(&backend_args)
            .output()
            .unwrap_or_else(|e| panic!("intentc {:?} should execute: {e}", backend_args));
        assert!(
            output.status.success(),
            "{:?}: status {:?}, stderr: {}",
            backend_args,
            output.status,
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8_lossy(&output.stdout).replace("\r\n", "\n");
        assert_eq!(stdout, "5\n", "for {:?}; got: {}", backend_args, stdout);
    }
}

// BUG-104 (found+fixed 2026-08-04, task #45). `Result<Option<T>,
// i64>` as a generic function's return type -- a 2-argument builtin
// generic enum where only ONE arg depends on T -- never resolved at
// all: `substitute_type_param`'s `Type::Apply` collapse was hard-
// gated on `args.len() == 1`, silently skipping any 2+-arg Apply.
// See the matching src/lib.rs test's doc comment for the full
// writeup.
#[test]
fn generic_fn_returning_two_arg_builtin_enum_produces_correct_output_on_both_backends() {
    let src = write_tmp_vani(
        "generic-fn-returning-two-arg-builtin-enum",
        r#"
fn wrap<T>(x: T, mode: i64) -> Result<Option<T>, i64> {
  if mode == 0 {
    return Result.Ok(Option.Some(x));
  }
  if mode == 1 {
    return Result.Ok(Option.None);
  }
  return Result.Err(77);
}
fn main() -> i64 {
  let r0: i64 = match wrap(6, 0) {
    Result.Ok(opt) then match opt {
      Option.Some(v) then v,
      Option.None then -1,
    },
    Result.Err(e) then e,
  };
  let r1: i64 = match wrap(6, 1) {
    Result.Ok(opt) then match opt {
      Option.Some(v) then v,
      Option.None then -1,
    },
    Result.Err(e) then e,
  };
  let r2: i64 = match wrap(6, 2) {
    Result.Ok(opt) then match opt {
      Option.Some(v) then v,
      Option.None then -1,
    },
    Result.Err(e) then e,
  };
  print r0;
  print r1;
  print r2;
  return 0;
}
"#,
    );
    let binary = env!("CARGO_BIN_EXE_intentc");
    for backend_args in [
        vec!["run", src.to_str().unwrap()],
        vec!["run", src.to_str().unwrap(), "--backend=c"],
    ] {
        let output = Command::new(binary)
            .args(&backend_args)
            .output()
            .unwrap_or_else(|e| panic!("intentc {:?} should execute: {e}", backend_args));
        assert!(
            output.status.success(),
            "{:?}: status {:?}, stderr: {}",
            backend_args,
            output.status,
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8_lossy(&output.stdout).replace("\r\n", "\n");
        assert_eq!(
            stdout, "6\n-1\n77\n",
            "for {:?}; got: {}",
            backend_args, stdout
        );
    }
}

// BUG-105 (found 2026-08-04 by tools/localfuzz, finding
// 20260804-151851-backend-divergence-a0a31dce79): two DIFFERENT
// non-ASCII parameter names (Burmese `က`, `ခ`) collided into the
// same C identifier via a lossy `sanitize_ident`, so `cc` rejected
// the generated C with "redefinition of parameter" while the LLVM
// backend compiled and ran fine. See src/lib.rs's
// `non_ascii_identifier_collision_compiles_to_c_lib` for the
// codegen-level root cause; this checks the full run produces the
// same, correct output on both backends.
#[test]
fn non_ascii_identifier_collision_produces_correct_output_on_both_backends() {
    let src = write_tmp_vani(
        "non-ascii-identifier-collision",
        r#"
fn add(က: i64, ခ: i64) -> i64 {
  return က + ခ;
}
fn main() -> i64 {
  print add(3, 4);
  return 0;
}
"#,
    );
    let binary = env!("CARGO_BIN_EXE_intentc");
    for backend_args in [
        vec!["run", src.to_str().unwrap()],
        vec!["run", src.to_str().unwrap(), "--backend=c"],
    ] {
        let output = Command::new(binary)
            .args(&backend_args)
            .output()
            .unwrap_or_else(|e| panic!("intentc {:?} should execute: {e}", backend_args));
        assert!(
            output.status.success(),
            "{:?}: status {:?}, stderr: {}",
            backend_args,
            output.status,
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8_lossy(&output.stdout).replace("\r\n", "\n");
        assert_eq!(stdout, "7\n", "for {:?}; got: {}", backend_args, stdout);
    }
}

// BUG-106 (found 2026-08-04 by tools/localfuzz plus follow-up
// investigation). A failed `assert` diverged between the C and LLVM
// backends in two independent ways -- see src/lib.rs's
// `failed_assert_exit_code_and_message_match_across_backends_lib`
// for the full root-cause writeup (Part A: message-carrying asserts
// exited 1/abort() on C vs 3/exit() on LLVM; Part B: message-less
// asserts on the SSA fast path were true undefined behavior on the
// LLVM side, reproducing as `lli` JIT crashes). This checks the
// actual process exit code and stderr are now consistent across
// both backends, for both a message-carrying and a message-less
// failing assert.
#[test]
fn failed_assert_exit_code_and_message_match_across_backends() {
    let with_message = write_tmp_vani(
        "failed-assert-with-message",
        r#"
fn main() -> i64 {
  assert 1 == 2, "deliberate failure";
  return 0;
}
"#,
    );
    let bare = write_tmp_vani(
        "failed-assert-bare",
        r#"
fn main() -> i64 {
  let x: i64 = 2;
  assert x == 1;
  return 0;
}
"#,
    );
    let binary = env!("CARGO_BIN_EXE_intentc");
    for (src, expect_message_substr) in [
        (&with_message, Some("deliberate failure")),
        (&bare, None),
    ] {
        let mut c_result = None;
        let mut llvm_result = None;
        for (backend_args, slot) in [
            (vec!["run", src.to_str().unwrap()], &mut llvm_result),
            (
                vec!["run", src.to_str().unwrap(), "--backend=c"],
                &mut c_result,
            ),
        ] {
            let output = Command::new(binary)
                .args(&backend_args)
                .output()
                .unwrap_or_else(|e| panic!("intentc {:?} should execute: {e}", backend_args));
            assert_eq!(
                output.status.code(),
                Some(3),
                "{:?}: a failed assert must exit with code 3 on both backends; \
                 status {:?}, stderr: {}",
                backend_args,
                output.status,
                String::from_utf8_lossy(&output.stderr)
            );
            let stderr = String::from_utf8_lossy(&output.stderr).replace("\r\n", "\n");
            if let Some(substr) = expect_message_substr {
                assert!(
                    stderr.contains(substr),
                    "{:?}: expected stderr to contain {:?}, got: {}",
                    backend_args,
                    substr,
                    stderr
                );
            }
            *slot = Some(stderr);
        }
        assert_eq!(
            c_result, llvm_result,
            "failed-assert stderr must match between C and LLVM backends for {:?}",
            src
        );
    }
}

// Feature-combination gap audit (2026-08-03), category 9 row 3:
// Box<T> through a generic function boundary, both a struct T and a
// scalar T, round-tripped through `identity<T>(b: Box<T>) -> Box<T>`.
#[test]
fn box_through_generic_function_boundary_produces_correct_output_on_both_backends() {
    let src = write_tmp_vani(
        "box_through_generic_fn",
        r#"
struct Point { x: i64, y: i64 }
fn identity<T>(b: Box<T>) -> Box<T> {
  return b;
}
fn main() -> i64 {
  let p: Point = Point { x: 3, y: 4 };
  let boxed: Box<Point> = box(p);
  let round_tripped: Box<Point> = identity(boxed);
  let n: i64 = 42;
  let boxed_n: Box<i64> = box(n);
  let round_tripped_n: Box<i64> = identity(boxed_n);
  print "ok";
  return 0;
}
"#,
    );
    let binary = env!("CARGO_BIN_EXE_intentc");
    for backend_args in [
        vec!["run", src.to_str().unwrap()],
        vec!["run", src.to_str().unwrap(), "--backend=c"],
    ] {
        let output = Command::new(binary)
            .args(&backend_args)
            .output()
            .unwrap_or_else(|e| panic!("intentc {:?} should execute: {e}", backend_args));
        assert!(
            output.status.success(),
            "{:?}: status {:?}, stderr: {}",
            backend_args,
            output.status,
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8_lossy(&output.stdout).replace("\r\n", "\n");
        assert_eq!(stdout, "ok\n", "for {:?}; got: {}", backend_args, stdout);
    }
}

// Feature-combination gap audit (2026-08-03), category 9 row 4:
// `parallel for` over a `Vec<Struct>` with an `OwnedStr` field, each
// iteration writing to its own distinct index via `clone_at` (no
// cross-iteration aliasing). Verified memory-safe separately with
// `valgrind --leak-check=full` on native AOT builds of both
// backends: 0 errors, all heap blocks freed.
#[test]
fn parallel_for_over_vec_struct_with_ownedstr_field_produces_correct_output_on_both_backends() {
    let src = write_tmp_vani(
        "parallel_for_ownedstr_struct",
        r#"
struct Item { id: i64, label: OwnedStr }
fn main() -> i64 {
  let source: Vec<Item> = vec(
    Item { id: 10, label: "a" + "" },
    Item { id: 20, label: "b" + "" },
    Item { id: 30, label: "c" + "" },
  );
  let items: Vec<Item> = vec(
    Item { id: 0, label: "x" + "" },
    Item { id: 0, label: "y" + "" },
    Item { id: 0, label: "z" + "" },
  );
  parallel for i from 0 to 3 {
    items[i] = clone_at(ref source, i);
  }
  let a: Item = clone_at(ref items, 0);
  let b: Item = clone_at(ref items, 1);
  let c: Item = clone_at(ref items, 2);
  print a.id;
  print b.id;
  print c.id;
  return 0;
}
"#,
    );
    let binary = env!("CARGO_BIN_EXE_intentc");
    for backend_args in [
        vec!["run", src.to_str().unwrap()],
        vec!["run", src.to_str().unwrap(), "--backend=c"],
    ] {
        let output = Command::new(binary)
            .args(&backend_args)
            .output()
            .unwrap_or_else(|e| panic!("intentc {:?} should execute: {e}", backend_args));
        assert!(
            output.status.success(),
            "{:?}: status {:?}, stderr: {}",
            backend_args,
            output.status,
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8_lossy(&output.stdout).replace("\r\n", "\n");
        assert_eq!(
            stdout, "10\n20\n30\n",
            "for {:?}; got: {}",
            backend_args, stdout
        );
    }
}

// Feature-combination gap audit (2026-08-03), category 10 row 1: the
// documented "two flat matches" rewrite of a nested `Result<Option
// <i64>, i64>` match produces correct output on both backends.
#[test]
fn nested_result_option_two_flat_matches_produces_correct_output_on_both_backends() {
    let src = write_tmp_vani(
        "nested_result_option_two_flat",
        r#"
fn lookup(x: i64) -> Result<Option<i64>, i64> {
  if x < 0 { return Result.Err(-1); }
  if x == 0 { return Result.Ok(Option.None); }
  return Result.Ok(Option.Some(x * 2));
}
fn classify(x: i64) -> i64 {
  let r: Result<Option<i64>, i64> = lookup(x);
  let inner: Option<i64> = match r {
    Result.Ok(opt) then opt,
    Result.Err(e) then Option.None,
  };
  let is_err: bool = match r {
    Result.Ok(_) then false,
    Result.Err(_) then true,
  };
  if is_err {
    return match r {
      Result.Ok(_) then 0,
      Result.Err(e) then e,
    };
  }
  return match inner {
    Option.Some(v) then v,
    Option.None then 0,
  };
}
fn main() -> i64 {
  print classify(5);
  print classify(0);
  print classify(0 - 1);
  return 0;
}
"#,
    );
    let binary = env!("CARGO_BIN_EXE_intentc");
    for backend_args in [
        vec!["run", src.to_str().unwrap()],
        vec!["run", src.to_str().unwrap(), "--backend=c"],
    ] {
        let output = Command::new(binary)
            .args(&backend_args)
            .output()
            .unwrap_or_else(|e| panic!("intentc {:?} should execute: {e}", backend_args));
        assert!(
            output.status.success(),
            "{:?}: status {:?}, stderr: {}",
            backend_args,
            output.status,
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8_lossy(&output.stdout).replace("\r\n", "\n");
        assert_eq!(
            stdout, "10\n0\n-1\n",
            "for {:?}; got: {}",
            backend_args, stdout
        );
    }
}

// Feature-combination gap audit (2026-08-03), category 10 row 2: a
// guarded slice-pattern arm through a generic Vec<T> element type,
// verified for both an i64 and an f64 instantiation of T.
#[test]
fn guarded_slice_pattern_through_generic_vec_produces_correct_output_on_both_backends() {
    let src = write_tmp_vani(
        "guarded_slice_pattern_generic",
        r#"
fn classify<T>(xs: Vec<T>, n: i64) -> i64 {
  return match xs {
    [a, b] if n > 10 then n,
    _ if n > 5 then 100,
    _ then -1,
  };
}
fn main() -> i64 {
  let ints: Vec<i64> = vec(1, 2);
  print classify(ints, 20);
  let ints2: Vec<i64> = vec(1, 2);
  print classify(ints2, 6);
  let ints3: Vec<i64> = vec(1, 2);
  print classify(ints3, 1);
  let floats: Vec<f64> = vec(1.0, 2.0);
  print classify(floats, 20);
  return 0;
}
"#,
    );
    let binary = env!("CARGO_BIN_EXE_intentc");
    for backend_args in [
        vec!["run", src.to_str().unwrap()],
        vec!["run", src.to_str().unwrap(), "--backend=c"],
    ] {
        let output = Command::new(binary)
            .args(&backend_args)
            .output()
            .unwrap_or_else(|e| panic!("intentc {:?} should execute: {e}", backend_args));
        assert!(
            output.status.success(),
            "{:?}: status {:?}, stderr: {}",
            backend_args,
            output.status,
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8_lossy(&output.stdout).replace("\r\n", "\n");
        assert_eq!(
            stdout, "20\n100\n-1\n20\n",
            "for {:?}; got: {}",
            backend_args, stdout
        );
    }
}

// Feature-combination gap audit (2026-08-03), category 10 row 3: an
// or-pattern-shaped guard referencing the variant's own payload
// binding, on both circle and square arms.
#[test]
fn or_pattern_guard_referencing_variant_binding_produces_correct_output_on_both_backends() {
    let src = write_tmp_vani(
        "or_pattern_guard_variant_binding",
        r#"
enum Shape {
  Circle(i64),
  Square(i64),
}
fn classify(s: Shape) -> i64 {
  return match s {
    Shape.Circle(n) if n == 1 || n == 2 then 100,
    Shape.Circle(n) then n,
    Shape.Square(n) if n == 1 || n == 2 then 200,
    Shape.Square(n) then n * 10,
  };
}
fn main() -> i64 {
  print classify(Shape.Circle(1));
  print classify(Shape.Circle(5));
  print classify(Shape.Square(2));
  print classify(Shape.Square(5));
  return 0;
}
"#,
    );
    let binary = env!("CARGO_BIN_EXE_intentc");
    for backend_args in [
        vec!["run", src.to_str().unwrap()],
        vec!["run", src.to_str().unwrap(), "--backend=c"],
    ] {
        let output = Command::new(binary)
            .args(&backend_args)
            .output()
            .unwrap_or_else(|e| panic!("intentc {:?} should execute: {e}", backend_args));
        assert!(
            output.status.success(),
            "{:?}: status {:?}, stderr: {}",
            backend_args,
            output.status,
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8_lossy(&output.stdout).replace("\r\n", "\n");
        assert_eq!(
            stdout, "100\n5\n200\n50\n",
            "for {:?}; got: {}",
            backend_args, stdout
        );
    }
}

// BUG-94 (2026-08-03), feature-combination gap audit category 11 row
// 1: `HashMap<StructKey, V>` with `self: ref Self` in the `Hash`/`Eq`
// impls -- exactly what the checker's own diagnostic suggests --
// crashed both backends outright. See the matching src/lib.rs test's
// doc comment for the full root-cause writeup (two separate bugs,
// one per backend, same root cause: the HashMap bundle hard-coded a
// by-value calling convention instead of matching the impl's real
// self-parameter convention).
#[test]
fn hashmap_struct_key_hash_eq_self_by_ref_produces_correct_output_on_both_backends() {
    let src = write_tmp_vani(
        "hashmap_struct_key_self_ref",
        r#"
struct Key { a: i64, b: i64 }
interface Hash { fn hash(self: ref Self) -> i64; }
interface Eq { fn eq(self: ref Self, other: ref Self) -> bool; }
implement Hash for Key {
  fn hash(self: ref Key) -> i64 {
    return self.a * 1000003 + self.b;
  }
}
implement Eq for Key {
  fn eq(self: ref Key, other: ref Key) -> bool {
    return self.a == other.a && self.b == other.b;
  }
}
fn main() -> i64 {
  let m: HashMap<Key, i64> = hashmap_new();
  let k1: Key = Key { a: 1, b: 2 };
  let k2: Key = Key { a: 3, b: 4 };
  let k3: Key = Key { a: 5, b: 6 };
  let _ = hashmap_insert(mut ref m, k1, 100);
  let _ = hashmap_insert(mut ref m, k2, 200);
  let _ = hashmap_insert(mut ref m, k3, 300);
  print hashmap_len(ref m);
  let lookup1: Key = Key { a: 1, b: 2 };
  print option_unwrap_or(hashmap_get(ref m, lookup1), -1);
  let lookup2: Key = Key { a: 3, b: 4 };
  print hashmap_contains_key(ref m, lookup2);
  let lookup_missing: Key = Key { a: 9, b: 9 };
  print hashmap_contains_key(ref m, lookup_missing);
  let update_k: Key = Key { a: 1, b: 2 };
  let old: i64 = option_unwrap_or(hashmap_insert(mut ref m, update_k, 999), -1);
  print old;
  let updated_lookup: Key = Key { a: 1, b: 2 };
  print option_unwrap_or(hashmap_get(ref m, updated_lookup), -1);
  let remove_k: Key = Key { a: 5, b: 6 };
  let removed: i64 = option_unwrap_or(hashmap_remove(mut ref m, remove_k), -1);
  print removed;
  print hashmap_len(ref m);
  return 0;
}
"#,
    );
    let binary = env!("CARGO_BIN_EXE_intentc");
    let expected = "3\n100\ntrue\nfalse\n100\n999\n300\n2\n";
    for backend_args in [
        vec!["run", src.to_str().unwrap()],
        vec!["run", src.to_str().unwrap(), "--backend=c"],
    ] {
        let output = Command::new(binary)
            .args(&backend_args)
            .output()
            .unwrap_or_else(|e| panic!("intentc {:?} should execute: {e}", backend_args));
        assert!(
            output.status.success(),
            "{:?}: status {:?}, stderr: {}",
            backend_args,
            output.status,
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8_lossy(&output.stdout).replace("\r\n", "\n");
        assert_eq!(stdout, expected, "for {:?}; got: {}", backend_args, stdout);
    }
}

// Feature-combination gap audit (2026-08-03), category 11 row 3: a
// `dyn Iface` method call held across (and between) TWO `.await`
// points inside an `async fn`. This surfaced a documentation-
// accuracy gap: `docs/missing_features.md` documented this shape as
// unsupported ("dyn-method receivers can't be held across suspend
// points"), but it actually works correctly on both backends --
// verified with two different concrete types behind the same `dyn`
// binding and two separate method calls, one before and one after a
// second await.
#[test]
fn dyn_iface_method_across_multiple_await_points_produces_correct_output_on_both_backends() {
    let src = write_tmp_vani(
        "dyn_across_multi_await",
        r#"
interface Speaker {
  fn speak(self: Self) -> i64;
}
struct Dog { id: i64 }
implement Speaker for Dog {
  fn speak(self: Dog) -> i64 { return self.id; }
}
struct Cat { id: i64 }
implement Speaker for Cat {
  fn speak(self: Cat) -> i64 { return self.id * 10; }
}
async fn delay(x: i64) -> i64 {
  return x;
}
async fn use_dyn_multi_await(use_dog: bool) -> i64 {
  let dog: Dog = Dog { id: 42 };
  let cat: Cat = Cat { id: 7 };
  let d: dyn Speaker = if use_dog { dog as dyn Speaker } else { cat as dyn Speaker };
  let v1: i64 = await(delay(1));
  let mid: i64 = d.speak();
  let v2: i64 = await(delay(2));
  let end: i64 = d.speak();
  return mid + end + v1 + v2;
}
fn main() -> i64 {
  let r1: i64 = await(use_dyn_multi_await(true));
  print r1;
  let r2: i64 = await(use_dyn_multi_await(false));
  print r2;
  return 0;
}
"#,
    );
    let binary = env!("CARGO_BIN_EXE_intentc");
    for backend_args in [
        vec!["run", src.to_str().unwrap()],
        vec!["run", src.to_str().unwrap(), "--backend=c"],
    ] {
        let output = Command::new(binary)
            .args(&backend_args)
            .output()
            .unwrap_or_else(|e| panic!("intentc {:?} should execute: {e}", backend_args));
        assert!(
            output.status.success(),
            "{:?}: status {:?}, stderr: {}",
            backend_args,
            output.status,
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8_lossy(&output.stdout).replace("\r\n", "\n");
        assert_eq!(
            stdout, "87\n143\n",
            "for {:?}; got: {}",
            backend_args, stdout
        );
    }
}

// BUG-107 (2026-08-04): a struct field of type `Vec<Box<dyn Iface>>`
// failed to compile on the C backend -- `cc` rejected the generated
// output with "unknown type name 'intent_dyn_Drawable'" because the
// `intent_vec_box_dyn_Drawable` bundle (referencing that typedef) was
// emitted BEFORE the typedef itself. `compile_to_c`-only tests would
// have caught the string, but this end-to-end run against a real `cc`
// invocation is what the original localfuzz finding actually hit
// (20260803-130927-backend-divergence-dc30074c7a). LLVM was unaffected;
// run on both backends anyway to confirm parity.
#[test]
fn vec_box_dyn_iface_struct_field_example_produces_correct_output_on_both_backends() {
    let binary = env!("CARGO_BIN_EXE_intentc");
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let example = format!(
        "{}/examples/language/english/vec_box_dyn_iface_struct_field.vani",
        manifest_dir
    );
    let expected = "shapes: 2\nids: 2\n";

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
        let stdout = String::from_utf8_lossy(&output.stdout).replace("\r\n", "\n");
        assert_eq!(
            stdout, expected,
            "Vec<Box<dyn Iface>> struct field example produced wrong output for {:?}",
            backend_args
        );
    }
}

// BUG-108 (2026-08-04): the tree-walking LLVM backend's Vec index
// read/write/mut-ref codegen had NO runtime bounds check at all --
// an out-of-range index silently read/wrote arbitrary memory instead
// of trapping. Tree-LLVM is not a rare fallback: ANY program with a
// struct literal or field access (or several dozen builtins,
// including `graph_new`/`graph_astar`/`graph_topo_sort`) always
// routes there (see `expr_ssa_supported` in src/main.rs). Found via a
// localfuzz finding originally mischaracterized as a narrower `mut
// ref Vec<T>` write-back bug
// (20260803-144958-backend-divergence-2125e1a114); bisected to this
// general gap with a minimal repro needing only ANY struct-typed
// local before an out-of-range Vec index. This is a real subprocess
// run (not just a `compile_to_llvm` substring check) because the
// actual bug is a runtime memory-safety violation that only shows up
// when the generated IR is actually executed by `lli`.
#[test]
fn tree_llvm_out_of_range_vec_index_aborts_on_both_backends() {
    use std::fs;
    let binary = env!("CARGO_BIN_EXE_intentc");
    let dir = std::env::temp_dir().join(format!(
        "intentc-bug108-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir_all(&dir).expect("mkdir");
    let src = dir.join("oob.vani");
    fs::write(
        &src,
        r#"
struct Foo { a: i64, b: i64 }
fn main() -> i64 {
  // The struct literal below forces this program off the SSA-LLVM
  // fast path and onto tree-LLVM (struct literals are always
  // rejected by `expr_ssa_supported`), which is exactly the
  // condition BUG-108 needed to reproduce.
  let g: Foo = Foo { a: 1, b: 2 };
  let xs: Vec<i64> = vec();
  print "before oob read";
  let v: i64 = xs[0];
  print "unreachable:", v;
  return 0;
}
"#,
    )
    .expect("write");

    for backend_args in [
        vec!["run", src.to_str().unwrap()],
        vec!["run", src.to_str().unwrap(), "--backend=c"],
    ] {
        let output = Command::new(binary)
            .args(&backend_args)
            .output()
            .unwrap_or_else(|e| panic!("intentc {:?} should execute: {e}", backend_args));
        let stdout = String::from_utf8_lossy(&output.stdout).replace("\r\n", "\n");
        assert!(
            !output.status.success(),
            "an out-of-range Vec index must abort/exit non-zero for {:?}, but it \
             succeeded -- stdout: {}",
            backend_args,
            stdout
        );
        // stdout is fully buffered (not a tty) and never flushed on
        // abort()/SIGABRT, so "before oob read" won't reliably show
        // up even when the abort fires exactly where expected --
        // the decisive check is that "unreachable: <garbage>" (the
        // print AFTER the OOB read) never appears, which is only
        // possible if the read silently "succeeded" with garbage
        // data instead of trapping (BUG-108's exact failure mode
        // pre-fix).
        assert!(
            !stdout.contains("unreachable:"),
            "the OOB read must abort before reaching the next print statement for \
             {:?} -- got: {}",
            backend_args,
            stdout
        );
    }
    let _ = fs::remove_dir_all(&dir);
}

// BUG-109 (2026-08-04): tree-LLVM's `Vec<bool>` LITERAL construction
// (`vec(true, true, …)`) used a byte-addressed, one-bool-per-slot
// buffer layout, incompatible with the packed (64-bools-per-i64-word)
// layout every other `intent_vec_bool` operation (Index read,
// IndexAssign write, push) expects. A `Vec<bool>` literal with 2+
// elements read back garbage for any index whose bit didn't happen to
// land in byte 0. This surfaced as an apparent LLVM task/async
// scheduling "hang" in a localfuzz finding
// (20260803-050543-run-crash-6bd324cd8f) derived near-verbatim from
// this shipped example: `let alive: Vec<bool> = vec(true, true,
// true);` then `if alive[j] { poll pool[j] }` in a round-robin
// scheduler silently read `alive[1]`/`alive[2]` as permanently
// `false` from construction (not from any later corruption or a real
// scheduling bug), so those pool slots were never polled again --
// indistinguishable from an infinite hang without bisecting away the
// task/async machinery entirely, which is what actually found this.
// This is a real subprocess run because the bug only manifests once
// the LLVM IR actually executes under `lli` (a `compile_to_llvm`
// string check wouldn't catch a data-layout mismatch that still
// verifies as valid IR).
#[test]
fn echo_pool_example_produces_correct_output_on_both_backends() {
    let binary = env!("CARGO_BIN_EXE_intentc");
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let example = format!(
        "{}/examples/language/english/echo_pool.vani",
        manifest_dir
    );
    let expected = "server bound (port > 0): true\ntotal bytes received across pool: 9\n";

    for backend_args in [vec!["run", &example], vec!["run", &example, "--backend=c"]] {
        // Wrapped in the real `timeout` command: if this regresses,
        // the test fails (killed, non-zero exit) after 30s instead
        // of hanging the whole suite (and CI) forever, same pattern
        // as the BUG-86 mutex-deadlock regression test above.
        let mut cmd_args = vec!["30", binary];
        cmd_args.extend(backend_args.iter().copied());
        let output = Command::new("timeout")
            .args(&cmd_args)
            .output()
            .unwrap_or_else(|e| panic!("timeout+intentc {:?} should execute: {e}", backend_args));
        assert!(
            output.status.success(),
            "{:?}: status {:?} (124 = timeout/hang -- BUG-109 regression), stderr: {}",
            backend_args,
            output.status,
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8_lossy(&output.stdout).replace("\r\n", "\n");
        assert_eq!(
            stdout, expected,
            "echo_pool example produced wrong output for {:?}",
            backend_args
        );
    }
}

// BUG-110 (2026-08-05): `InstrKind::Binary`'s `checked` flag (mirroring
// the typed-IR field of the same name, set by the checker and refined
// by SMT elision) was silently DROPPED during SSA lowering, so BOTH
// SSA backends (ssa_backend_c.rs, ssa_backend_llvm.rs) emitted fully
// unchecked arithmetic regardless of what the checker determined. This
// is the DEFAULT/preferred backend path for any program without a
// struct literal, field access, or a handful of denylisted builtins
// (`expr_ssa_supported` in main.rs) -- i.e. most ordinary programs --
// so this silently stripped overflow/divide-by-zero/shift-range
// protection from nearly everything, exposed directly to C's
// undefined-behavior-on-signed-overflow optimizations and to raw
// hardware traps.
//
// Found via a localfuzz finding (20260803-033452-run-crash-
// 99db3e1928) originally attributed to "parallel/sort library
// loading" in the handoff doc -- the parallel/sort libs the `lli`
// invocation loads are unconditionally loaded regardless of whether
// the program uses them (a red herring). The actual repro is
// `examples/language/odia/keywords.vani`'s factorial fuzzed from
// `n - 1` to `n - -1` (unbounded/increasing recursion instead of
// bounded/decreasing). Before this fix: `--backend=c` hung forever
// (100% CPU, flat stack -- gcc statically proved the recursive branch
// unreachable-by-UB since it would eventually signed-overflow, and
// replaced it with an infinite `jmp $` spin, confirmed via `-
// Waggressive-loop-optimizations` and objdump disassembly); the
// default LLVM backend crashed via a raw, uncontrolled deep-recursion
// stack exhaustion inside `lli`'s own interpreter. After this fix:
// both backends now genuinely recurse (the overflow check is a real,
// must-respect side effect gcc can no longer prove away) and
// correctly, honestly crash FAST from real stack exhaustion --
// `SIGSEGV` on C (confirmed via direct `cc`-compiled-binary
// inspection), an abort()-driven `lli` crash on LLVM -- instead of
// hanging on one backend and doing an uncontrolled crash on the
// other. This test's contract is deliberately narrow: NOT "produces
// correct output" (the input is a genuinely broken, unbounded-
// recursion program -- there's no correct output), just "terminates
// quickly on both backends" (proving neither hangs).
#[test]
fn odia_factorial_unbounded_recursion_fails_fast_on_both_backends() {
    let binary = env!("CARGO_BIN_EXE_intentc");
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let example = format!(
        "{}/examples/language/odia/keywords.vani",
        manifest_dir
    );
    // Same single-character mutation the localfuzz finding used:
    // `n - 1` (bounded/decreasing) -> `n - -1` (unbounded/increasing).
    let source = std::fs::read_to_string(&example)
        .expect("read examples/language/odia/keywords.vani")
        .replacen("factorial(n - 1)", "factorial(n - -1)", 1);
    let dir = std::env::temp_dir().join(format!(
        "intentc-bug110-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&dir).expect("mkdir");
    let src = dir.join("unbounded_factorial.vani");
    std::fs::write(&src, &source).expect("write");

    for backend_args in [
        vec!["run", src.to_str().unwrap()],
        vec!["run", src.to_str().unwrap(), "--backend=c"],
    ] {
        // `timeout` proves this isn't a hang (BUG-110's pre-fix C
        // symptom): a fast non-zero/killed exit is EXPECTED here (the
        // program is genuinely broken), a `timeout`-triggered kill
        // (status 124) is the regression this test guards against.
        let mut cmd_args = vec!["15", binary];
        cmd_args.extend(backend_args.iter().copied());
        let output = Command::new("timeout")
            .args(&cmd_args)
            .output()
            .unwrap_or_else(|e| panic!("timeout+intentc {:?} should execute: {e}", backend_args));
        assert!(
            !output.status.success(),
            "unbounded recursion must crash/abort, not succeed, for {:?}",
            backend_args
        );
        assert_ne!(
            output.status.code(),
            Some(124),
            "must fail fast, not hang until the 15s timeout kills it (BUG-110 regression) for {:?} -- stdout: {}, stderr: {}",
            backend_args,
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
    let _ = std::fs::remove_dir_all(&dir);
}

// BUG-110 regression guard, correct-usage side: the REAL shipped
// example (unmodified, bounded/decreasing `n - 1` recursion) must
// still produce the right answer on both backends -- the new checked-
// arithmetic guards must not fire as false positives on ordinary,
// non-overflowing recursion/arithmetic.
#[test]
fn odia_keywords_example_produces_correct_output_on_both_backends() {
    let binary = env!("CARGO_BIN_EXE_intentc");
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let example = format!(
        "{}/examples/language/odia/keywords.vani",
        manifest_dir
    );
    // Odia dialect renders integers with Odia-script digits, not
    // ASCII (confirmed by running the example directly; both
    // backends already agreed on this before BUG-110's fix, so it's
    // not itself part of what this test guards).
    let expected = "5! = \u{0B67}\u{0B68}\u{0B66}  6! = \u{0B6D}\u{0B68}\u{0B66}\n";

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
        let stdout = String::from_utf8_lossy(&output.stdout).replace("\r\n", "\n");
        assert_eq!(
            stdout, expected,
            "odia keywords example produced wrong output for {:?}",
            backend_args
        );
    }
}

// BUG-111 (2026-08-05): SSA-LLVM emitted invalid LLVM IR for
// `let x: f64 = <integer literal>;` (and the f32 equivalent) --
// `lli` rejected the generated `.ll` outright at parse time with
// "integer constant must have integer type" before the program
// ever ran. Root cause: the checker desugars the implicit
// int-literal-to-float coercion into a `TypedExprKind::Cast` node
// wrapping the still-integer-typed literal; SSA lowering turns
// that into `InstrKind::Cast { x: Operand::Const(Const::Int(0)),
// to: F64 }`. The SSA-LLVM emitter's `operand_type` helper has no
// `ValueId` to look up for a bare `Operand::Const`, so it returned
// `None`, and the old fallback defaulted the cast's source type to
// its OWN TARGET type -- making a real int-to-float cast look like
// a same-type identity op. That took the "identity" branch in
// `emit_cast`, emitting `fadd double 0.0, 0` (the literal's plain
// integer spelling) instead of a real `sitofp`. This is a real
// subprocess/`lli` run, not a string-content check, because the
// bug is that `lli` refuses to even PARSE the generated IR --
// exactly the class of failure a `compile_to_llvm` string
// assertion wouldn't catch (the tree-LLVM backend, which this
// program's lack of any struct literal keeps it OFF of, doesn't
// hit this path at all -- see `expr_ssa_supported` in
// src/main.rs).
#[test]
fn int_literal_to_float_let_produces_correct_output_on_both_backends() {
    use std::fs;
    let binary = env!("CARGO_BIN_EXE_intentc");
    let dir = std::env::temp_dir().join(format!(
        "intentc-bug111-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir_all(&dir).expect("mkdir");
    let src = dir.join("f64_lit.vani");
    fs::write(
        &src,
        r#"
fn main() -> i64 {
  let n: f64 = 0;
  let total: f64 = 7;
  let half: f32 = 3;
  print n;
  print total;
  print half;
  return 0;
}
"#,
    )
    .expect("write");
    let expected = "0\n7\n3\n";

    for backend_args in [
        vec!["run", src.to_str().unwrap()],
        vec!["run", src.to_str().unwrap(), "--backend=c"],
    ] {
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
        let stdout = String::from_utf8_lossy(&output.stdout).replace("\r\n", "\n");
        assert_eq!(
            stdout, expected,
            "int-literal-to-float let produced wrong output for {:?}",
            backend_args
        );
    }
    let _ = fs::remove_dir_all(&dir);
}

// BUG-113/115 (2026-08-05): a family of runtime traps -- the
// `requires`-clause precondition guard (tree-LLVM), the Vec bounds
// check (both tree-LLVM and SSA-LLVM), and the checked-overflow /
// checked-divisor / checked-shift guards (SSA-LLVM, added by
// BUG-108/110 the same day) -- all used a raw `call void @abort()`.
// Under `lli`'s JIT, a raw `abort()` produces "PLEASE submit a bug
// report to https://github.com/llvm/llvm-project/issues/..." and a
// full internal-crash stack dump instead of a clean process exit --
// misleading for what is actually a well-defined, expected language-
// level trap. This is the exact class BUG-106 already fixed for
// plain `assert` statements; these sites just hadn't been touched
// yet. Fixed by switching every one of them to `exit(3)`, matching
// every other vāṇी runtime trap's exit-code convention. Confirmed via
// a minimal NON-recursive out-of-bounds index (no stack overflow
// involved at all) that this reproduces independent of BUG-108/110's
// own repros, whose writeups had (incorrectly, for this general case)
// called the `lli` crash report "an accepted, not-further-fixable
// characteristic."
#[test]
fn runtime_traps_exit_cleanly_instead_of_crashing_lli_on_llvm_backend() {
    let binary = env!("CARGO_BIN_EXE_intentc");
    let cases: &[(&str, &str)] = &[
        (
            "requires-violated",
            r#"
fn safe_sqrt(x: f64) -> f64 requires sqrt(x) < 1000000.0; {
  return sqrt(x);
}
fn main() -> i64 {
  let y: f64 = safe_sqrt(1.0e30);
  print y;
  return 0;
}
"#,
        ),
        (
            "bounds-check-tree-llvm",
            r#"
struct Foo { a: i64 }
fn main() -> i64 {
  let g: Foo = Foo { a: 1 };
  let v: Vec<i64> = vec(1, 2, 3);
  print v[10];
  return 0;
}
"#,
        ),
        (
            "bounds-check-ssa-llvm",
            r#"
fn main() -> i64 {
  let v: Vec<i64> = vec(1, 2, 3);
  print v[10];
  return 0;
}
"#,
        ),
        (
            "checked-divisor-ssa-llvm",
            r#"
fn id(x: i64) -> i64 { return x; }
fn main() -> i64 {
  return 10 / id(0);
}
"#,
        ),
    ];
    for (name, src) in cases {
        let src_path = write_tmp_vani(&format!("bug115-{name}"), src);
        let output = Command::new(binary)
            .args(["run", src_path.to_str().unwrap()])
            .output()
            .unwrap_or_else(|e| panic!("intentc run {} should execute: {e}", name));
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert_eq!(
            output.status.code(),
            Some(3),
            "{}: expected a clean exit(3), not an lli crash-report signal; \
             status {:?}, stderr: {}",
            name,
            output.status,
            stderr
        );
        assert!(
            !stderr.contains("PLEASE submit a bug report"),
            "{}: lli still produced its misleading internal-crash report:\n{}",
            name,
            stderr
        );
    }
}

// BUG-114 (2026-08-05): `ssa_backend_c.rs`'s `c_const` used Rust's
// Display formatting (`{}`) for `Const::Float`, which omits both the
// decimal point AND any exponent notation for a large whole-number
// f64 (e.g. `1e20` -> the 21-digit string
// "100000000000000000000", no "." and no "e"). C's lexer parses a
// bare digit sequence with neither as an INTEGER constant -- once
// that exceeds `unsigned long long`'s range (~1.8e19), gcc/clang warn
// "integer constant is too large for its type" and silently
// truncate/wrap it before the implicit int->double conversion,
// corrupting the value with no compiler error and no runtime crash
// (confirmed: `1.0e30` printed as `5.07694e+18` instead of `1e+30`).
// Found by hand while root-causing BUG-113 (a `requires`-clause repro
// that happened to assign a large f64 literal). Fixed by switching to
// `{:?}` (Debug), matching `backend_c.rs`'s tree-emitter's
// `emit_float_literal`, which already correctly produces `1e30`-style
// notation -- valid C float syntax (`digit-sequence exponent-part`
// needs no decimal point).
#[test]
fn large_float_literal_produces_correct_output_on_both_backends() {
    let binary = env!("CARGO_BIN_EXE_intentc");
    let src_path = write_tmp_vani(
        "bug114-large-float-literal",
        r#"
fn main() -> i64 {
  let z: f64 = 1.0e30;
  print z;
  return 0;
}
"#,
    );
    for backend_args in [
        vec!["run", src_path.to_str().unwrap()],
        vec!["run", src_path.to_str().unwrap(), "--backend=c"],
    ] {
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
        let stdout = String::from_utf8_lossy(&output.stdout).replace("\r\n", "\n");
        assert_eq!(
            stdout, "1e+30\n",
            "{:?}: large f64 literal produced the wrong value (BUG-114)",
            backend_args
        );
    }
}

// BUG-116 (2026-08-05): `f.requires` was never lowered into SSA at
// all (`ssa.rs` never read the field). The checker uses an
// unprovable-at-call-site `requires` clause as a licensed ASSUMPTION
// to elide runtime `checked` guards on operations inside the function
// body that the precondition makes provably safe -- e.g. `requires b
// > 0` lets `a / b` skip its divide-by-zero guard -- but with no
// SSA-side enforcement of the precondition itself, calling such a
// function with a violating argument hit a completely unguarded
// operation (confirmed via direct IR inspection: a bare `sdiv`, zero
// guard calls anywhere in the module) instead of a controlled trap --
// a raw hardware SIGFPE, not even the (now-fixed) misleading `lli`
// crash report, on a default `vanic run` with no struct literal or
// denylisted builtin anywhere in the program. Fixed by lowering each
// `requires` clause into the same guard shape `TypedStmt::Assert`
// already uses, emitted at function entry before the body; gated
// `ssa_path_supports` (main.rs) to also require every `requires`
// expression itself be SSA-supported, so a clause using an
// SSA-denylisted builtin (e.g. `sqrt`) correctly falls back to the
// tree backend instead of emitting invalid IR.
#[test]
fn requires_clause_violation_traps_and_valid_call_still_works_on_both_backends() {
    let binary = env!("CARGO_BIN_EXE_intentc");
    let src = r#"
fn safe_div(a: i64, b: i64) -> i64
requires b > 0;
{
  return a / b;
}
fn id(x: i64) -> i64 { return x; }
fn main() -> i64 {
  print safe_div(10, id(2));
  print safe_div(10, id(0));
  return 0;
}
"#;
    let src_path = write_tmp_vani("bug116-requires-enforcement", src);
    for backend_args in [
        vec!["run", src_path.to_str().unwrap()],
        vec!["run", src_path.to_str().unwrap(), "--backend=c"],
    ] {
        let output = Command::new(binary)
            .args(&backend_args)
            .output()
            .unwrap_or_else(|e| panic!("intentc {:?} should execute: {e}", backend_args));
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains('5'),
            "{:?}: the VALID call (safe_div(10, 2) == 5) must still succeed; \
             stdout: {}, stderr: {}",
            backend_args,
            stdout,
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(
            output.status.code(),
            Some(3),
            "{:?}: the VIOLATING call (safe_div(10, 0)) must trap with exit(3), \
             not run to completion or crash uncontrolled; status {:?}, stderr: {}",
            backend_args,
            output.status,
            String::from_utf8_lossy(&output.stderr)
        );
    }
}

// BUG-117 (2026-08-05): the `#[bounded(N)]` recursion-depth guard
// (both tree-LLVM and SSA-LLVM) used the same raw `call void
// @abort()` as BUG-113/115/116's other guards, found via a broader
// grep after that pass landed. Same fix: `exit(3)`.
#[test]
fn bounded_attribute_violation_exits_cleanly_instead_of_crashing_lli() {
    let binary = env!("CARGO_BIN_EXE_intentc");
    let src_path = write_tmp_vani(
        "bug117-bounded-abort",
        r#"
#[bounded(3)]
fn deep(n: i64) -> i64 {
  if n <= 0 { return 0; }
  return deep(n - 1) + 1;
}
fn main() -> i64 { return deep(10); }
"#,
    );
    let output = Command::new(binary)
        .args(["run", src_path.to_str().unwrap()])
        .output()
        .unwrap_or_else(|e| panic!("intentc run should execute: {e}"));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(
        output.status.code(),
        Some(3),
        "expected a clean exit(3) for a #[bounded(N)] violation, not an lli \
         crash-report signal; status {:?}, stderr: {}",
        output.status,
        stderr
    );
    assert!(
        !stderr.contains("PLEASE submit a bug report"),
        "lli still produced its misleading internal-crash report:\n{}",
        stderr
    );
}

// BUG-126 (2026-08-07): reassigning an Array-typed binding (via
// `x = [...]` or same-scope `let`-shadowing, which the checker
// desugars into the same Reassign node) produced invalid C --
// `v_xs = ((int64_t[5]){...});`, which `cc` rejects since C arrays
// aren't assignable via `=`. Separately, reassigning a `ref
// T`-typed binding corrupted memory on LLVM: `ctx.locals[name]`
// for a ref holds the raw pointer VALUE (not an alloca address --
// see the L4(B) Let-path comment in backend_llvm.rs), but Reassign
// unconditionally `store`d into it as though it WERE an address,
// silently overwriting the first field of whatever the ref pointed
// at. Found via localfuzz backend-divergence findings (tracked
// pre-fix in docs/UNRESOLVED_GAPS_TODO.md item A1). Fixed by
// special-casing `Type::Array` in backend_c.rs's Reassign arm
// (memcpy/per-element store into the existing storage instead of
// `=`) and `ty.is_any_ref()` in backend_llvm.rs's Reassign arm
// (rebind `ctx.locals` to the new pointer value instead of
// `store`ing through the old one, mirroring the Let path).
#[test]
fn reassign_array_and_ref_example_produces_correct_output_on_both_backends() {
    let binary = env!("CARGO_BIN_EXE_intentc");
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let example = format!(
        "{}/examples/language/english/reassign_array_and_ref.vani",
        manifest_dir
    );
    let expected = "10\n63\n63\n";

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
            "reassigning an Array binding (C backend) or a ref binding \
             (LLVM backend) produced the wrong result for {:?} -- see \
             BUG-126",
            backend_args
        );
    }
}

// BUG-127 (2026-08-07): the checker's overflow-elision pass
// deliberately keeps SMT facts about a reassigned variable alive
// across a loop body (for a separate loop-invariant-preservation
// check), which made a fact true only on the FIRST time control
// reaches a statement (e.g. `n == 0`, from before the loop) look
// like it still held on every later iteration too. The elision
// pass "proved" `n + i64::MIN` never overflows using that stale
// fact and elided the runtime guard; the LLVM backend then wrapped
// silently on the second iteration and looped forever instead of
// trapping -- turning an intended 5-iteration loop into a real
// infinite one. Wrapped in the real `timeout` command: if this
// regresses, the test fails (killed, exit 124) after 10s instead of
// hanging the whole suite forever, same pattern as the BUG-109 /
// echo_pool regression tests above. This is a real subprocess run
// because the bug is a runtime hang, not a compile-time/string-level
// difference a `compile_to_llvm` check could catch on its own.
#[test]
fn loop_carried_overflow_not_elided_example_traps_instead_of_hanging_on_both_backends() {
    let binary = env!("CARGO_BIN_EXE_intentc");
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let example = format!(
        "{}/examples/language/english/loop_carried_overflow_not_elided.vani",
        manifest_dir
    );

    for (backend_args, expect_code) in [
        (vec!["run", &example], 3),
        // 134 = 128 + SIGABRT (6): the C-backend overflow trap still
        // raises a raw `abort()`, and BUG-130 (2026-08-07) fixed
        // `vanic run` to report the shell convention for a signal-
        // killed child instead of masking it as a bare `1`.
        (vec!["run", &example, "--backend=c"], 134),
    ] {
        let mut cmd_args = vec!["10", binary];
        cmd_args.extend(backend_args.iter().copied());
        let output = Command::new("timeout")
            .args(&cmd_args)
            .output()
            .unwrap_or_else(|e| panic!("timeout+intentc {:?} should execute: {e}", backend_args));
        assert_eq!(
            output.status.code(),
            Some(expect_code),
            "{:?}: status {:?} (124 = timeout/hang -- BUG-127 regression: the \
             loop-carried overflow check was wrongly elided again), stderr: {}",
            backend_args,
            output.status,
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(
            !String::from_utf8_lossy(&output.stdout)
                .contains("unreachable: overflow should have trapped"),
            "{:?}: the overflow trap did not fire before the print -- BUG-127 \
             regression",
            backend_args
        );
    }
}

// BUG-129 (2026-08-07): tree-C's `requires`-clause runtime guard
// still used the raw libc `assert()` macro (SIGABRT on failure) long
// after BUG-116 gave the SSA-C path's own `requires` lowering a
// clean `fprintf(stderr, "assertion failed: %s\n", msg); exit(3);`
// shape. `vanic build` falls back to tree-C for the WHOLE module
// whenever ANY function uses an SSA-unsupported feature (here,
// `match` in `pick`), so this affected `f`'s `requires` clause too
// even though `f` itself has nothing SSA-unsupported about it. Real
// subprocess run (not a `compile_to_c` string check) because the
// actual bug is in the SIGNAL behavior at runtime -- a string check
// can't distinguish `exit(3)` from a `SIGABRT`-raising `assert()`
// that happens to print similar-looking text.
#[test]
fn requires_guard_survives_tree_c_fallback_example_traps_cleanly_with_exit3() {
    let binary = env!("CARGO_BIN_EXE_intentc");
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let example = format!(
        "{}/examples/language/english/requires_guard_survives_tree_c_fallback.vani",
        manifest_dir
    );

    let output = Command::new(binary)
        .args(["run", &example, "--backend=c"])
        .output()
        .unwrap_or_else(|e| panic!("intentc run --backend=c should execute: {e}"));
    assert_eq!(
        output.status.code(),
        Some(3),
        "BUG-129 regression: a requires-clause violation on tree-C fell back to \
         raw assert()/SIGABRT instead of a clean exit(3); status {:?}, stderr: {}",
        output.status,
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "5\n",
        "the first (satisfied) requires call should still print normally before \
         the second (violating) call traps"
    );
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("precondition violated in 'f'"),
        "expected a clean precondition-violated message on stderr, got: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

// BUG-130 (2026-08-07): `vanic run`'s own process wrapper used
// `status.code().unwrap_or(1)` at every call site, which silently
// converts a signal-killed child into a generic exit code `1` --
// indistinguishable from a program that legitimately called
// `exit(1)`, and losing which signal it actually was. Integer
// overflow on the C backend still raises a real `SIGABRT`
// (deliberately out of scope for the BUG-106-class `exit(3)`
// conversions, which were scoped to the LLVM backend's misleading-
// `lli`-crash-report problem -- see the "What actually happens"
// section of `tutorials/src/intermediate/10b_runtime_errors_primer.md`),
// making it a reliable way to produce a signal-killed child for this
// test. (The `#[bounded(N)]` guard used to serve this same role, but
// itself moved to a clean `exit(3)` on both C codegen paths as a
// follow-up cleanup after BUG-130 -- see the "Aside (not fixed, out
// of scope)" note this test's own history carries in
// `docs/TODO_CURRENT.md`'s BUG-130 entry.) Expects the shell
// convention `128 + signal` (134 = 128 + SIGABRT's 6), matching what
// a directly-executed binary's own shell would show.
#[test]
fn signal_killed_child_reports_128_plus_signal_not_a_bare_1() {
    let binary = env!("CARGO_BIN_EXE_intentc");
    let src = write_tmp_vani(
        "bug130-signal-exit-code",
        r#"
fn add_it(a: i64, b: i64) -> i64 { return a + b; }
fn main() -> i64 { return add_it(9223372036854775807, 1); }
"#,
    );
    let output = Command::new(binary)
        .args(["run", src.to_str().unwrap(), "--backend=c"])
        .output()
        .unwrap_or_else(|e| panic!("intentc run --backend=c should execute: {e}"));
    assert_eq!(
        output.status.code(),
        Some(134),
        "BUG-130 regression: a SIGABRT-killed child was reported as a bare exit \
         code instead of 128+signal (134 = 128 + SIGABRT); status {:?}, stderr: {}",
        output.status,
        String::from_utf8_lossy(&output.stderr)
    );
}

// BUG-131 (2026-08-07): `sort`/`sort_by`'s block-partition scan (only
// entered for >= 128 elements) used AVX-512 intrinsics unconditionally
// on any x86_64 host with no runtime CPU-capability check -- SIGILL on
// any x86_64 CPU predating AVX-512 (confirmed on this dev machine's
// own Haswell CPU). Fixing that crash surfaced a second, pre-existing
// bug: `double`'s mask compare reused `int64_t`'s raw-bit-pattern
// comparison, which does NOT preserve IEEE-754 ordering for negative
// doubles -- invisible before this fix since the crash always fired
// first on a non-AVX-512 host. Both fixed in `sort_runtime.c`: real
// runtime CPUID dispatch via `__builtin_cpu_supports("avx512f")`, and
// a genuinely-floating-point AVX-512/scalar compare for `double`
// instead of reusing `int64_t`'s bit-pattern path. Real subprocess run
// (not a string check) because `sort_runtime.c` is a separate C file
// compiled by a real `cc` invocation at `vanic run` time, entirely
// outside anything `compile_to_c`/`compile_to_llvm` touch.
#[test]
fn sort_large_block_partition_example_produces_correct_output_on_both_backends() {
    let binary = env!("CARGO_BIN_EXE_intentc");
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let example = format!(
        "{}/examples/language/english/sort_large_block_partition.vani",
        manifest_dir
    );
    let expected = "i64 sorted ok, min: -997663\n\
                     i64 sorted ok, max: 998334\n\
                     f64 sorted ok, min: -997.663\n\
                     f64 sorted ok, max: 998.334\n";

    for backend_args in [vec!["run", &example], vec!["run", &example, "--backend=c"]] {
        let output = Command::new(binary)
            .args(&backend_args)
            .output()
            .unwrap_or_else(|e| panic!("intentc {:?} should execute: {e}", backend_args));
        assert!(
            output.status.success(),
            "BUG-131 regression: {:?} failed with status {:?} (132/133/etc = still \
             SIGILL-crashing on AVX-512), stderr: {}",
            backend_args,
            output.status,
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert_eq!(
            stdout.replace("\r\n", "\n"),
            expected,
            "BUG-131 regression: sort produced wrong output for {:?} -- likely the \
             double raw-bit-pattern comparison bug again",
            backend_args
        );
    }
}

// BUG-133 (2026-08-07): `ensures` now mirrors `requires`'s existing
// model -- an SMT-undecidable postcondition (here, `opaque` has no
// `ensures` of its own, so SMT has nothing to reason `wrapper`'s
// postcondition from) no longer hard-fails the build; it compiles and
// gets a real runtime guard at the return site instead, using the
// existing `intent_assert_fail`/`exit(3)` mechanism. Real subprocess
// runs (not compile_to_c/compile_to_llvm string checks) because the
// whole point is verifying actual RUNTIME behavior: the guard must
// stay silent when satisfied and trap with the right exit code and
// message when actually violated.
#[test]
fn undecidable_ensures_runtime_guard_traps_only_when_actually_violated() {
    let binary = env!("CARGO_BIN_EXE_intentc");
    let satisfied = r#"
fn opaque(mode: i64) -> i64 {
  return match mode {
    0 then 5,
    _ then 0 - 5
  };
}
fn wrapper(mode: i64) -> i64
  ensures _return >= 0;
{
  return opaque(mode);
}
fn main() -> i64 {
  print wrapper(0);
  return 0;
}
"#;
    let violated = satisfied.replace("wrapper(0)", "wrapper(1)");

    for (source, expect_status, expect_stdout_contains) in [
        (satisfied, None, Some("5")),
        (violated.as_str(), Some(3), None),
    ] {
        let src = write_tmp_vani("bug133-ensures-runtime", source);
        for backend_args in [
            vec!["run", src.to_str().unwrap()],
            vec!["run", src.to_str().unwrap(), "--backend=c"],
        ] {
            let output = Command::new(binary)
                .args(&backend_args)
                .output()
                .unwrap_or_else(|e| panic!("intentc {:?} should execute: {e}", backend_args));
            if let Some(code) = expect_status {
                assert_eq!(
                    output.status.code(),
                    Some(code),
                    "BUG-133 regression: {:?} expected exit {code} (postcondition \
                     violated), got status {:?}, stderr: {}",
                    backend_args,
                    output.status,
                    String::from_utf8_lossy(&output.stderr)
                );
                assert!(
                    String::from_utf8_lossy(&output.stderr)
                        .contains("postcondition violated in 'wrapper'"),
                    "expected the postcondition-violated message on stderr, got: {}",
                    String::from_utf8_lossy(&output.stderr)
                );
            } else {
                assert!(
                    output.status.success(),
                    "BUG-133 regression: {:?} should run cleanly when the ensures \
                     clause is actually satisfied, got status {:?}, stderr: {}",
                    backend_args,
                    output.status,
                    String::from_utf8_lossy(&output.stderr)
                );
            }
            if let Some(expected) = expect_stdout_contains {
                assert!(
                    String::from_utf8_lossy(&output.stdout).contains(expected),
                    "expected stdout to contain {:?} for {:?}, got: {}",
                    expected,
                    backend_args,
                    String::from_utf8_lossy(&output.stdout)
                );
            }
        }
    }
}

// BUG-133, continued: the runtime guard reads back the already-
// materialized return temp (`__intent_ret_<span>`), not the raw
// return expression a second time -- a side-effecting return (here,
// a `push` the ensures-guarded function calls indirectly through
// `side_effecting`) must fire exactly once, not twice.
#[test]
fn undecidable_ensures_runtime_guard_does_not_double_evaluate_return_expr() {
    let binary = env!("CARGO_BIN_EXE_intentc");
    let source = r#"
fn opaque(mode: i64) -> i64 {
  return match mode {
    0 then 5,
    _ then 0 - 5
  };
}
fn side_effecting(counter: mut ref Vec<i64>, mode: i64) -> i64 {
  let n: i64 = push(counter, 1) as i64;
  let _ = n;
  return opaque(mode);
}
fn wrapper(counter: mut ref Vec<i64>, mode: i64) -> i64
  ensures _return >= 0;
{
  return side_effecting(counter, mode);
}
fn main() -> i64 {
  let calls: Vec<i64> = vec();
  let _ = wrapper(mut ref calls, 0);
  print len(ref calls) as i64;
  return 0;
}
"#;
    let src = write_tmp_vani("bug133-ensures-no-double-eval", source);
    for backend_args in [
        vec!["run", src.to_str().unwrap()],
        vec!["run", src.to_str().unwrap(), "--backend=c"],
    ] {
        let output = Command::new(binary)
            .args(&backend_args)
            .output()
            .unwrap_or_else(|e| panic!("intentc {:?} should execute: {e}", backend_args));
        assert!(
            output.status.success(),
            "{:?} failed: {}",
            backend_args,
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(
            String::from_utf8_lossy(&output.stdout).trim(),
            "1",
            "BUG-133 regression: {:?} -- the ensures runtime guard double-evaluated \
             the return expression (push ran more than once), got: {}",
            backend_args,
            String::from_utf8_lossy(&output.stdout)
        );
    }
}

// BUG-134 (2026-08-07): `invariant`'s runtime-guard follow-up to
// BUG-133. Verifies the entry-check and preservation-check guards
// actually fire (exit 3 + message) only when genuinely violated, and
// stay silent when satisfied, on both backends -- real subprocess
// runs since the point is observing actual runtime behavior.
#[test]
fn undecidable_invariant_runtime_guards_trap_only_when_actually_violated() {
    let binary = env!("CARGO_BIN_EXE_intentc");
    let opaque_fn = r#"
fn opaque(mode: i64) -> i64 {
  return match mode { 0 then 5, _ then 0 - 5 };
}
"#;
    // Entry check satisfied throughout -> runs to completion.
    let entry_ok = format!(
        "{opaque_fn}fn main() -> i64 {{\n  let n: i64 = 0;\n  while n < 3\n  \
         invariant opaque(n) >= 0 - 100;\n  {{ n = n + 1; }}\n  print n;\n  return 0;\n}}\n"
    );
    // Entry check violated on the very first iteration.
    let entry_violated = format!(
        "{opaque_fn}fn main() -> i64 {{\n  let n: i64 = 0;\n  while n < 3\n  \
         invariant opaque(n) >= 1000;\n  {{ n = n + 1; }}\n  print n;\n  return 0;\n}}\n"
    );
    // Preservation check violated on the first iteration's post-body state
    // (opaque(1) + 1 = -4, fails `!= -4`) but entry (n=0: 5+0=5) is fine.
    let preservation_violated = format!(
        "{opaque_fn}fn main() -> i64 {{\n  let n: i64 = 0;\n  while n < 3\n  \
         invariant opaque(n) + n != 0 - 4;\n  {{ n = n + 1; }}\n  print n;\n  return 0;\n}}\n"
    );

    for (source, expect_status, expect_msg) in [
        (entry_ok, None, None),
        (entry_violated, Some(3), Some("does not hold at loop entry")),
        (preservation_violated, Some(3), Some("is not preserved by the loop body")),
    ] {
        let src = write_tmp_vani("bug134-invariant-runtime", &source);
        for backend_args in [
            vec!["run", src.to_str().unwrap()],
            vec!["run", src.to_str().unwrap(), "--backend=c"],
        ] {
            let output = Command::new(binary)
                .args(&backend_args)
                .output()
                .unwrap_or_else(|e| panic!("intentc {:?} should execute: {e}", backend_args));
            match (expect_status, expect_msg) {
                (Some(code), Some(msg)) => {
                    assert_eq!(
                        output.status.code(),
                        Some(code),
                        "BUG-134 regression: {:?} expected exit {code}, got status {:?}, stderr: {}",
                        backend_args,
                        output.status,
                        String::from_utf8_lossy(&output.stderr)
                    );
                    assert!(
                        String::from_utf8_lossy(&output.stderr).contains(msg),
                        "expected {:?} on stderr, got: {}",
                        msg,
                        String::from_utf8_lossy(&output.stderr)
                    );
                }
                _ => {
                    assert!(
                        output.status.success(),
                        "BUG-134 regression: {:?} should run cleanly when the invariant \
                         is actually satisfied throughout, got status {:?}, stderr: {}",
                        backend_args,
                        output.status,
                        String::from_utf8_lossy(&output.stderr)
                    );
                }
            }
        }
    }
}

// BUG-134, continued: `continue` must NOT bypass the preservation
// check -- confirmed as a REAL bug while building this (a bare
// end-of-body append is skipped by `continue`, which jumps straight
// to the loop's condition re-check). Also confirms `break` correctly
// does NOT require the invariant to hold (breaking exits the loop,
// there's no "next iteration" to preserve it for), an unlabeled
// `continue` inside a NESTED loop does not wrongly trigger the OUTER
// loop's check, and a labeled `continue 'outer` from within a nested
// loop DOES trigger the outer loop's check.
#[test]
fn undecidable_invariant_preservation_check_is_not_bypassed_by_continue() {
    let binary = env!("CARGO_BIN_EXE_intentc");
    let opaque_fn = r#"
fn opaque(mode: i64) -> i64 {
  return match mode { 0 then 5, _ then 0 - 5 };
}
"#;
    // Violated exactly at n=1 (opaque(1)+1 = -4, fails != -4), which
    // the loop `continue`s past -- without the fix, this ran to
    // completion silently instead of trapping.
    let continue_bypass = format!(
        "{opaque_fn}fn main() -> i64 {{\n  let n: i64 = 0;\n  while n < 3\n  \
         invariant opaque(n) + n != 0 - 4;\n  {{\n    n = n + 1;\n    if n == 1 {{ continue; }}\n  \
         }}\n  print \"reached end\", n;\n  return 0;\n}}\n"
    );
    // Same violating shape, but a `break` instead of `continue` at
    // the violating iteration -- must NOT trap (no next iteration to
    // preserve the invariant for).
    let break_not_checked = format!(
        "{opaque_fn}fn main() -> i64 {{\n  let n: i64 = 0;\n  while n < 100\n  \
         invariant opaque(n) + n != 0 - 4;\n  {{\n    n = n + 1;\n    if n == 1 {{ break; }}\n  \
         }}\n  print \"reached end\", n;\n  return 0;\n}}\n"
    );
    // Unlabeled continue inside a NESTED loop must not trigger the
    // OUTER loop's (undecidable) invariant check -- the outer
    // invariant here always holds, so this must run to completion.
    let nested_unlabeled_continue = format!(
        "{opaque_fn}fn main() -> i64 {{\n  let n: i64 = 0;\n  let total: i64 = 0;\n  while n < 3\n  \
         invariant opaque(n) >= 0 - 100;\n  {{\n    n = n + 1;\n    let m: i64 = 0;\n    \
         while m < 3 {{\n      m = m + 1;\n      if m == 2 {{ continue; }}\n      total = total + 1;\n    \
         }}\n  }}\n  print total;\n  return 0;\n}}\n"
    );
    // Labeled `continue 'outer` from within a nested loop DOES target
    // the outer loop and must trip its (violated) preservation check.
    let labeled_continue = format!(
        "{opaque_fn}fn main() -> i64 {{\n  let n: i64 = 0;\n  'outer: while n < 3\n  \
         invariant opaque(n) + n != 0 - 4;\n  {{\n    n = n + 1;\n    let m: i64 = 0;\n    \
         while m < 3 {{\n      m = m + 1;\n      if n == 1 {{ continue 'outer; }}\n    \
         }}\n  }}\n  print \"reached end\", n;\n  return 0;\n}}\n"
    );

    for (source, expect_status, expect_msg) in [
        (continue_bypass, Some(3), Some("is not preserved by the loop body")),
        (break_not_checked, None, None),
        (nested_unlabeled_continue, None, None),
        (labeled_continue, Some(3), Some("is not preserved by the loop body")),
    ] {
        let src = write_tmp_vani("bug134-invariant-continue", &source);
        for backend_args in [
            vec!["run", src.to_str().unwrap()],
            vec!["run", src.to_str().unwrap(), "--backend=c"],
        ] {
            let output = Command::new(binary)
                .args(&backend_args)
                .output()
                .unwrap_or_else(|e| panic!("intentc {:?} should execute: {e}", backend_args));
            match (expect_status, expect_msg) {
                (Some(code), Some(msg)) => {
                    assert_eq!(
                        output.status.code(),
                        Some(code),
                        "BUG-134 regression: {:?} expected exit {code}, got status {:?}, stderr: {}",
                        backend_args,
                        output.status,
                        String::from_utf8_lossy(&output.stderr)
                    );
                    assert!(
                        String::from_utf8_lossy(&output.stderr).contains(msg),
                        "expected {:?} on stderr, got: {}",
                        msg,
                        String::from_utf8_lossy(&output.stderr)
                    );
                }
                _ => {
                    assert!(
                        output.status.success(),
                        "BUG-134 regression: {:?} should run cleanly, got status {:?}, stderr: {}",
                        backend_args,
                        output.status,
                        String::from_utf8_lossy(&output.stderr)
                    );
                }
            }
        }
    }
}

// Follow-up cleanup after BUG-130 (2026-08-07): BUG-117 already fixed the
// `#[bounded(N)]` recursion-depth guard's raw `abort()` on both LLVM codegen
// paths, but its C-backend counterpart (tree-C and SSA-C, each with its own
// separate copy of the guard) was explicitly flagged as "not fixed, out of
// scope" in BUG-130's own writeup. Both C codegen paths now use the same
// `exit(3)` + message shape as every other C-backend runtime guard
// (`assert`/`requires`/`ensures`/`invariant`). Verified on both codegen
// paths: `#[no_mangle]` anywhere in the module forces the whole module onto
// tree-C (see `ssa_path_supports` in `src/main.rs`); without it, this
// program is small enough to take the default SSA-C path.
#[test]
fn bounded_attribute_violation_exits_cleanly_on_c_backend_ssa_and_tree_paths() {
    let binary = env!("CARGO_BIN_EXE_intentc");
    for (stem, source) in [
        (
            "bug130-followup-bounded-c-ssa",
            r#"
#[bounded(3)]
fn deep(n: i64) -> i64 {
  if n <= 0 { return 0; }
  return deep(n - 1) + 1;
}
fn main() -> i64 { return deep(10); }
"#
            .to_string(),
        ),
        (
            "bug130-followup-bounded-c-tree",
            r#"
#[no_mangle]
fn keep_alive() -> i64 { return 0; }
#[bounded(3)]
fn deep(n: i64) -> i64 {
  if n <= 0 { return 0; }
  return deep(n - 1) + 1;
}
fn main() -> i64 { return deep(10); }
"#
            .to_string(),
        ),
    ] {
        let src = write_tmp_vani(stem, &source);
        let output = Command::new(binary)
            .args(["run", src.to_str().unwrap(), "--backend=c"])
            .output()
            .unwrap_or_else(|e| panic!("intentc run --backend=c should execute: {e}"));
        assert_eq!(
            output.status.code(),
            Some(3),
            "expected a clean exit(3) for a #[bounded(N)] violation on the C \
             backend ({stem}), not a raw SIGABRT; status {:?}, stderr: {}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(
            String::from_utf8_lossy(&output.stderr).contains("recursion bound exceeded"),
            "expected the recursion-bound message on stderr ({stem}), got: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
}

// BUG-136 (2026-08-07): found via localfuzz's `graph_algo2.vani`
// backend-divergence finding -- reported as a divergence because the C
// backend's raw `abort()` (deliberately kept for bounds/overflow/div-by-
// zero/shift, per the same BUG-106-class scoping decision documented in
// `tutorials/src/intermediate/10b_runtime_errors_primer.md`'s Row 2) does
// not flush stdio, so every `print` statement buffered before the trap
// was silently lost -- while the LLVM backend's `exit(3)` (BUG-120)
// preserves it. Same underlying trap, same computed values, but the C
// backend's stdout looked empty next to LLVM's, reading like the two
// backends disagreed when they didn't. Fixed by adding `fflush(stdout);`
// immediately before each of these `abort()` calls.
#[test]
fn c_backend_preserves_buffered_stdout_before_a_raw_abort_trap() {
    let binary = env!("CARGO_BIN_EXE_intentc");
    let src = write_tmp_vani(
        "bug136-stdout-lost-before-abort",
        r#"
fn main() -> i64 {
  print "line one";
  print "line two";
  let xs: Vec<i64> = vec(1, 2, 3);
  let i: i64 = -1;
  print xs[i];
  return 0;
}
"#,
    );
    let output = Command::new(binary)
        .args(["run", src.to_str().unwrap(), "--backend=c"])
        .output()
        .unwrap_or_else(|e| panic!("intentc run --backend=c should execute: {e}"));
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stdout.contains("line one") && stdout.contains("line two"),
        "BUG-136 regression: buffered stdout before an abort()-raising trap was \
         lost on the C backend; stdout: {stdout:?}, stderr: {stderr:?}, status: {:?}",
        output.status
    );
    assert!(
        stderr.contains("index out of bounds"),
        "expected the bounds-check message on stderr, got: {stderr:?}"
    );
}

// BUG-137 (2026-08-07): found via localfuzz triaging a batch of previously-
// unreviewed candidates. `let (q, r) = f(...);` redeclared (shadowed) in the
// same scope compiled fine on LLVM/SSA-C but failed to even build on tree-C
// (duplicate `int64_t v_q` declaration -- tuple-destructure codegen never
// got the same shadow-handling regular `let` has). Verified on BOTH C
// codegen paths: the default (SSA-C, no `#[no_mangle]`) and tree-C (forced
// via an unrelated `#[no_mangle] fn`).
#[test]
fn tuple_destructure_shadow_runs_correctly_on_both_c_codegen_paths() {
    let binary = env!("CARGO_BIN_EXE_intentc");
    for (stem, source) in [
        (
            "bug137-tuple-shadow-ssa-c",
            r#"
fn divmod(a: i64, b: i64) -> (i64, i64) {
  return (a / b, a % b);
}
fn main() -> i64 {
  let (q, r) = divmod(17, 5);
  let (q, r) = divmod(20, 3);
  return q + r;
}
"#
            .to_string(),
        ),
        (
            "bug137-tuple-shadow-tree-c",
            r#"
#[no_mangle]
fn keep_alive() -> i64 { return 0; }
fn divmod(a: i64, b: i64) -> (i64, i64) {
  return (a / b, a % b);
}
fn main() -> i64 {
  let (q, r) = divmod(17, 5);
  let (q, r) = divmod(20, 3);
  return q + r;
}
"#
            .to_string(),
        ),
    ] {
        let src = write_tmp_vani(stem, &source);
        let output_c = Command::new(binary)
            .args(["run", src.to_str().unwrap(), "--backend=c"])
            .output()
            .unwrap_or_else(|e| panic!("intentc run --backend=c should execute: {e}"));
        assert_eq!(
            output_c.status.code(),
            Some(8),
            "BUG-137 regression ({stem}): expected exit 8 (20/3 = 6 rem 2, \
             6+2=8), got status {:?}, stderr: {}",
            output_c.status,
            String::from_utf8_lossy(&output_c.stderr)
        );

        let output_llvm = Command::new(binary)
            .args(["run", src.to_str().unwrap()])
            .output()
            .unwrap_or_else(|e| panic!("intentc run should execute: {e}"));
        assert_eq!(
            output_llvm.status.code(),
            Some(8),
            "{stem}: LLVM backend should agree with C; status {:?}, stderr: {}",
            output_llvm.status,
            String::from_utf8_lossy(&output_llvm.stderr)
        );
    }
}

// BUG-139 (2026-08-07): found via localfuzz. An enum variant payload
// naming a nonexistent type (`String` instead of `OwnedStr`) used to be
// silently accepted at declaration time as long as no variant construction
// ever forced a real type lookup -- `vanic check` exited 0 with no
// diagnostic. Verifies the real CLI path (not just the compile() library
// helper) now rejects it immediately, before either backend is even
// attempted.
#[test]
fn enum_variant_with_unknown_payload_type_rejected_by_vanic_check() {
    let binary = env!("CARGO_BIN_EXE_intentc");
    let src = write_tmp_vani(
        "bug139-unknown-enum-payload-type",
        r#"
enum Result { Ok(i64), Err(String) }
fn main() -> i64 { return 0; }
"#,
    );
    let output = Command::new(binary)
        .args(["check", src.to_str().unwrap()])
        .output()
        .unwrap_or_else(|e| panic!("intentc check should execute: {e}"));
    assert!(
        !output.status.success(),
        "BUG-139 regression: an enum variant payload naming a nonexistent \
         type should be rejected by `vanic check`, got status {:?}",
        output.status
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("unknown type 'String'"),
        "expected an unknown-type diagnostic on stderr, got: {stderr}"
    );
}

// BUG-140 (2026-08-07): found via localfuzz. `parallel for i from
// i64::MIN to 4 { ... xs[i] ... }` on the C backend used to silently
// execute ZERO iterations (GCC's OpenMP canonical-loop trip-count
// computation is UB when the true iteration count overflows the loop
// variable's type), returning the untouched initial reduction value
// instead of trapping -- while LLVM correctly trapped on the same
// program. Verifies both the pathological case now traps identically
// on both backends, AND that a normal, non-pathological `parallel for`
// still computes the correct answer (no false-positive regression).
#[test]
fn parallel_for_extreme_start_bound_traps_instead_of_silently_skipping_on_c() {
    let binary = env!("CARGO_BIN_EXE_intentc");
    let src = write_tmp_vani(
        "bug140-parallel-for-i64-min",
        r#"
fn main() -> i64 {
  let xs: [i64; 4] = [1, 2, 3, 4];
  let prod: i64 = 1;
  parallel for i from -9223372036854775808 to 4
  reduce prod with *;
  {
    prod = prod * xs[i];
  }
  print prod;
  return 0;
}
"#,
    );
    let output_c = Command::new(binary)
        .args(["run", src.to_str().unwrap(), "--backend=c"])
        .output()
        .unwrap_or_else(|e| panic!("intentc run --backend=c should execute: {e}"));
    assert_eq!(
        output_c.status.code(),
        Some(134),
        "BUG-140 regression: C backend should trap (134) on this extreme \
         loop range instead of silently skipping the loop, got status {:?}, \
         stdout: {}, stderr: {}",
        output_c.status,
        String::from_utf8_lossy(&output_c.stdout),
        String::from_utf8_lossy(&output_c.stderr)
    );
    assert!(
        String::from_utf8_lossy(&output_c.stderr).contains("integer overflow"),
        "expected an overflow message on stderr, got: {}",
        String::from_utf8_lossy(&output_c.stderr)
    );

    let output_llvm = Command::new(binary)
        .args(["run", src.to_str().unwrap()])
        .output()
        .unwrap_or_else(|e| panic!("intentc run should execute: {e}"));
    assert_eq!(
        output_llvm.status.code(),
        Some(3),
        "LLVM should also trap on this same program; status {:?}",
        output_llvm.status
    );

    // Normal case: no false-positive regression.
    let normal_src = write_tmp_vani(
        "bug140-parallel-for-normal",
        r#"
fn main() -> i64 {
  let xs: [i64; 4] = [1, 2, 3, 4];
  let prod: i64 = 1;
  parallel for i from 0 to 4
  reduce prod with *;
  {
    prod = prod * xs[i];
  }
  print prod;
  return 0;
}
"#,
    );
    let output_normal = Command::new(binary)
        .args(["run", normal_src.to_str().unwrap(), "--backend=c"])
        .output()
        .unwrap_or_else(|e| panic!("intentc run --backend=c should execute: {e}"));
    assert!(
        output_normal.status.success(),
        "a normal parallel_for loop should not trap; status {:?}, stderr: {}",
        output_normal.status,
        String::from_utf8_lossy(&output_normal.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&output_normal.stdout).trim(),
        "24",
        "expected the correct product 24, got: {}",
        String::from_utf8_lossy(&output_normal.stdout)
    );
}
