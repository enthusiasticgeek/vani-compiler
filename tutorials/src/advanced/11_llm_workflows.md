# Advanced 11 -- Using vāṇी with an LLM (Claude / GPT / MCP)

> **Learning goal**: drive an off-the-shelf large language model
> (Claude, GPT-4-class, Llama-3-class, local) to generate
> useful vāṇी programs -- either by pasting a static context
> bundle into a chat, or by wiring the dedicated MCP server so
> an agent can write, type-check, and run vāṇी source on its
> own without leaving the conversation.

vāṇी treats AI-assisted code generation as a first-class
workflow. The compiler's SMT verifier + deterministic
diagnostics make it unusually suited as a target for LLM
output -- when the model gets something wrong, the compiler
tells the model *exactly* what's wrong in source terms, and
the model can iterate. This chapter walks the two shipped
tools and shows the write-verify-iterate loop end to end.

## What ships

Two scripts under [`tools/llm_context/`](https://github.com/enthusiasticgeek/vani-compiler/blob/main/tools/llm_context/):

- **`bundle.py`** (Phase ML-1, 2026-06-07) -- a static Markdown
  context bundle, ~13K tokens full, ~7K with `--no-examples`.
  Paste it into any LLM as a system prompt and the model can
  generate working vāṇी without any training.
- **`mcp_server.py`** (Phase ML-2, 2026-06-07) -- exposes the
  same content as an [MCP](https://modelcontextprotocol.io/)
  server so an agent pulls just the section it needs AND can
  call `vanic check` / `vanic run` / `vanic emit-c` on its own
  generated source -- closing the write-verify loop.

Both are deliberately **plain Python**, no model training, no
hosted endpoint, no compiler-binary dependency beyond the
existing `vanic` CLI.

## Workflow 1 -- paste-the-bundle (no MCP needed)

The simplest setup: works with Claude.ai, ChatGPT, Gemini,
local Llama-3, or any chat client that accepts a long opening
message.

### Generate the bundle

```bash
# Pipe to clipboard (paste as the system message or first turn)
python3 tools/llm_context/bundle.py | pbcopy             # macOS
python3 tools/llm_context/bundle.py | xclip -sel clip    # X11

# Or save to disk for re-use
python3 tools/llm_context/bundle.py > /tmp/vani_ctx.md

# Tight-context variants:
python3 tools/llm_context/bundle.py --no-examples        # ~7K tokens
python3 tools/llm_context/bundle.py --no-examples --no-limits  # ~4K tokens

# Single section only:
python3 tools/llm_context/bundle.py --section aliases    # keyword table only
python3 tools/llm_context/bundle.py --section patterns   # GoF catalog only
```

### What's inside

| # | Section | Source of truth |
|---|---|---|
| 1 | System prompt orienting the model | `bundle.py::emit_system_prompt` |
| 2 | Keyword alias table (English <-> Sanskrit <-> Hindi <-> Marathi) | `tools/vani_translate.py::ALIASES` |
| 3 | SOV verb-at-end statement shape table | README + bundle |
| 4 | 22 GoF design patterns, one-line intent each | `examples/.../design_patterns/` |
| 5 | English example corpus signatures (`intent` + `fn`) | 155 example files |
| 6 | Dialect-aware error prefixes | `src/diagnostic.rs::localize_message` |
| 7 | v1 limitations catalog (verbatim) | `docs/v1_limitations.md` |

The bundle has **no hand-curated content** -- every section is
generated from existing repo files. Add a new keyword to the
translator, the bundle picks it up. Add a new limitation to
the catalog, the bundle picks it up. The bundle script never
drifts from reality.

### Prime the model + ask for code

Paste the bundle as the first message. Then ask:

> "Here's the vāṇी context. Now: write a `fn factorial(n: i64)
> -> i64` with `requires n >= 0;` and `ensures result >= 1;`
> clauses, using English keywords."

A capable model produces something like:

```vani
fn factorial(n: i64) -> i64
requires n >= 0;
ensures result >= 1;
{
  if n == 0 { return 1; }
  return n * factorial(n - 1);
}
```

### Run the model's output through the compiler

```bash
echo '...model output...' > /tmp/out.vani
vanic check /tmp/out.vani
vanic run   /tmp/out.vani
```

### Iterate

Compiler diagnostics + the bundle's Sec.7 limitations catalog
together give the model exactly the feedback it needs:

```
/tmp/out.vani:5:14: error: value 'n' was moved; cannot use after move
  return n * factorial(n - 1);
             ^^^^^^^^^^^^^^^^
  help: 1. After `let other = n`, `n` is no longer usable ...
  help: 2. vāṇी uses affine ownership: each heap-owning value ...
  help: 3. Either (a) borrow instead of moving ...
```

Paste the diagnostic back into the chat. The model's next
attempt usually fixes the issue -- the help-line elaborations
(see [Intermediate 10b runtime errors](../intermediate/10b_runtime_errors_primer.md))
were explicitly designed to give LLMs and human readers the
same actionable feedback shape.

## Workflow 2 -- MCP server (agentic write-verify loop)

When your client speaks [MCP](https://modelcontextprotocol.io/)
(Claude Desktop, Claude Code CLI, Cursor, plus several other
agent hosts), the dedicated server is a tighter loop: the
agent pulls just the bundle section it needs AND can run the
compiler on its own generated source -- no manual paste-back.

### Setup

```bash
# Install the Python MCP SDK
pip install mcp

# Build a release vanic binary
cd /path/to/vani && cargo build --release
export VANI_BIN=$(realpath target/release/vanic)
```

### Wire into Claude Desktop

Add to `~/Library/Application Support/Claude/claude_desktop_config.json`
(macOS) or `%APPDATA%\Claude\claude_desktop_config.json` (Windows):

```json
{
  "mcpServers": {
    "vani-context": {
      "command": "python3",
      "args": ["/abs/path/to/vani/tools/llm_context/mcp_server.py"],
      "env": {
        "VANI_BIN": "/abs/path/to/vani/target/release/vanic"
      }
    }
  }
}
```

Restart Claude Desktop. The new server's resources and tools
appear in the agent's available-capabilities list.

Cursor, Claude Code, and most other MCP hosts read the same
JSON shape -- point `command` at `python3` and `args[0]` at
`mcp_server.py`.

### Resources (pulled on demand, save context window)

| URI | Content |
|---|---|
| `vani://system-prompt` | Orienting system prompt |
| `vani://aliases` | TokenKind <-> dialect spelling table |
| `vani://sov` | SOV verb-at-end shape table |
| `vani://patterns` | 22-pattern GoF catalog |
| `vani://examples` | Signatures of all 155 English examples |
| `vani://errors` | Dialect-aware error prefix table |
| `vani://limits` | v1 limitations catalog |
| `vani://full-bundle` | All of the above concatenated |

Instead of pasting all 13K tokens upfront, the agent pulls
`vani://aliases` when (and only when) writing a Devanagari
file. Big savings on long-running agent sessions.

### Tools (the agent calls these directly)

| Name | What it does |
|---|---|
| `vani_check` | Type-check inline `.vani` source (lexer + parser + checker + SMT). Returns diagnostics. |
| `vani_run` | Compile + run inline source via LLVM or C backend. Returns stdout / stderr / exit code. |
| `vani_emit_c` | Emit the lowered C source for debugging codegen layouts. |
| `list_patterns` | Enumerate the 22 GoF design pattern examples. |
| `get_pattern` | Fetch the full source of a named pattern (`observer`, `visitor`, etc.). |

### The write-verify loop end-to-end

A sample agent turn -- what happens inside one user message
when you ask "write me a sorted vec wrapper with a binary
search":

1. Agent fetches `vani://aliases` + `vani://examples` (~1.5K
   tokens combined, way under the 13K full bundle).
2. Agent drafts a `SortedVec<i64>` wrapper with `push`,
   `find`, and `requires` / `ensures` clauses.
3. Agent calls `vani_check` on its draft. The tool reports
   one diagnostic -- say, an `ensures` clause SMT couldn't
   discharge.
4. Agent reads the diagnostic + the `help:` elaboration,
   adjusts the `requires` clause to give the solver the
   missing fact.
5. Agent calls `vani_check` again -> clean.
6. Agent calls `vani_run` with a smoke-test main -> expected
   stdout matches.
7. Agent presents the verified source to you.

Steps 3-6 happened inside the agent's own context, NOT a
manual paste-back. The compiler is in-the-loop; the agent
self-corrects against SMT proof obligations. This is what
makes vāṇी an unusually good target for AI-assisted code:
the agent has a deterministic oracle it can interrogate.

### What's *not* in the MCP server

- **No HTTP transport.** stdio JSON-RPC only. Use an MCP
  bridge (e.g. `mcp-bridge`) if your host needs HTTP.
- **No streaming output.** Compile/run results return when
  the process exits. Long-running programs time out (60s for
  `vani_check`, 120s for `vani_run`).
- **No persistent state.** Each tool invocation runs the
  compiler from a fresh temp file -- no cached incremental
  build across calls. Tradeoff: deterministic results, slower
  feedback than an in-process compiler would give.

## Which workflow when

| Scenario | Use |
|---|---|
| Quick "generate me a vāṇी snippet" in a chat client | **Bundle paste** (workflow 1) |
| Agentic loop where you want the agent to verify before suggesting | **MCP server** (workflow 2) |
| Local model (Llama-3, Qwen) without MCP support | **Bundle paste** (workflow 1) |
| You're using Claude Desktop / Code / Cursor anyway | **MCP server** (workflow 2) |
| One-off + no agent infrastructure | **Bundle paste** (workflow 1) |

The bundle covers the 80% case. The MCP server adds the
self-verifying tool layer on top -- same content, more
plumbing, tighter loop.

## What's queued (not shipped today)

The roadmap (see [TODO.md `🤖 ML model that learns vāṇी`](https://github.com/enthusiasticgeek/vani-compiler/blob/main/TODO.md))
has two further phases that haven't started:

- **Phase ML-3** (~20-30h focused + ~$100-300 GPU credits):
  LoRA fine-tune a small open-weights model (Llama-3 8B or
  Qwen-2.5 7B) on the English <-> Sanskrit <-> Hindi <-> Marathi
  translation corpus. Ship as a downloadable artifact for
  users who can't reach hosted APIs.
- **Phase ML-4** (deferred / external): hosted inference
  service. Out of scope for the compiler-side roadmap; lands
  only if someone steps up to host it.

Both phases gate on demand. The bundle + MCP cover most
use cases today; ML-3 is for users who want offline / no-API-
key operation. **Custom-trained-from-scratch transformer is
explicitly NOT planned** -- wouldn't beat fine-tuning for the
cost.

## A summary you can carry

- **`tools/llm_context/bundle.py`** -- Markdown context bundle
  to stdout. Paste into Claude / ChatGPT / local LLM and the
  model generates passable vāṇी with zero training. Trim
  flags (`--no-examples`, `--no-limits`, `--section`) for
  tight context budgets.
- **`tools/llm_context/mcp_server.py`** -- MCP server exposing
  the same bundle as 8 addressable resources + 5 tools. Agent
  pulls just what it needs AND can run the compiler on its
  own output. Works with Claude Desktop / Code / Cursor and
  any MCP-speaking host.
- The compiler's **SMT-discharged diagnostics + step-by-step
  `help:` elaborations** were designed to be readable by both
  humans AND LLMs -- the same shape of feedback that helps a
  newcomer fix a move-after-use also helps an LLM iterate
  toward a verified solution.
- **ML-3 (LoRA fine-tune)** and **ML-4 (hosted inference)**
  are queued, not shipped. The bundle + MCP cover most use
  cases today.

The takeaway: **AI-assisted vāṇी is a write-verify-iterate
loop, not a one-shot prompt.** The compiler is in the loop;
the model self-corrects against SMT obligations; the result is
unusually high-quality generated code for a language this
young.

## Cross-reference

- [`tools/llm_context/README.md`](https://github.com/enthusiasticgeek/vani-compiler/blob/main/tools/llm_context/README.md)
  -- the canonical setup doc; mirror of this chapter for
  maintainers
- [Intermediate 10b -- Runtime errors + panic-free design](../intermediate/10b_runtime_errors_primer.md)
  -- the same WHAT/WHY/HOW diagnostic shape that LLMs read
- [Intermediate 12b -- Compile time vs runtime](../intermediate/12b_compile_time_vs_runtime_primer.md)
  -- the SMT proof obligations the agent uses for write-verify
- [Advanced 10 -- Compiler internals tour](10_internals.md)
  -- what's inside `vani_check` / `vani_run` / `vani_emit_c`
  when the agent calls them
- [Intermediate 12a -- SMT primer](../intermediate/12a_smt_primer.md)
  -- the compile-time prove-it-correct layer that gives the
  agent its oracle


---

**Previous**: [Sec.10 -- Compiler internals tour ->](10_internals.md)
**Next**: [Sec.12 -- Safety-critical standards ->](12_safety_standards.md)
