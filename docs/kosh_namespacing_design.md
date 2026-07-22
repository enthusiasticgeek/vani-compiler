# Kosh package namespacing + dependency-graph resolution — design doc

**Status:** Phase 1 shipped 2026-07-21. Phases 2-6 planned, not started.
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

`pub(kosh)` becomes the real package-boundary marker: plain `pub` stays
visible only within the package's own module tree (as today); only
`pub(kosh)` items become visible to external consumers as
`pkgname::item`. A package can therefore have internal `pub` helpers
that never leak into its public API surface.

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

### Phase 2 — Circular dependency detection (planned)

- Reuse the existing Tarjan SCC implementation (`src/safety.rs`,
  already powers `vanic acyclicity`'s function-call-graph analysis)
  against the *package* graph instead.
- Any cycle (including a package accidentally listing itself) becomes
  a hard compile-time error showing the full cycle chain, checked
  before any compilation is attempted — upgrading Phase 1's plain
  "circular dependency detected: 'x' is reachable from itself" into a
  real `A -> B -> C -> A` diagnostic.

### Phase 3 — Automatic per-package namespacing (planned)

Fixes Bug 1 (name collisions). The bulk of the design work:

- Wrap each resolved graph node's top-level items in
  `module <pkg_name> { ... }` before concatenation.
- Wire up `pub(kosh)` as described above.
- Validate at `vanic publish` time that `[package].name` is a valid
  vāṇी identifier (namespace-safe) — extend the existing
  audit-safety-style gate (`vanic::safety`, `manifest::publish_package`).
- Combined with Phase 1's identity-based dedup, this is what kills both
  bugs together: a shared transitive dep is compiled exactly once and
  exposed under one namespace, regardless of how many dependents pull
  it in or how deeply it's vendored.
- **Breaking change**: every existing kosh package's internal
  cross-package calls, and every consumer's unqualified calls to
  dependency functions, stop resolving once namespacing lands. See
  Phase 5/6.

### Phase 4 — `vani.lock` becomes a real lockfile (planned)

- Today `vani.lock` (`manifest::write_lockfile`) only pins checksums
  for *direct* deps. Extend it to record the full resolved graph
  (every transitive package, resolved version, checksum) so
  `vanic build`/`check` don't re-walk and re-resolve every `vani.toml`
  on every compile, and `vanic update` has something concrete to diff
  against.

### Phase 5 — Migration UX (planned)

- Special-case diagnostic: an unqualified call that would resolve to a
  dependency's function post-namespacing gets *"did you mean
  `matrix::mat_mul`? Kosh dependency functions now require a package
  prefix"* instead of a bare unknown-function error.
- Update `docs/kosh_design.md`, `docs/namespaces_design.md`,
  `tutorials/src/intermediate/16_packages.md` — and correct this
  session's now-superseded DOC-3 claim
  (`docs/TODO_CURRENT.md`, "Device I/O + Big-O doc audit" section) that
  `use` lines are always redundant for `[deps]` entries. That claim was
  true only for the top-level-entry case; Phase 1's fix makes it true
  again in general (transitive deps are now resolved automatically
  regardless of `use` lines), but Phase 3's namespacing changes what
  the call sites look like regardless.

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
