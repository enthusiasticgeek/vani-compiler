# LLM context bundle (Phase ML-1 + ML-2)

Two scripts live here:

- **`bundle.py`** (Phase ML-1) — assembles a Markdown context
  bundle to stdout for one-shot pasting into Claude / GPT / a
  local LLM. **Static**; the model carries the whole bundle in
  its prompt.
- **`mcp_server.py`** (Phase ML-2) — exposes the same content as
  an [MCP](https://modelcontextprotocol.io/) server so an agent
  can pull just the section it needs **and** call
  `vanic check` / `vanic run` / `vanic emit-c` on its own
  generated source — closing the write-verify loop end-to-end.

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
# (byte/token figures confirmed by testing 2026-08-01; the corpus
# has grown a lot since this doc's original ~54K/~13K estimate)
python3 tools/llm_context/bundle.py --no-examples    # cuts ~43K bytes
python3 tools/llm_context/bundle.py --no-limits      # cuts ~64K bytes

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
| 5 | English example corpus signatures (`intent` + `fn`) | `examples/language/english/*.vani` (165 files) |
| 6 | Dialect-aware error prefixes | `src/diagnostic.rs::localize_message` |
| 7 | v1 limitations catalog | `docs/v1_limitations.md` (verbatim) |

Approximate token cost (chars/4 estimate; confirmed by testing
2026-08-01 -- the corpus, especially `docs/v1_limitations.md`,
has grown substantially since this table's original ~54K/~13K
figures):

| Bundle | Bytes | Tokens (est.) |
|---|---|---|
| Full (all sections) | ~126K | ~31K |
| `--no-examples` | ~83K | ~21K |
| `--no-examples --no-limits` | ~19K | ~5K |
| Single `--section aliases` | ~4.4K | ~1.1K |

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

## `mcp_server.py` — MCP server (Phase ML-2)

Exposes the same bundle content as an MCP server. Each section
of `bundle.py` becomes an addressable resource; agents pull only
what they need. The server also adds **tools** that let the
agent shell out to `vanic check` / `vanic run` / `vanic emit-c`
on its own generated source — so a code-gen agent can verify
each iteration against the SMT verifier and runtime *before*
suggesting code to the user.

### Resources

| URI | What's behind it |
|---|---|
| `vani://system-prompt` | Orienting system prompt |
| `vani://aliases`       | TokenKind ↔ dialect spelling table |
| `vani://sov`           | SOV verb-at-end shape table |
| `vani://patterns`      | 22-pattern GoF catalog |
| `vani://examples`      | Signatures of all 165 English examples |
| `vani://errors`        | Dialect-aware error prefix table |
| `vani://limits`        | v1 limitations catalog |
| `vani://full-bundle`   | All of the above concatenated |

### Tools

| Name | What it does |
|---|---|
| `vani_check`    | Type-check inline `.vani` source (runs lexer + parser + checker + SMT). Returns the compiler's diagnostics. |
| `vani_run`      | Compile + run inline source via LLVM or C backend. Returns stdout, stderr, exit code. |
| `vani_emit_c`   | Emit the lowered C source for debugging codegen layouts. |
| `list_patterns` | Enumerate the 22 GoF design pattern examples. |
| `get_pattern`   | Fetch the full source of a named pattern (e.g. `observer`). |

### Setup

Requires the official Python MCP SDK:

```bash
pip install mcp
```

…and the `vanic` binary somewhere reachable:

```bash
cd /path/to/vani && cargo build --release
# either add target/release/ to $PATH, or point the env var at it:
export VANI_BIN=/path/to/vani/target/release/vanic
```

#### Claude Desktop

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

#### Cursor / any MCP host

Same shape — most MCP hosts read a JSON config block keyed by
server name. Point `command` at `python3` and `args[0]` at
`mcp_server.py`.

### Why MCP after the static bundle?

- **Resources** save the agent's context window. Instead of
  pasting all 13K tokens up front, the agent pulls just the
  `vani://aliases` table when it needs to write a Devanagari
  file.
- **Tools** close the write-verify loop. Without them, an
  agent that writes vāṇी source has to ask the user to run the
  compiler and paste back diagnostics. With them, the agent
  iterates inside the same conversation: write → check → fix →
  check → done. SMT-backed compile-time proofs at every step
  are vāṇी's unique selling point for AI-assisted code.

### What's *not* in the MCP server

- **No HTTP transport.** stdio JSON-RPC only. Use an MCP
  bridge (e.g. `mcp-bridge`) if you need HTTP.
- **No streaming output.** Compile/run results return when the
  process exits. Long-running programs eventually time out (60s
  for `vani_check`, 120s for `vani_run`).
- **No persistent state.** Each tool invocation runs the
  compiler from a fresh temp file. There's no incremental
  build cache reused across calls.

## Why a static bundle (Phase ML-1) AND an MCP server (Phase ML-2)?

- The **static bundle** ships **today** — no protocol work, no
  agent integration, no hosted service. Paste it into any
  conversation that accepts a system prompt.
- The **MCP server** is for hosts that speak MCP. Same bundle
  content, but the agent pulls it on demand instead of carrying
  it in every prompt's context — and gains compile/verify tools
  that close the iteration loop.
- Both layers gate on the same data prep — getting the bundle
  shape right in `bundle.py` was the hard part; `mcp_server.py`
  just wraps it.

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
