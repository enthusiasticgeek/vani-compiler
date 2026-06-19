# vāṇी — Current Work Queue

Actionable items fully within our control, ordered by effort.
Blocked items (macOS hardware, grammar consultant, IOCP) are at the bottom.

Last updated: 2026-06-18

---

## Immediate (< 1 h)

- [ ] **1. Publish to crates.io** — `cargo publish`. All required fields present in
  `Cargo.toml`. Gives `cargo install vanic` to Rust users. See
  [docs/decisions.md](decisions.md) for rationale.

- [x] **2. Update RELEASING.md** — Point at `0.1.2-dev`; document `RELEASE_NOTES/`
  workflow and `body_path` release step. ✅ done 2026-06-19

- [ ] **3. Remove `intentc` legacy binary** — Delete `[[bin]] name = "intentc"` from
  `Cargo.toml` at next release boundary (v0.1.x → v0.2 or when the release cycle
  ends). Add a compiler warning to `main.rs` when invoked as `intentc`.

---

## Short (2–4 h each)

- [x] **4. Add 4 missing Devanagari aliases to lexer** — `extern` / `type` / `intent`
  / `invariant` are shown in the README table but may not be wired in `lexer.rs`.
  Verify + add if missing; add lib tests. ✅ done 2026-06-19 (all 4 already wired; added tests for प्रकार + बाह्य)

- [x] **5. Groom `docs/v1_limitations.md`** — Mark limitations resolved since
  2026-06-09 ✅; add entries for parametric `Mutex<T>` (no longer i64-only),
  `Barrier`, `RwLock<T>/ReadGuard/WriteGuard`. ✅ done 2026-06-19 (L15/L16/L17)

- [x] **6. Tutorial: Barrier primer** — `tutorials/src/advanced/02b_barrier_primer.md`.
  Same format as `02a_parallelism_primer.md`. Add to `SUMMARY.md`. ✅ done 2026-06-19

- [x] **7. Tutorial: RwLock primer** — `tutorials/src/advanced/02c_rwlock_primer.md`.
  Add to `SUMMARY.md`. ✅ done 2026-06-19

- [x] **8. Tutorial: default methods + blanket impls primer** —
  `tutorials/src/intermediate/04d_default_methods_primer.md`. Add to `SUMMARY.md`. ✅ done 2026-06-19

- [x] **9. Update `tutorials/src/SUMMARY.md`** — Add the three new primer entries
  above to the book index. ✅ done 2026-06-19

---

## Medium (4–8 h each)

- [x] **10. Condense `STATUS.md` / `TODO.md`** — Both are 500 KB+. Extract
  pre-Arc-8 shipped history to `STATUS_ARCHIVE.md` / `TODO_ARCHIVE.md`. Keep main
  files as current-state ledgers. ✅ done 2026-06-19 (STATUS.md: 11741→306 lines; TODO.md: 10585→40 lines)

- [x] **11. A.2 Examples reorganization** — Verify all Devanagari examples live under
  `examples/language/{sanskrit,hindi,marathi}/`; add `// श्री।` header to each.
  Move any English examples not yet under `examples/language/english/`. ✅ done 2026-06-19
  (14 Sanskrit + 12 Hindi + 12 Marathi — all have // श्री। header; moved path_c_ref_returns.vani
  and vec_of_ref.vani from examples/ root to examples/language/english/)

- [ ] **12. Arc 7 Win64 / AArch64 ABI** — Complete float-class + mixed struct
  Win64 struct-return classifier (~6–8 h). Code work only; CI wiring is separate.

- [ ] **13. Finer Sanskrit / Hindi / Marathi purity gate** — Tighten the
  `// vani-lang:` pragma in `lexer.rs` to distinguish the three dialects (currently
  only English vs Devanagari at script level).

---

## Larger (dedicated session)

- [ ] **14. Homebrew formula** — `homebrew-vanic` tap repo. **Gate**: wait until
  macOS is empirically verified on a Darwin host.

- [ ] **15. B.1 Cross-language `.vani` translator CLI** — `tools/vani_translate.py`
  already has `ALIASES`; build a proper round-trip CLI (~4–6 h).

- [ ] **16. C.x SOV completion (mechanical parser side)** — Verb-at-end shapes for
  `let` / `fn` / `if` / `while` / `match` / `struct` / `enum` (~10–15 h). Grammar
  consultant review is separate; this is just the parser work.

---

## Blocked (not in our control)

| Item | Blocker |
|---|---|
| macOS empirical verification | Darwin hardware needed |
| Grammar consultant pass | External native-speaker review |
| Windows IOCP async-TCP (`tcp_echo_epoll` etc.) | Readiness-vs-completion model mismatch (R8 in decisions.md) |
| Arc 7 Win64 / AArch64 CI wiring | CI runner setup |
