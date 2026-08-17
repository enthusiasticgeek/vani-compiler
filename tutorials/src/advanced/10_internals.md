# Advanced 10 -- Compiler internals tour

> **Learning goal**: orient yourself in the vāṇī compiler
> source tree so you can read a diagnostic, find the
> responsible pass, and contribute a fix.

A compiler is an assembly line that transforms text into a
running program in five broad stages:
1. **Lexer** -- reads characters, groups them into tokens
   (`let`, `42`, `+`). Like a copy editor who circles every
   word on the page.
2. **Parser** -- reads tokens, builds a tree of the program's
   structure (the AST). Like the editor grouping circled words
   into sentences and paragraphs.
3. **Checker** -- verifies types, ownership, contract
   obligations. Like a fact-checker who verifies every claim
   before publication.
4. **Codegen** -- translates the checked tree into LLVM IR or C.
   Like a translator who renders the document in another
   language.
5. **Backend** -- LLVM compiles IR to machine code; `cc`
   compiles C.

Each Rust source file in `src/` maps closely to one stage.
This chapter walks each one.

## The pipeline at 30,000 ft

```
.vani source
    |
    |  src/lexer.rs        -- tokens + script-purity gate
    v
  tokens
    |
    |  src/parser.rs       -- recursive-descent -> ast::Program
    v
   AST  (src/ast.rs)
    |
    |  src/checker.rs      -- type-check + affine ownership +
    |                        SMT discharge (via src/smt.rs)
    v
  TypedProgram  (src/ir.rs)
    |
    |  src/ssa.rs           -- TypedProgram -> SSA Module (for
    |                        the SSA backends; tree-* skip this;
    |                        confirmed by testing -- an earlier
    |                        version of this page said `src/lower.rs`,
    |                        which doesn't exist)
    v
   IR Module  (src/ir.rs)
    |
    +-> src/backend_c.rs       -- tree-C emit  (canonical C)
    +-> src/ssa_backend_c.rs   -- SSA-C emit (preferred when supported)
    +-> src/backend_llvm.rs    -- tree-LLVM emit (canonical LLVM IR)
    +-> src/ssa_backend_llvm.rs -- SSA-LLVM emit
```

Each backend gets the same `TypedProgram`; the SSA backends
additionally lower to `Module` first. The driver (`src/main.rs`)
tries the SSA path; if a feature isn't supported there, it
falls back to the tree backend automatically.

## Module-by-module pointers

All function names below confirmed by testing/grepping the real
source (an earlier version of this table had six wrong names --
`parse_fn_decl`, `check_program`, `check_call_args`,
`check_dyn_coerce`, `encode_predicate`, `discharge_proof`, and
`emit_program` don't exist anywhere in the codebase; `src/lower.rs`
doesn't exist as a file at all).

| File | Purpose | What to look for |
|---|---|---|
| `src/lexer.rs` | Tokens + multi-script purity | `Script::classify`, `enforce_language_purity`, the per-script `*_keyword` functions |
| `src/parser.rs` | Recursive-descent parser | `parse_function`, `parse_let_stmt`, the SOV-shape detectors |
| `src/ast.rs` | Untyped AST | `enum Stmt`, `enum Expr`, the `CLOSURE_MAKE_REGISTRY` thread-locals |
| `src/checker.rs` | Type checker + affine borrow checker | `check` (the top-level entry point), `check_call`, `make_dyn_coerce` |
| `src/smt.rs` | SMT encoder (Z3 backend) | `try_prove` (the top-level discharge entry point, returns a `Verdict`), `build_query`, the `VANIC_SMT_DEBUG` env var |
| `src/ir.rs` | TypedProgram + IR Module | `TypedExprKind`, `Instruction`, `BasicBlock` |
| `src/ssa.rs` | TypedProgram -> SSA Module | `lower_program`, `lower_expr_to_value`/`lower_expr_to_operand` |
| `src/backend_c.rs` | Tree-C codegen | `emit_c` (the real entry point behind `CBackend::emit`), `emit_print_expr_no_newline`, the per-script numeral helpers |
| `src/ssa_backend_c.rs` | SSA-C codegen | `emit`, `intent_print_item` handler |
| `src/backend_llvm.rs` | Tree-LLVM codegen | `emit_llvm`, `emit_print_items`, `emit_brahmi_print_helper_ll` |
| `src/ssa_backend_llvm.rs` | SSA-LLVM codegen | `emit`, the LLVM IR helper emitter |
| `src/diagnostic.rs` | Error rendering | `localize_label`, `localize_message`, the per-dialect prefix tables |
| `src/main.rs` | CLI driver | Subcommand dispatch, the SSA-vs-tree fallback logic |

## How a compile flows

When you run `vanic run foo.vani`:

1. **Manifest discovery** (`src/main.rs`, `manifest::find_manifest`).
   If no source file is given, walk up looking for `vani.toml`.
2. **File resolution** (`src/lib.rs::resolve_uses`).
   Recursively splice every `use "path";` file into one
   combined source. Records each file's contribution in a
   `FileMap` so diagnostics can be reverse-mapped.
3. **Lexing** (`src/lexer.rs::lex`). Produces tokens; runs the
   script-purity gate; records the file's `PrintLangMode`
   into a thread-local so backends can pick the right numeral
   helper.
4. **PRELUDE injection** (`src/lib.rs::inject_prelude`). Lexes
   the prelude source and merges it into the user's program.
   *Note: this used to clobber the PrintLangMode set in step
   3 -- Phase 1.1 fixed it by saving/restoring around the
   inject call.*
5. **Parsing** (`src/parser.rs`).
6. **Type checking** (`src/checker.rs`). Produces a
   `TypedProgram` with every expression annotated.
7. **SMT discharge** (`src/smt.rs`). For every `prove`,
   `assert`, `requires` at a call site, `ensures` at a return,
   and loop `invariant`, encode the predicate as Z3 input and
   discharge. Failures surface as diagnostics with the
   counterexample.
8. **Codegen**. SSA backend tries first; falls back to tree
   on any unsupported feature. The output is C or LLVM IR.
9. **Linking + run**. Tree-C invokes `cc`; LLVM uses `lli` to
   JIT or `llc + cc` to produce a binary.

## SSA vs. tree codegen: what actually gates the fallback

Step 8 above says "SSA backend tries first; falls back to tree
on any unsupported feature" -- true, but that sentence hides a
few sharp edges worth knowing before you add a language feature
or a builtin.

**Where the gate lives**: `src/main.rs`, not the backends
themselves. `emit_llvm_via_ssa` / `emit_c_via_ssa` each call
`ssa_path_supports(ir, extra_reject)`, which walks every
function's param types, return type, and body:

- `ssa_type_supported(ty)` -- can this *type* appear in an SSA
  signature at all?
- `extra_reject(stmt)` -- backend-specific statement/expression
  exclusions, e.g. `ssa_llvm_extra_reject` (checked only for the
  LLVM path) and `ssa_c_extra_reject` (C path). Each is usually
  an exhaustive recursive walker over `TypedStmt`/`TypedExpr`
  looking for one specific unsupported shape -- see
  `stmt_uses_vec_of_atomic_or_channel` or
  `stmt_calls_f64_to_str_fixed` for the pattern to copy.

Only if every function clears both checks does the driver call
`lower_program` + `ssa_backend_llvm::emit` / `ssa_backend_c::emit`;
otherwise (or if that `Ok`/`Err` comes back `Err`) it calls
`LlvmBackend.emit(ir)` / `CBackend.emit(ir)` -- the tree path --
instead.

**The gate is whole-program, not per-function or per-call-site.**
`ssa_path_supports` returns one `bool` for the entire
`TypedProgram`. If ONE function anywhere in the program uses an
unsupported shape, the ENTIRE program -- every function, not just
the offending one -- gets tree-codegen'd for that target. Output
is correct either way; but if you're diffing generated C/LLVM IR
and a program looks unexpectedly tree-shaped, this is why.

**What's currently gated out is not exhaustively documented
anywhere** -- treat the examples below as illustrative, not a
complete list; `git grep extra_reject` in `main.rs` is the actual
source of truth:
- Payloaded enums force tree-LLVM (SSA-LLVM has no tagged-union
  codegen yet); SSA-C handles them fine.
- `Vec<Atomic<T>>` / `Vec<Channel<T,N>>` force tree-LLVM (closure
  #212 -- SSA-LLVM's vec-literal emit assumes a value-shaped
  element, and Atomic/Channel are pointer-shaped).
- `f64_to_str_fixed` forces both tree-C and tree-LLVM (neither
  SSA backend has an implementation).

<img class="manas" src="../images/mascot/manas_mascot_caution.png" title="this code needs extra care"/>

**The trap**: adding a new builtin to `checker.rs` + the tree
backends does **not** make it safe to ship on the SSA path by
default -- and there's no automatic detection that it's missing.
Neither SSA backend's `Call`-lowering has an error path for a
name it doesn't recognize; it silently assumes "must be a
user-defined function," mangles the callee to `fn_<name>`, and
the program only fails at LLVM-verify / link time with an
*undefined symbol* -- not at compile time, and not with a
diagnostic that points anywhere near the real cause. This is
exactly the bug that shipping `f64_to_str_fixed` surfaced (full
writeup: item 27.1 in
[`docs/TODO_CURRENT.md`](https://github.com/enthusiasticgeek/vani-compiler/blob/main/docs/TODO_CURRENT.md)).

Two consequences for anyone adding a language feature or builtin:
1. **Test through `vanic run` / `vanic emit`**, not just
   `compile_to_c` / `compile_to_llvm`. The latter two (what
   `src/lib.rs`'s ~1900 tests mostly use) call the tree backends
   *directly*, bypassing the SSA path entirely -- they would not
   have caught the bug above.
2. If the new construct isn't SSA-supported, add a
   `stmt_uses_<thing>` / `expr_uses_<thing>` walker (copy the
   `f64_to_str_fixed` or `vec_of_atomic_or_channel` pair) and OR
   it into `ssa_llvm_extra_reject` / `ssa_c_extra_reject` as
   appropriate. Add a regression test asserting
   `emit_llvm_via_ssa` / `emit_c_via_ssa` output actually reaches
   the tree backend's symbol name (`intent_<thing>`) and not a
   `fn_<name>` mangle -- the compile_to_c/compile_to_llvm test
   suite can't see this layer at all.

## How to contribute a fix

1. **Find the failing test or symptom**. The test ledger lives
   in `src/lib.rs` (2400+ tests as of 2026-08-01, up from an
   earlier "1900+" -- the count only grows, check
   `grep -c '#\[test\]' src/lib.rs` for the current figure rather
   than trusting a number in prose) and `tests/run_end_to_end.rs`
   (100+ tests, similarly grown from an earlier "54").
2. **Reproduce locally**: `cargo test --release --lib <name>`.
3. **Trace the diagnostic** with `VANIC_SMT_DEBUG=1` if it's an
   SMT failure; for codegen issues, run `vanic emit foo.vani
   --backend=c` and read the C output.
4. **Find the right module** in the table above.
5. **Pin your fix with a test**: every Phase 1+ commit adds a
   `*_pragma_compiles_and_emits_*` or similar regression to
   make sure the fix doesn't unravel.
6. **Run the full sweeps** before pushing: `cargo test --release
   --lib && cargo test --release --test run_end_to_end`.

## How the test ledger is organized

| Suite | Where | What it covers |
|---|---|---|
| Lib tests (2400+) | `src/lib.rs` | Per-pass unit tests -- checker, lower, codegen shape pins, regression for each phase |
| Parity + regression sweep (100+) | `tests/run_end_to_end.rs` | Real `vanic run`/`emit` invocations through the actual CLI (the only layer that exercises the SSA-vs-tree fallback -- see the warning above) -- backend-parity tests, real end-to-end bug regressions, and the `llvm_backend_run_produces_same_output_as_c` test, which iterates 60+ examples internally |
| Example walks | `tests/run_end_to_end.rs` | Cover the design-pattern + dialect examples |

## Where the dialect surface lives

All dialect-related source-of-truth files:

- **Lexer**: `src/lexer.rs` -- `DialectLang`, `Script`,
  `PrintLangMode`, `enforce_language_purity`, per-script
  `*_keyword` functions.
- **Diagnostic**: `src/diagnostic.rs` -- `DiagLang`,
  `localize_label`, `localize_message`.
- **Backends**: per-script numeral helpers in all four backend
  files (search for `intent_print_int_`).
- **Translator**: `tools/vani_translate.py::ALIASES`.
- **LLM context bundle**: `tools/llm_context/bundle.py`.
- **Docs**: `README.md`'s Tier-I/II tables;
  `docs/v1_limitations.md`.

## When you'd peek into internals

- Writing a new dialect (Sec.9). The pipeline above is your
  roadmap.
- Debugging a "proof failed" you can't reduce. `VANIC_SMT_DEBUG=1`
  + `src/smt.rs` are the answer.
- Building a tool that consumes vāṇī AST. `cargo run --release
  --bin vanic -- ast foo.vani` prints the AST -- as Rust's `{:#?}`
  pretty-debug format, NOT JSON (confirmed by testing; an earlier
  version of this page overclaimed JSON, and there's no `--json`
  flag on this subcommand to get it). Skips the type checker, so it
  still shows you something even for a program the checker would
  reject.
- Contributing a new backend feature. Start by reading two
  parallel backend files (tree-C and SSA-C) side by side --
  the shapes line up.

## Source-of-truth doc

The repo's [`README.md`](https://github.com/enthusiasticgeek/vani-compiler/blob/main/README.md)
has the running language reference;
[`STATUS.md`](https://github.com/enthusiasticgeek/vani-compiler/blob/main/STATUS.md)
is the time-ordered changelog;
[`docs/v1_limitations.md`](https://github.com/enthusiasticgeek/vani-compiler/blob/main/docs/v1_limitations.md)
is the canonical catalog of documented v1 deviations.

---

**Congratulations -- you've completed the Advanced track!**

That's the whole tutorial set (Beginner + Intermediate +
Advanced -- 34 lessons). The next-best thing is to:

- Read `examples/language/english/` end to end. With all three
  tracks behind you, every file should be navigable.
- File issues for rough patches you hit. The compiler's most
  honest design feedback is from real programs.
- Contribute a fix or a dialect. The execution plan in
  [`TODO.md`](https://github.com/enthusiasticgeek/vani-compiler/blob/main/TODO.md)
  has phase-by-phase queued work; pick whatever calls to you.


---

**Previous**: [Sec.9 -- Adding a new dialect ->](09_new_dialect.md)

**Next**: [Sec.11 -- Using vani with an LLM ->](11_llm_workflows.md)
