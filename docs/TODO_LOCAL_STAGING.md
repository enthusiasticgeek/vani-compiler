# vani-compiler -- Local Fuzz Staging Log (auto-generated, NOT authoritative)

Candidate findings from the unattended local-model fuzz harness
(`tools/localfuzz/`), running on the `local-fuzz-findings` branch only.
Entries here are drafted by a small local model that has NOT read the
compiler source and has NOT verified root cause -- treat every entry as
an unverified lead, not a confirmed bug.

Promote a real finding into `docs/TODO_CURRENT.md` (on `main`, with a
proper BUG-N writeup) only after a human or frontier-model session has:
1. Reproduced it independently.
2. Root-caused it in the actual source.
3. Confirmed it isn't already-known/expected behavior.

---
