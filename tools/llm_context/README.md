# LLM context bundle (Phase ML-1)

`bundle.py` assembles a self-contained Markdown context bundle
that orients an off-the-shelf LLM (Claude, GPT-4-class,
Llama-3-class) as a vāṇी programmer. The bundle is regenerated
from repo sources of truth on every run — no stale copy to keep
in sync.

This is the cheapest layer on the ML roadmap (see
[TODO.md §*"ML model — value assessment"*](../../TODO.md)):
**~80% of the way to useful code generation, zero training**.

## Usage

```bash
# Pipe to clipboard (paste into Claude / ChatGPT as a system
# message or the first user message)
python3 tools/llm_context/bundle.py | pbcopy           # macOS
python3 tools/llm_context/bundle.py | xclip -sel clip  # X11

# Save to disk for re-use
python3 tools/llm_context/bundle.py > /tmp/vani_ctx.md

# Trim heavy sections when the model has a tight context budget
python3 tools/llm_context/bundle.py --no-examples    # cuts ~30K bytes
python3 tools/llm_context/bundle.py --no-limits      # cuts ~10K bytes

# Emit a single section
python3 tools/llm_context/bundle.py --section aliases   # keyword table only
python3 tools/llm_context/bundle.py --section patterns  # GoF catalog only
```

## What's in the bundle

| # | Section | Source of truth |
|---|---|---|
| 1 | System prompt orienting the model | `bundle.py::emit_system_prompt` |
| 2 | TokenKind ↔ {english, sanskrit, hindi, marathi} alias table | `tools/vani_translate.py::ALIASES` |
| 3 | SOV verb-at-end statement shape table | README + `bundle.py::emit_sov_table` |
| 4 | 22 GoF design patterns, one-line intent each | `examples/language/english/design_patterns/**/*.vani` |
| 5 | English example corpus signatures (`intent` + `fn`) | `examples/language/english/*.vani` (155 files) |
| 6 | Dialect-aware error prefixes | `src/diagnostic.rs::localize_message` |
| 7 | v1 limitations catalog | `docs/v1_limitations.md` (verbatim) |

Approximate token cost (using Claude tokenization):

| Bundle | Bytes | Tokens (est.) |
|---|---|---|
| Full (all sections) | ~54K | ~13K |
| `--no-examples` | ~27K | ~7K |
| `--no-examples --no-limits` | ~17K | ~4K |
| Single `--section aliases` | ~5K | ~1.2K |

## Sample workflow

1. **Generate the bundle**:
   ```bash
   python3 tools/llm_context/bundle.py > /tmp/vani_ctx.md
   ```

2. **Prime the model**. Open Claude / ChatGPT / your local LLM
   client. Paste the bundle as the first message. Add your task:

   > "Here's the vāṇी context. Now: write a `fn factorial(n: i64) -> i64`
   > with `requires n >= 0;` and `ensures result >= 1;` clauses,
   > using English keywords."

3. **Run the model's output through the compiler**:
   ```bash
   echo '…model output…' > /tmp/out.vani
   cargo run --release -- check /tmp/out.vani
   cargo run --release -- run   /tmp/out.vani
   ```

4. **Iterate**. The compiler's diagnostics + the bundle's §7
   limitations catalog give the model the feedback it needs to
   fix v1-incompatible suggestions.

## Why a static bundle (Phase ML-1) before an MCP server (Phase ML-2)?

- The bundle ships **today** — no protocol work, no agent
  integration, no hosted service.
- The MCP path (ML-2) is the natural follow-on: same bundle
  content, but the agent pulls it on demand instead of carrying
  it in every prompt's context.
- Both layers gate on the same data prep — getting the bundle
  shape right here de-risks the MCP work later.

## Maintaining the bundle

The bundle has **no** hand-curated content. Every section is
sourced from a file already maintained for another reason:

- Add a new keyword → update `tools/vani_translate.py::ALIASES`
  → bundle picks it up automatically.
- Add a new design pattern → drop a `.vani` file with an
  `intent "…"` line under `design_patterns/<category>/` → bundle
  picks it up automatically.
- Add a new v1 limitation → append to `docs/v1_limitations.md` →
  bundle picks it up automatically.

If the bundle ever drifts from reality, the fix is to edit the
underlying source-of-truth file, not the bundle script.
