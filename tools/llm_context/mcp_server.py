#!/usr/bin/env python3
"""
mcp_server — expose the vāṇी context bundle as an MCP server.

Phase ML-2 (2026-06-07): builds on Phase ML-1's static
`bundle.py`. Where ML-1 prints a single Markdown blob to stdout,
this server exposes each bundle section as an addressable MCP
*resource* + adds *tools* so AI agents can actually compile,
type-check, and run their generated vāṇी source from inside
the conversation.

The unique selling point: an agent writes vāṇी source, calls
`vani_check` to type-check it (which runs the SMT verifier),
and gets compile-time proof of correctness before suggesting
the code to the user. No other language gives an agent SMT-level
feedback this cheaply.

────────────────────────────────────────────────────────────
Resources (read with `read_resource`):

    vani://system-prompt   — orienting system prompt
    vani://aliases         — TokenKind ↔ dialect spelling table
    vani://sov             — SOV verb-at-end shape table
    vani://patterns        — 22 GoF design patterns catalog
    vani://examples        — English example corpus signatures
    vani://errors          — Dialect-aware error prefixes
    vani://limits          — v1 limitations catalog
    vani://full-bundle     — all of the above concatenated

Tools (call with `call_tool`):

    vani_check    — type-check inline .vani source (returns diagnostics)
    vani_run      — compile + run inline source (returns stdout/stderr)
    vani_emit_c   — emit C source for inline .vani source
    list_patterns — list the 22 GoF pattern names + intents
    get_pattern   — fetch the full source of a named GoF pattern

────────────────────────────────────────────────────────────
Setup (Claude Desktop / Cursor / any MCP host):

Add this to your MCP host's config — e.g. for Claude Desktop on
macOS at `~/Library/Application Support/Claude/claude_desktop_config.json`:

    {
      "mcpServers": {
        "vani-context": {
          "command": "python3",
          "args": ["/abs/path/to/vani/tools/llm_context/mcp_server.py"]
        }
      }
    }

The server speaks stdio JSON-RPC; no port to open. Requires
the `mcp` Python SDK (`pip install mcp`).

The `vani_*` tools shell out to the `vanic` binary on `$PATH`.
Build it once with `cargo build --release` from the repo root,
then ensure `target/release/vanic` (or `intentc`) is on `$PATH`,
or set `VANI_BIN=/abs/path/to/vanic` in the host config's `env:`
block.
"""

import asyncio
import importlib.util
import os
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path
from typing import Any

try:
    from mcp.server import Server
    from mcp.server.stdio import stdio_server
    import mcp.types as types
except ImportError:
    sys.stderr.write(
        "mcp SDK not installed. Run `pip install mcp` "
        "(Python 3.10+). See tools/llm_context/README.md.\n"
    )
    sys.exit(2)

# Import bundle.py as a module (no name collision).
HERE = Path(__file__).resolve().parent
BUNDLE_PY = HERE / "bundle.py"
_spec = importlib.util.spec_from_file_location("bundle", BUNDLE_PY)
_bundle = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(_bundle)

REPO_ROOT = HERE.parent.parent
PATTERNS_DIR = REPO_ROOT / "examples" / "language" / "english" / "design_patterns"


def vanic_binary() -> str:
    """Resolve the `vanic` binary path. Prefers $VANI_BIN; falls
    back to `vanic` / `intentc` on $PATH; final fallback is the
    repo's `target/release/` build output."""
    explicit = os.environ.get("VANI_BIN")
    if explicit and Path(explicit).exists():
        return explicit
    for cand in ("vanic", "intentc"):
        found = shutil.which(cand)
        if found:
            return found
    for cand in ("vanic", "intentc"):
        local = REPO_ROOT / "target" / "release" / cand
        if local.exists():
            return str(local)
    raise FileNotFoundError(
        "could not find `vanic`/`intentc` binary — set $VANI_BIN or "
        "run `cargo build --release` from the repo root"
    )


def _section_markdown(name: str) -> str:
    out: list[str] = []
    emitter = _bundle.SECTIONS[name]
    emitter(out)
    return "\n".join(out)


# Resource definitions: (uri suffix, MCP resource name, description, section key)
RESOURCES = [
    ("system-prompt", "vāṇी system prompt",
     "Orienting prompt that frames the model as a vāṇी programmer.",
     "system"),
    ("aliases", "TokenKind ↔ dialect alias table",
     "Maps every TokenKind to its canonical spelling in English / Sanskrit / Hindi / Marathi.",
     "aliases"),
    ("sov", "SOV verb-at-end statement shapes",
     "How Sanskrit-pragma `<expr> लिख;` and friends desugar to English keyword-first form.",
     "sov"),
    ("patterns", "GoF design patterns catalog",
     "22-pattern catalog with one-line intent + path to the worked example for each.",
     "patterns"),
    ("examples", "English example corpus signatures",
     "For each of the 155 English-keyword `.vani` examples, the `intent` line + every `fn` signature.",
     "examples"),
    ("errors", "Dialect-aware error prefixes",
     "Devanagari labels + prefix translations for the highest-frequency error families.",
     "errors"),
    ("limits", "v1 limitations catalog",
     "Documented v1 deviations (no Box, no enum-destructure, no `let mut`, …) with workarounds.",
     "limits"),
    ("full-bundle", "Full context bundle (all sections)",
     "Everything above, concatenated. Use when you have an unbounded context window.",
     None),
]


server = Server("vani-context")


@server.list_resources()
async def list_resources() -> list[types.Resource]:
    return [
        types.Resource(
            uri=f"vani://{suffix}",
            name=name,
            description=desc,
            mimeType="text/markdown",
        )
        for (suffix, name, desc, _) in RESOURCES
    ]


@server.read_resource()
async def read_resource(uri: Any) -> str:
    uri_str = str(uri)
    for (suffix, _name, _desc, section_key) in RESOURCES:
        if uri_str == f"vani://{suffix}":
            if section_key is None:
                # full bundle
                out: list[str] = []
                _bundle.emit_system_prompt(out)
                _bundle.emit_aliases(out, _bundle.load_aliases())
                _bundle.emit_sov_table(out)
                _bundle.emit_patterns(out)
                _bundle.emit_examples(out)
                _bundle.emit_error_prefixes(out)
                _bundle.emit_limitations(out)
                return "\n".join(out)
            return _section_markdown(section_key)
    raise ValueError(f"unknown resource URI: {uri_str}")


@server.list_tools()
async def list_tools() -> list[types.Tool]:
    return [
        types.Tool(
            name="vani_check",
            description=(
                "Type-check vāṇी source (runs lexer + parser + checker + SMT). "
                "Returns the compiler's diagnostic output. Empty output = "
                "clean compile."
            ),
            inputSchema={
                "type": "object",
                "properties": {
                    "source": {
                        "type": "string",
                        "description": "Full .vani source code. Must start with an `intent \"...\";` declaration.",
                    },
                },
                "required": ["source"],
            },
        ),
        types.Tool(
            name="vani_run",
            description=(
                "Compile + run vāṇी source via the LLVM backend. Returns "
                "stdout, stderr, and the exit code as JSON-shaped text."
            ),
            inputSchema={
                "type": "object",
                "properties": {
                    "source": {"type": "string", "description": "Full .vani source."},
                    "backend": {
                        "type": "string",
                        "enum": ["llvm", "c"],
                        "default": "llvm",
                        "description": "Which backend to run through.",
                    },
                },
                "required": ["source"],
            },
        ),
        types.Tool(
            name="vani_emit_c",
            description="Emit the lowered C source for inline vāṇी code. Useful for debugging generated layouts.",
            inputSchema={
                "type": "object",
                "properties": {
                    "source": {"type": "string", "description": "Full .vani source."},
                },
                "required": ["source"],
            },
        ),
        types.Tool(
            name="list_patterns",
            description="List the 22 GoF design pattern examples that ship in the repo.",
            inputSchema={"type": "object", "properties": {}},
        ),
        types.Tool(
            name="get_pattern",
            description="Fetch the full source of a named GoF design pattern example.",
            inputSchema={
                "type": "object",
                "properties": {
                    "name": {
                        "type": "string",
                        "description": "Pattern stem (e.g. `observer`, `singleton`, `composite`). Case-insensitive.",
                    },
                },
                "required": ["name"],
            },
        ),
    ]


def _write_temp_source(source: str) -> Path:
    tmp = Path(tempfile.mkstemp(suffix=".vani", prefix="vani-mcp-")[1])
    tmp.write_text(source)
    return tmp


def _run_vanic(args: list[str], timeout: float = 60.0) -> tuple[int, str, str]:
    """Invoke `vanic` with the given args. Returns (exit_code, stdout, stderr)."""
    try:
        bin_path = vanic_binary()
    except FileNotFoundError as e:
        return (127, "", str(e))
    proc = subprocess.run(
        [bin_path] + args,
        capture_output=True,
        text=True,
        timeout=timeout,
    )
    return (proc.returncode, proc.stdout, proc.stderr)


@server.call_tool()
async def call_tool(name: str, arguments: dict | None) -> list[types.TextContent]:
    args = arguments or {}

    if name == "vani_check":
        source = args.get("source", "")
        tmp = _write_temp_source(source)
        try:
            code, stdout, stderr = _run_vanic(["check", str(tmp)])
        finally:
            tmp.unlink(missing_ok=True)
        body = stderr or stdout or "(clean — no diagnostics)"
        return [types.TextContent(
            type="text",
            text=f"exit={code}\n{body}",
        )]

    if name == "vani_run":
        source = args.get("source", "")
        backend = args.get("backend", "llvm")
        tmp = _write_temp_source(source)
        try:
            cmd = ["run", str(tmp)]
            if backend == "c":
                cmd.append("--backend=c")
            code, stdout, stderr = _run_vanic(cmd, timeout=120.0)
        finally:
            tmp.unlink(missing_ok=True)
        return [types.TextContent(
            type="text",
            text=f"exit={code}\n--- stdout ---\n{stdout}--- stderr ---\n{stderr}",
        )]

    if name == "vani_emit_c":
        source = args.get("source", "")
        tmp = _write_temp_source(source)
        try:
            code, stdout, stderr = _run_vanic(["emit", str(tmp), "--backend=c"])
        finally:
            tmp.unlink(missing_ok=True)
        if code != 0:
            return [types.TextContent(
                type="text",
                text=f"emit failed (exit {code}):\n{stderr or stdout}",
            )]
        return [types.TextContent(type="text", text=stdout)]

    if name == "list_patterns":
        rows: list[str] = ["# 22 GoF design patterns (vāṇी examples)\n"]
        for category in sorted(PATTERNS_DIR.iterdir()):
            if not category.is_dir():
                continue
            rows.append(f"## {category.name.capitalize()}\n")
            for vfile in sorted(category.glob("*.vani")):
                intent = ""
                for line in vfile.read_text().splitlines():
                    if line.strip().startswith("intent "):
                        intent = line.strip()[7:].strip().strip('";')
                        break
                rows.append(f"- **{vfile.stem}** — {intent}")
            rows.append("")
        return [types.TextContent(type="text", text="\n".join(rows))]

    if name == "get_pattern":
        pname = (args.get("name") or "").strip().lower()
        if not pname:
            return [types.TextContent(type="text", text="error: `name` is required")]
        for vfile in PATTERNS_DIR.glob("**/*.vani"):
            if vfile.stem.lower() == pname:
                rel = vfile.relative_to(REPO_ROOT)
                return [types.TextContent(
                    type="text",
                    text=f"# {rel}\n\n```vani\n{vfile.read_text()}```",
                )]
        return [types.TextContent(
            type="text",
            text=f"no pattern named `{pname}` — call list_patterns to see options",
        )]

    return [types.TextContent(type="text", text=f"unknown tool: {name}")]


async def main() -> None:
    async with stdio_server() as (read_stream, write_stream):
        await server.run(
            read_stream,
            write_stream,
            server.create_initialization_options(),
        )


if __name__ == "__main__":
    asyncio.run(main())
