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

---

### Candidate: 20260802-180614-run-crash-1a1740134e

Repro: `tools/localfuzz/findings/20260802-180614-run-crash-1a1740134e/repro.vani`

(ollama unavailable -- raw finding only)

```json
{
  "kind": "run-crash",
  "c": {
    "rc": null,
    "stdout": "",
    "stderr": "",
    "timed_out": true
  },
  "llvm": {
    "rc": null,
    "stdout": "",
    "stderr": "",
    "timed_out": true
  }
}
```


---

### Candidate: 20260802-194717-backend-divergence-c336be7192

Repro: `tools/localfuzz/findings/20260802-194717-backend-divergence-c336be7192/repro.vani`

(ollama unavailable -- raw finding only)

```json
{
  "kind": "backend-divergence",
  "c": {
    "rc": 0,
    "stdout": "0\n10\n20\n81\n40\n",
    "stderr": "",
    "timed_out": false
  },
  "llvm": {
    "rc": 0,
    "stdout": "0\n10\n20\n80\n40\n",
    "stderr": "",
    "timed_out": false
  }
}
```

