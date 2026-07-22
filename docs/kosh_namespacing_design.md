# Kosh package namespacing + dependency-graph resolution — design doc

**Status:** Phases 1-5 shipped 2026-07-21. Phase 6 planned, not
started. **Phase 3 still breaks 8 of the 12 published kosh packages
until Phase 6's migration runs** — see Phase 3's "Verified" section.
**Authored:** 2026-07-21.

---

## Problem statement

Two related bugs, both surfaced the same day by a direct user question
("what happens if a kosh-index package has the same function name as a
vāṇी built-in — namespace or modules?") followed by hands-on testing:

### Bug 1 — no per-package namespace, flat global names

Every `[deps]` entry's source is textually concatenated into one
combined buffer before type-checking ever runs (`compile_path` /
`resolve_uses` in `src/lib.rs`). There is no per-package module
wrapper, no `pkgname::item` qualification. `collect_signatures`
(`checker.rs`) checks every top-level function name in that combined
program against the ~500-name `BUILTIN_FUNCTION_NAMES` list and against
every other top-level function; any collision — whether against a
vāṇी builtin or against another package's function — is a hard
compile error, not shadowing, not priority-based resolution:

```
error: function 'abs' is a built-in name and cannot be redefined
```

Verified directly: `fn abs(x: i64) -> i64 { ... }` in any file that
gets concatenated into the program fails this way, unconditionally.
This means Kosh package authors currently avoid collisions only by
*manual convention* — see e.g. vani-matrix's header comment
`// BUILTINS (do NOT reimplement): abs, sqrt, pow, exp, log, sin, cos, ...`
— never by any compiler-enforced boundary. At ecosystem scale this
doesn't hold: **package authors cannot control each other's function
names**, and a same-named helper in two unrelated packages that happen
to both get pulled into one project is an unrecoverable compile error
with no workaround short of forking and renaming.

### Bug 2 — transitive dependencies were silently missing, or (with the
### Phase 1 fix in place) exposed a real version-conflict blocker

`manifest::load_manifest` already recurses into each dependency's own
`vani.toml` — but only to validate it exists and read its `entry_path`;
it silently discarded that dependency's *own* `[deps]`. `compile_path`
then walked only the top-level project's direct deps, one level.

Verified directly with a scratch project depending on both `probability`
and `optimize` (both real, published kosh packages, each independently
vendoring their own copy of `vani-matrix`): compiling failed with
`unknown function 'mat_transpose'` / `'mat_mul_rect'` / `'mat_inv_n'` —
`vani-matrix`'s functions were not part of the combined program at all,
despite both vendored copies existing on disk. The only reason any
existing package worked was that, before this session's MAINT-2 cleanup
(commit history 2026-07-21), each package's `lib.vani` still had an
explicit `use "../vendor/<dep>/src/lib.vani";` line — which happened to
work because `resolve_uses` recursively follows literal `use`
statements regardless of nesting depth. MAINT-2 removed those lines as
"redundant" (true only when the package is the *top-level* compiled
entry, false when it's consumed as someone else's dependency) — which
is what turned a latent design gap into an active regression across 9
published packages.

---

## Root design decision: reuse the existing module system, don't invent one

vāṇी already ships a real `module { }` / `use foo::bar;` / `pub` /
`pub(kosh)` system (`docs/namespaces_design.md`, closures #242-258).
Notably, `pub(kosh)` is already documented as *"preparatory; behaves as
`pub` today, enforces at the future kosh boundary"* — this feature was
built with exactly this problem in mind and never finished.

**Decision:** each `[deps]` entry gets implicitly wrapped in
`module <pkg_name> { ... }` at compile time (a compiler-internal
wrapping step before concatenation — no source rewriting of the
dependency's files needed). Consumers reference dependency functions as
`matrix::mat_mul(...)` or import them with the existing
`use matrix::mat_mul;`. No new syntax; only new compiler behavior in
how `[deps]` sources are folded into the combined program.

**Correction (made once Phase 3 implementation started, see below):**
the original draft of this doc had `pub`/`pub(kosh)` backwards. The
actual documented semantics in `ast.rs` (`ModuleVisibility`'s doc
comment) are: `pub(kosh)` means *"exported within this kosh but NOT
through the kosh boundary into external dependents"* — i.e. visible
across a package's own internal modules, but never crossing into
`pkgname::item` for an external consumer. Plain `pub` is the one with
no such restriction — it's what crosses the boundary. So: plain `pub`
items are visible to external consumers as `pkgname::item`; `pub(kosh)`
items stay internal to the package's own module tree (useful for a
package with multiple internal modules sharing helpers without exposing
them as part of the public API). Phase 3's actual v1 implementation
sidesteps needing package authors to write either annotation at all —
see "Visibility default flipped" below.

---

## Phases

### Phase 1 — Real transitive dependency graph ✅ shipped 2026-07-21

Fixes Bug 2 (missing functions). Does **not** fix Bug 1 (namespacing) —
that's Phase 3.

- `manifest::resolve_transitive_deps(&Manifest) -> Result<Vec<Dependency>, String>`
  (new, `src/manifest.rs`): recursively walks `[deps]`, and each
  dependency's own `[deps]`, transitively. Returns the flattened set of
  every package reachable from the root manifest, in name-sorted
  (deterministic) order.
- **Identity for dedup is `(name, resolved_version)`, not file path.**
  This is the actual diamond-dependency fix: `probability`'s vendored
  `matrix` and `optimize`'s vendored `matrix` are two different
  canonical file paths on disk but must resolve to one compiled copy
  when both versions agree.
- **Version conflict = hard error.** v1 requires a single resolved
  version per package name across the whole graph (no per-edge version
  resolution the way Cargo does — not needed at current ecosystem
  scale, and an explicit non-goal for now). Diagnostic:
  `dependency version conflict: 'matrix' is required as 0.1.0 via one
  path and 0.2.0 via another`.
- **Cycle safety net.** A `visiting: HashSet<PathBuf>` tracks the
  current DFS path (canonical manifest paths, pushed before recursing
  into a dep, popped after) so a package that's (accidentally)
  reachable from itself cannot loop forever. This is a safety net, not
  a diagnostic — Phase 2 gives the real cycle-chain error message.
- Wired into all three compilation entry points that used to do the
  one-level walk: `compile_path`, `compile_library_path`,
  `resolve_combined_source` (`src/lib.rs`). `vanic vendor`'s direct-deps
  vendoring behavior (`main.rs`, `manifest::vendor_deps`) is
  intentionally untouched — Phase 1 only fixes dependency-source
  resolution for *compilation*, not the on-disk vendor layout.

**Verified**:
- The exact repro above (`probability` + `optimize` sharing `matrix`)
  no longer reports missing functions. Instead it correctly reports a
  **real, previously-invisible bug in the published ecosystem**:
  `vani-probability` vendors `vani-matrix` v0.1.0, `vani-optimize`
  vendors v0.2.0 — genuine version drift between two published
  packages' pinned copies of a shared dependency. This is a MAINT-item
  for kosh-index, tracked separately (align both to the same matrix
  version and republish).
- Manually aligning both vendored copies to the same version (0.2.0) in
  a scratch test made the diamond resolve cleanly — single compile, no
  duplicate-definition error, `ok: src/main.vani`.
- Regression swept against all 12 real kosh packages via
  `vanic audit-safety` (exercises the same `resolve_transitive_deps`
  path for every package's normal single-dependency case) — all still
  pass cleanly. Two test-file spot checks (`vani-probability`,
  `vani-optimize`) still compile correctly end-to-end (`vanic check`,
  full SMT verification included).

### Phase 2 — Circular dependency detection ✅ shipped 2026-07-21

- `manifest::check_dependency_cycles(root_name, root_manifest_path)`
  reuses the exact Tarjan SCC implementation that backs
  `vanic acyclicity`'s function-call-graph analysis
  (`src/acyclicity.rs::tarjan_scc`, made `pub(crate)` — it was already
  generic over any `HashMap<String, Vec<String>>` adjacency, no
  algorithm changes needed) against the *package* graph instead.
- Deliberately does **not** build its graph via `load_manifest`.
  Explained below — this was the crux of getting Phase 2 actually
  working, not just compiling.
- Any cycle (including a package accidentally listing itself) becomes
  a hard compile-time error showing the full cycle chain, checked
  before any compilation is attempted — replacing Phase 1's plain
  "circular dependency detected: 'x' is reachable from itself" with a
  real `pkg_a -> pkg_b -> pkg_a` diagnostic listing every cycle found.

**Two real bugs found and fixed while implementing this, neither of
which were introduced by Phase 1 — both pre-existing, just never
triggered before because nobody had a circular Kosh dependency to test
with:**

1. **`load_manifest`'s own recursion had zero cycle protection.**
   `load_manifest` recurses into each `[deps]` entry's own `vani.toml`
   purely to read its `entry_path`/`root_dir`/`package_version` (see
   the code as it existed before this session). A genuine cycle
   (`pkg_a` depends on `pkg_b` depends on `pkg_a`) recursed
   unboundedly and crashed/hung — verified directly: the first test
   attempt hit exactly this, independent of anything Phase 1 or Phase
   2 added. Fixed by threading a `visiting: HashSet<PathBuf>` DFS-path
   guard through an internal `load_manifest_impl` (pushed before
   recursing into a dep, popped after a successful load — so a
   legitimate diamond, where the same manifest is reached via two
   different non-overlapping paths, is unaffected).

2. **Consequently, `check_dependency_cycles` could not be built as a
   thin wrapper around `load_manifest`.** The natural first attempt —
   build the package graph by calling `load_manifest` on each node —
   fails precisely on cyclic input, since that's now correctly
   rejected via bug 1's fix. Every caller in `lib.rs` used the pattern
   `if let Ok(m) = manifest::load_manifest(...) { ...walk m.deps... }`,
   which silently skips the entire `[deps]` block on any `Err` —
   meaning a real cycle produced a *false negative*: no error at all,
   dependencies silently dropped, and (in this session's exact test
   case) the program "succeeded" because the trivial `main()` never
   actually needed anything from the broken dependency chain. Verified
   directly: the first working build of Phase 2 reported `ok` on a
   deliberately circular `pkg_a <-> pkg_b` fixture instead of an error.

   Fixed by making `check_dependency_cycles` build its graph from
   `parse_toml_minimal` directly — the same non-recursive raw-TOML
   parse `load_manifest` itself uses internally, but without the
   recursive entry-path resolution that makes `load_manifest` fragile
   on cyclic input. Cycle-safety during graph *construction* comes
   purely from a `loaded: HashSet<PathBuf>` (each manifest file visited
   at most once while building adjacency); Tarjan SCC over the
   resulting complete graph is what actually finds the cycles. A new
   `check_cycles_before_load` entry point runs this check against the
   *root* manifest before `compile_path`/`compile_library_path`/
   `resolve_combined_source` ever call `load_manifest` on it — the one
   call site that must be guarded before the fact, not after, since a
   root-level cycle makes `load_manifest(root)` itself the thing that
   fails.

**Verified end-to-end** with a real 3-package fixture
(`proot` → `pkg_a` → `pkg_b` → `pkg_a`): `vanic check` now reports
```
error: circular dependency detected in the Kosh dependency graph:
  pkg_a -> pkg_b -> pkg_a

vāṇी does not support circular package dependencies. Break the cycle
by removing one of the [deps] edges shown above.
```
cleanly and immediately (no hang, no false success), while the earlier
Phase 1 diamond-dependency fixture (`probability` + `optimize` sharing
`matrix`, versions aligned) still resolves correctly with no false
cycle report. Regression-swept clean against all 12 real kosh packages.

### Phase 3 — Automatic per-package namespacing ✅ shipped 2026-07-21

Fixes Bug 1 (name collisions) — the actual thing this whole arc was
started to fix.

- Each resolved dependency (from Phase 1's flattened graph) is wrapped
  in a synthetic `module <pkg_name> { ... }` — **textually**, not via
  an AST merge: `wrap_deps_into_combined` (`src/lib.rs`) pushes the
  `module <name> {` header directly into the same buffer `resolve_uses`
  appends into, then the closing `}` after. This was a deliberate
  simplification over a "proper" AST-level merge (parse each
  dependency separately, splice its items into `program.modules`) —
  it reuses the entire existing single-string/single-parse pipeline
  unchanged, and `resolve_uses`'s span-tracking (`file_map.push`,
  based on `out.len()` at the time it runs) stays correct automatically
  as long as the wrapper header lands in the buffer first.
- **Visibility default flipped for wrapped modules only.** Existing
  kosh packages carry zero `pub`/`pub(kosh)` annotations anywhere —
  they were written assuming a flat global namespace where every
  top-level item was already implicitly reachable. Plain module
  semantics default every item to *private*, which would make a
  naively-wrapped dependency's entire surface invisible even via
  `pkgname::item`. `mark_kosh_boundary_modules_pub` (`src/lib.rs`,
  called from `compile_with` after parsing) force-sets every visibility
  bit to `pub` for any top-level module whose name is a known dependency
  package — regardless of what the source actually wrote. This is a
  deliberate v1 simplification, not an oversight: it's strictly no more
  permissive than today's status quo (everything was already callable
  by anyone who included the file), it only adds the namespace
  qualification requirement. True per-item encapsulation (an author
  deliberately hiding some items from consumers) is an explicit
  non-goal — layering it in later needs no migration, since it would
  only ever make some already-visible items private, never the reverse.
- Package names are validated as legal vāṇी identifiers before wrapping
  (`is_valid_vani_identifier`) — a hyphenated or otherwise invalid
  `[package].name` gets a clear diagnostic instead of a confusing parse
  error deep inside the wrapped dependency source. (`vanic publish`-time
  validation of this, so a bad name is caught at publish rather than at
  every consumer's compile, is still open — folded into Phase 5.)
- Combined with Phase 1's identity-based dedup, this is what kills both
  original bugs together: a shared transitive dep is compiled exactly
  once and exposed under one namespace, regardless of how many
  dependents pull it in or how deeply it's vendored. Verified directly
  with a 4-package diamond fixture (`proot` depending on `pkg_x` and
  `pkg_y`, both depending on `shared`): `pkg_x::triple_via_shared`,
  `pkg_y::quad_via_shared`, and a direct `shared::double` call from
  `proot` itself (via Phase 1's transitive flattening) all produced
  correct results in one run.

**A real parser gap found and fixed along the way, unrelated to the
namespacing logic itself**: module bodies had no dispatch branch at all
for `#[attr]`-prefixed items (`TokenKind::Hash`) — every real kosh
package leans heavily on `#[bounded_stack(...)]`/`#[wcet(...)]`
attributes (MAINT-1, this same session), which nobody had ever tried
wrapping inside a `module { }` block before, since the module system
predates any use case that would combine the two. Fixed by adding a
`Hash` branch to the module-body item dispatch (`parser.rs`) that calls
the exact same `parse_attributed_fn` top-level items already use — no
new parsing logic, just a missing wire-up.

**Verified**:
- The original motivating question, directly: a dependency package
  defining `fn abs(x: i64) -> i64 { ... }` (colliding with the vāṇी
  builtin `abs`) now compiles and runs correctly — `abs(-7)` (builtin)
  and `mypkg::abs(-7)` (package function) both resolve and both return
  `7`, zero collision error.
- An unqualified call to a dependency function (`square(5)` instead of
  `mathlib::square(5)`) now correctly fails with "unknown function" —
  proving real namespace isolation, not just that qualified calls
  happen to also work.
- The diamond fixture above.
- **Expected, deliberate breakage of the published ecosystem**: 8 of
  the 12 real kosh packages that declare `[deps]` (`vectorcalc`,
  `algebra`, `pde`, `interval`, `tensor`, `signal`, `optimize`,
  `probability`) now fail `vanic audit-safety` with "unknown function"
  errors at their own internal calls into their dependencies (e.g.
  `vani-pde` calling `mat_zeros(...)` instead of
  `matrix::mat_zeros(...)`) — because none of them use qualified syntax
  yet. This is the anticipated breaking change, not a bug; fixing it is
  Phase 6's job. The 4 self-contained packages with no `[deps]`
  (`complex`, `discrete`, `sparse`, `geometry`) are unaffected and still
  pass cleanly.

### Phase 4 — `vani.lock` becomes a real lockfile ✅ shipped 2026-07-21

- `write_lockfile` now calls `resolve_transitive_deps` instead of
  walking only `manifest.deps` — every package reachable through the
  graph gets a `[[package]]` entry, not just direct ones.
- Direct deps keep the existing `path`/`version-req` fields (meaningful
  relative to the root project) plus a new `direct = true` marker.
  Transitive-only deps get `direct = false` and a canonicalized
  absolute `root-path` instead of `path` — there's no single
  well-defined "path relative to root" for a package that might be
  vendored at different nesting depths by different dependents in the
  same graph (verified: `vani.lock` today is write-only, nothing parses
  it back except an mtime check in `lockfile_is_stale`, so this only
  needs to be an accurate snapshot, not round-trip-parseable).
- Verified against a 3-package transitive fixture (`proot` → `pkg_x` →
  `shared`): `vani.lock` correctly lists `shared` with `direct = false`
  and a real absolute `root-path`, alongside `pkg_x` with
  `direct = true` and its relative `path`.

### Phase 5 — Migration UX ✅ shipped 2026-07-21

- **Compiler diagnostic**: `checker.rs`'s "unknown function" path now
  calls `module_suggestion_for`, which scans the signature table for a
  `<module>__<name>` mangled match (module separators normalized back
  to `::`, private/`__priv__` matches skipped since those already get
  their own diagnostic). When found, the error becomes:
  ```
  error: unknown function 'mat_zeros'
    help: 1. No function named `mat_zeros` is visible at this call site -- but `matrix::mat_zeros` exists.
    help: 2. Did you mean `matrix::mat_zeros`? ...
    help: 3. If `matrix` is a Kosh package dependency, every call site ... needs updating to `matrix::mat_zeros` ...
  ```
  Verified directly against the real (Phase-3-broken) `vani-pde`
  package — every failure site got the exact correct suggestion,
  effectively turning the error output into a ready-made fix list for
  Phase 6's migration.
- **A second real bug found via this same diagnostic work, this one in
  `vanic add` itself, not the namespacing logic**: `registry_add`
  wrote the raw registry package name as the `[deps]` key verbatim.
  The real published `hello-kosh` package (verified: exists in
  `kosh-index`, `[package].name = "hello-kosh"`) has a hyphen, which
  fails `is_valid_vani_identifier` — meaning the *default*, documented
  `vanic add hello-kosh` workflow generated a `vani.toml` that failed
  to compile with no path to fix it short of hand-editing. Fixed with
  `sanitize_dep_key` (`manifest.rs`): non-identifier characters become
  `_`, a leading digit gets a `_` prefix; applied to the `[deps]` key
  only — the vendored directory and registry lookups still use the
  real, unsanitized name. `vanic add` now prints a note when it
  sanitizes: ``note: 'hello-kosh' isn't a valid vāṇī identifier, so it's
  added to [deps] as `hello_kosh`.`` Verified end-to-end against the
  real package: `vani.toml` now gets `hello_kosh = { path =
  "./vendor/hello-kosh", ... }` and the identifier error is gone
  (hello-kosh's own source has an unrelated, pre-existing bug —
  `fn greet() -> str` should be `Str` — surfaced only because this may
  be the first time anything actually compiled against it; out of
  scope here, not touched).
- **Docs updated**: `docs/kosh_design.md` (dependency calling
  convention + transitive resolution note),
  `docs/namespaces_design.md` (corrected the `pub`/`pub(kosh)` mixup
  from Phase 3, updated the "still queued" section),
  `tutorials/src/intermediate/16_packages.md` (rewrote the "using the
  dependency in code" section with accurate syntax and the
  `hello-kosh` → `hello_kosh` sanitization note; updated the
  `vani.lock` example for Phase 4's new fields). The `docs/TODO_CURRENT.md`
  DOC-3 claim ("`use` lines are always redundant for `[deps]`") needed
  no correction — it was already made accurate again by Phase 1's
  transitive-resolution fix; Phase 3 changes what the call *syntax*
  looks like (`pkgname::item`), not whether a `use` statement is
  needed to reach it.

### Phase 6 — Migrate and republish the ecosystem (planned)

- Update all ~12 kosh math packages' internal cross-package calls to
  qualified/`use`-imported form.
- Re-run `vanic audit-safety` + full test suites for each.
- Republish (patch/minor bump per package as appropriate).
- Specifically re-verify the `probability` + `optimize` diamond case
  compiles clean with `matrix` included exactly once, post-namespacing.
- Fix the version-drift bug Phase 1 surfaced (align `probability` and
  `optimize` to the same `matrix` version) as part of this pass, not
  before — no point re-pinning versions twice across two migrations.

---

## Non-goals (v1)

- Multiple coexisting versions of the same package in one dependency
  graph (Cargo-style per-edge resolution). Real complexity; not needed
  at current ecosystem scale (~12-15 first-party packages, no external
  contributors yet per `docs/kosh_design.md`).
- Semver-range-based version *selection* (picking the best of several
  compatible versions across the graph). v1 requires exact agreement;
  a real resolver is future work if the registry grows enough to need
  it.
