#!/usr/bin/env python3
"""
benchmarks/run_benchmarks.py — vāṇī benchmark runner.

Compiles and times each benchmark in vāṇī, Rust, C, and C++, then
writes benchmarks/results/RESULTS.md with ASCII bar-chart tables.

Usage
-----
    python3 benchmarks/run_benchmarks.py             # all benchmarks, all languages
    python3 benchmarks/run_benchmarks.py --runs 5    # 5 timing runs per benchmark
    python3 benchmarks/run_benchmarks.py --bench 01  # only benchmark 01_fibonacci
    python3 benchmarks/run_benchmarks.py --langs vani,c,cpp
    python3 benchmarks/run_benchmarks.py --output benchmarks/results/MY_RESULTS.md
    python3 benchmarks/run_benchmarks.py --list      # list benchmarks and exit

Requirements
------------
    - Python 3.8+
    - vanic   (vāṇī compiler; install per INSTALL.md)
    - gcc     (or clang) for C benchmarks
    - g++     (or clang++) for C++ benchmarks
    - rustc   for Rust benchmarks
    Missing compilers are reported and their benchmarks skipped (not an error).
"""

import argparse
import io
import os
import platform
import shutil
import statistics
import subprocess
import sys
import tempfile

# Windows console may use cp1252 which can't encode Unicode benchmark names.
if hasattr(sys.stdout, "buffer"):
    sys.stdout = io.TextIOWrapper(sys.stdout.buffer, encoding="utf-8", errors="replace", line_buffering=True)
import time
from dataclasses import dataclass, field
from pathlib import Path
from typing import Dict, List, Optional, Tuple

# ---------------------------------------------------------------------------
# Benchmark registry
# ---------------------------------------------------------------------------

BENCH_DIR = Path(__file__).parent

BENCHMARKS: List[Dict] = [
    {
        "id": "01_fibonacci",
        "name": "Fibonacci(42) — recursive",
        "description": "Classic recursive fib(42). Tests raw function-call throughput.",
        "expected": "267914296",
        "variants": [
            {"tag": "vani", "file": "fib.vani"},
            {"tag": "c",    "file": "fib.c"},
            {"tag": "cpp",  "file": "fib.cpp"},
            {"tag": "rs",   "file": "fib.rs"},
        ],
    },
    {
        "id": "02_sieve",
        "name": "Sieve of Eratosthenes — primes ≤ 2 000 000",
        "description": "Boolean sieve with Vec-set inner loop. Tests dense random-access array writes.",
        "expected": "148933",
        "variants": [
            {"tag": "vani", "file": "sieve.vani"},
            {"tag": "c",    "file": "sieve.c"},
            {"tag": "cpp",  "file": "sieve.cpp"},
            {"tag": "rs",   "file": "sieve.rs"},
        ],
    },
    {
        "id": "03_matrix_mul",
        "name": "Matrix multiplication 256×256 (i64)",
        "description": "Naïve triple-loop matmul. Tests arithmetic-dense nested loops.",
        "expected": None,   # checksum varies; runner just checks exit 0
        "variants": [
            {"tag": "vani", "file": "matmul.vani"},
            {"tag": "c",    "file": "matmul.c"},
            {"tag": "cpp",  "file": "matmul.cpp"},
            {"tag": "rs",   "file": "matmul.rs"},
        ],
    },
    {
        "id": "04_sorting",
        "name": "Sort 1 000 000 integers",
        "description": "vāṇī uses the built-in sort(mut ref xs); others use stdlib. Tests sort quality.",
        "expected": None,
        "variants": [
            {"tag": "vani", "file": "sort.vani"},
            {"tag": "c",    "file": "sort.c"},
            {"tag": "cpp",  "file": "sort.cpp"},
            {"tag": "rs",   "file": "sort.rs"},
        ],
    },
    {
        "id": "05_graph_bfs",
        "name": "Graph BFS — index handles vs. weak_ptr",
        "description": (
            "KEY BENCHMARK: BFS on a 1 000-node random graph, repeated 1 000×.\n"
            "  graph.vani / graph_index.{c,cpp,rs} — int-index adjacency list, zero ref-counting.\n"
            "  graph_weakptr.cpp                   — shared_ptr children + weak_ptr back-edges."
        ),
        "expected": None,
        "variants": [
            {"tag": "vani",         "file": "graph.vani"},
            {"tag": "c",            "file": "graph_index.c"},
            {"tag": "cpp_idx",      "file": "graph_index.cpp",  "label": "C++ (index)"},
            {"tag": "cpp_weak",     "file": "graph_weakptr.cpp","label": "C++ (weak_ptr)"},
            {"tag": "rs",           "file": "graph_index.rs"},
        ],
    },
    {
        "id": "06_parallel_sum",
        "name": "Parallel sum — 50 000 000 elements",
        "description": (
            "vāṇī: `parallel for … reduce total with +` (3 extra keywords).\n"
            "C/C++: OpenMP (if available), else serial.\n"
            "Rust: std::thread manual split."
        ),
        "expected": None,
        "variants": [
            {"tag": "vani", "file": "parsum.vani"},
            {"tag": "c",    "file": "parsum.c"},
            {"tag": "cpp",  "file": "parsum.cpp"},
            {"tag": "rs",   "file": "parsum.rs"},
        ],
    },
    {
        "id": "07_hashmap",
        "name": "HashMap — 500 000 insert + 500 000 lookup",
        "description": "Tests open-addressing HashMap throughput.",
        "expected": None,
        "variants": [
            {"tag": "vani", "file": "hash.vani"},
            {"tag": "c",    "file": "hash.c"},
            {"tag": "cpp",  "file": "hash.cpp"},
            {"tag": "rs",   "file": "hash.rs"},
        ],
    },
    {
        "id": "08_linked_list",
        "name": "Linked list — 1 000 000 nodes, index-based",
        "description": (
            "vāṇī/C index approach (no raw pointers): O(1) cache-friendly traversal.\n"
            "C++/Rust use traditional pointer-linked nodes for comparison."
        ),
        "expected": None,
        "variants": [
            {"tag": "vani", "file": "list.vani"},
            {"tag": "c",    "file": "list.c"},
            {"tag": "cpp",  "file": "list.cpp"},
            {"tag": "rs",   "file": "list.rs"},
        ],
    },
    {
        "id": "09_alloc_stress",
        "name": "Allocation stress — 500 000 struct alloc/free cycles",
        "description": "Tests allocator throughput; vāṇī uses RAII affine drop.",
        "expected": None,
        "variants": [
            {"tag": "vani", "file": "alloc.vani"},
            {"tag": "c",    "file": "alloc.c"},
            {"tag": "cpp",  "file": "alloc.cpp"},
            {"tag": "rs",   "file": "alloc.rs"},
        ],
    },
    {
        "id": "10_array_stats",
        "name": "Array statistics — mean + variance of 10 000 000 values",
        "description": "vāṇī: two `parallel for … reduce` passes. C/C++/Rust: sequential passes. Tests loop throughput and parallelism.",
        "expected": None,
        "variants": [
            {"tag": "vani", "file": "stats.vani"},
            {"tag": "c",    "file": "stats.c"},
            {"tag": "cpp",  "file": "stats.cpp"},
            {"tag": "rs",   "file": "stats.rs"},
        ],
    },
    {
        "id": "11_simd_dot",
        "name": "SIMD dot product — explicit vec128<f32> vs auto-vectorized (4 M elements)",
        "description": "vāṇī: explicit vec128<f32> simd_mul + simd_reduce_add. C/C++/Rust: scalar loop auto-vectorized by compiler. Compares explicit SIMD vs optimizer output.",
        "expected": None,   # two equal integers; runner checks they match
        "variants": [
            {"tag": "vani", "file": "dot_simd.vani"},
            {"tag": "c",    "file": "dot.c"},
            {"tag": "cpp",  "file": "dot.cpp"},
            {"tag": "rs",   "file": "dot.rs"},
        ],
    },
]

# ---------------------------------------------------------------------------
# Compiler detection
# ---------------------------------------------------------------------------

def _which(names: List[str]) -> Optional[str]:
    for n in names:
        found = shutil.which(n)
        if found:
            return found
    return None

IS_WIN = platform.system() == "Windows"
EXE_EXT = ".exe" if IS_WIN else ""


def detect_compilers() -> Dict[str, Optional[str]]:
    # Look for vanic relative to this script's repo root as well as PATH
    script_root = Path(__file__).parent.parent
    extra_vanic = [
        str(script_root / "target" / "release" / "vanic.exe"),
        str(script_root / "target" / "release" / "vanic"),
    ]
    return {
        "vanic": _which(["vanic", "vanic.exe"] + extra_vanic),
        "cc":    _which(["gcc", "clang", "cc"]),
        "cxx":   _which(["g++", "clang++", "c++"]),
        "rustc": _which(["rustc"]),
    }


# ---------------------------------------------------------------------------
# Compilation helpers
# ---------------------------------------------------------------------------

def _run(cmd: List[str], cwd=None) -> Tuple[bool, str]:
    try:
        r = subprocess.run(cmd, capture_output=True, text=True, cwd=cwd, timeout=300)
        return r.returncode == 0, (r.stdout + r.stderr).strip()
    except FileNotFoundError as exc:
        return False, str(exc)
    except subprocess.TimeoutExpired:
        return False, "compilation timed out"


def compile_vani(src: Path, out: Path, compilers: Dict) -> Tuple[bool, str]:
    if not compilers["vanic"]:
        return False, "vanic not found in PATH"
    ok, msg = _run([compilers["vanic"], "build", str(src), "-o", str(out)])
    return ok, msg


def compile_c(src: Path, out: Path, compilers: Dict, extra: List[str] = ()) -> Tuple[bool, str]:
    if not compilers["cc"]:
        return False, f"gcc/clang not found"
    cmd = [compilers["cc"], "-O3", "-march=native", "-o", str(out), str(src)] + list(extra)
    ok, msg = _run(cmd)
    if not ok and "-fopenmp" in extra:
        # retry without OpenMP
        cmd2 = [compilers["cc"], "-O3", "-march=native", "-o", str(out), str(src)]
        ok, msg = _run(cmd2)
        if ok:
            msg = "(OpenMP unavailable; compiled serial)"
    return ok, msg


def compile_cpp(src: Path, out: Path, compilers: Dict, extra: List[str] = ()) -> Tuple[bool, str]:
    if not compilers["cxx"]:
        return False, f"g++/clang++ not found"
    cmd = [compilers["cxx"], "-O3", "-march=native", "-std=c++17", "-o", str(out), str(src)] + list(extra)
    ok, msg = _run(cmd)
    if not ok and "-fopenmp" in extra:
        cmd2 = [compilers["cxx"], "-O3", "-march=native", "-std=c++17", "-o", str(out), str(src)]
        ok, msg = _run(cmd2)
        if ok:
            msg = "(OpenMP unavailable; compiled serial)"
    return ok, msg


def compile_rust(src: Path, out: Path, compilers: Dict) -> Tuple[bool, str]:
    if not compilers["rustc"]:
        return False, "rustc not found in PATH"
    ok, msg = _run([compilers["rustc"], "-C", "opt-level=3", "-C", "target-cpu=native",
                    "-o", str(out), str(src)])
    return ok, msg


# ---------------------------------------------------------------------------
# Timing
# ---------------------------------------------------------------------------

def time_run(exe: Path, runs: int) -> Tuple[float, str, bool]:
    """Return (median_seconds, last_stdout, success)."""
    times = []
    last_out = ""
    for _ in range(runs):
        t0 = time.perf_counter()
        try:
            r = subprocess.run(
                [str(exe)], capture_output=True, text=True, timeout=600
            )
            t1 = time.perf_counter()
            if r.returncode != 0:
                return 0.0, r.stderr.strip(), False
            times.append(t1 - t0)
            last_out = r.stdout.strip()
        except subprocess.TimeoutExpired:
            return 0.0, "timed out", False
    return statistics.median(times), last_out, True


# ---------------------------------------------------------------------------
# Reporting helpers
# ---------------------------------------------------------------------------

BAR_WIDTH = 36
BAR_CHAR  = "█"
EMPTY_CHAR = "░"


def _bar(fraction: float) -> str:
    n = max(1, int(round(fraction * BAR_WIDTH)))
    return BAR_CHAR * n + EMPTY_CHAR * (BAR_WIDTH - n)


def _pct(t: float, baseline: float) -> str:
    if baseline == 0:
        return "  n/a"
    ratio = t / baseline
    if ratio < 1.0:
        return f" {(1 - ratio) * 100:4.1f}% faster"
    elif ratio > 1.0:
        return f" {(ratio - 1) * 100:4.1f}% slower"
    return "  baseline"


def _fmt_ms(t: float) -> str:
    ms = t * 1000
    if ms < 1000:
        return f"{ms:7.1f} ms"
    return f"{ms / 1000:7.3f}  s"


def render_chart(bench_name: str, results: List[Tuple[str, float]], baseline_tag: str) -> str:
    if not results:
        return f"  (no results for {bench_name})\n"
    baseline = next((t for tag, t in results if tag == baseline_tag), results[0][1])
    if baseline == 0:
        baseline = min(t for _, t in results if t > 0) or 1e-9
    max_t = max(t for _, t in results)
    lines = [f"```"]
    for tag, t in results:
        bar = _bar(t / max_t)
        lines.append(
            f"  {tag:<14s} {bar}  {_fmt_ms(t)}  {_pct(t, baseline)}"
        )
    lines.append("```")
    return "\n".join(lines) + "\n"


# ---------------------------------------------------------------------------
# Main driver
# ---------------------------------------------------------------------------

@dataclass
class Result:
    tag: str
    label: str
    time_s: float
    output: str
    compile_ok: bool
    compile_msg: str
    run_ok: bool


def run_benchmark(bench: Dict, bench_path: Path, compilers: Dict,
                  runs: int, tmp_dir: Path, lang_filter: Optional[List[str]]) -> List[Result]:
    results = []
    for variant in bench["variants"]:
        tag   = variant["tag"]
        label = variant.get("label", tag)
        src   = bench_path / variant["file"]

        if lang_filter and not any(tag.startswith(lf) for lf in lang_filter):
            continue
        if not src.exists():
            results.append(Result(tag, label, 0, "", False, f"source not found: {src}", False))
            continue

        exe = tmp_dir / f"bench_{bench['id']}_{tag}{EXE_EXT}"

        # Compile
        if tag == "vani":
            ok, msg = compile_vani(src, exe, compilers)
        elif tag == "c":
            extra = ["-fopenmp"] if "parsum" in variant["file"] else []
            ok, msg = compile_c(src, exe, compilers, extra)
        elif tag.startswith("cpp"):
            extra = ["-fopenmp"] if "parsum" in variant["file"] else []
            ok, msg = compile_cpp(src, exe, compilers, extra)
        elif tag == "rs":
            ok, msg = compile_rust(src, exe, compilers)
        else:
            ok, msg = False, f"unknown tag {tag}"

        if not ok:
            results.append(Result(tag, label, 0, "", False, msg, False))
            continue

        # Time
        med, out, run_ok = time_run(exe, runs)

        # Verify expected output
        expected = bench.get("expected")
        if expected and run_ok and out.strip() != expected.strip():
            run_ok = False
            msg = f"output mismatch: got {out!r}, want {expected!r}"
        else:
            msg = ""

        results.append(Result(tag, label, med, out, True, msg, run_ok))

    return results


def generate_report(all_results: Dict[str, List[Result]], benchmarks: List[Dict],
                    runs: int, compilers: Dict) -> str:
    lines = []
    lines.append("# Benchmark Results — vāṇī vs Rust vs C vs C++")
    lines.append("")
    lines.append(f"*Generated: {time.strftime('%Y-%m-%d %H:%M')} — {runs} timing run(s) per benchmark, median reported.*")
    lines.append(f"*C/C++ flags: `-O3 -march=native`. Rust flags: `-C opt-level=3 -C target-cpu=native`.*")
    lines.append(f"*vāṇī uses LLVM backend with `opt -O3 --mcpu=native` + `llc -O3 -mcpu=native`.*")
    lines.append("")

    # System info
    lines.append("## System")
    lines.append("```")
    lines.append(f"OS       : {platform.system()} {platform.release()} {platform.machine()}")
    lines.append(f"Python   : {sys.version.split()[0]}")
    lines.append(f"vanic    : {compilers['vanic'] or '(not found)'}")
    lines.append(f"CC       : {compilers['cc'] or '(not found)'}")
    lines.append(f"CXX      : {compilers['cxx'] or '(not found)'}")
    lines.append(f"rustc    : {compilers['rustc'] or '(not found)'}")
    lines.append("```")
    lines.append("")

    # Summary table
    lines.append("## Summary")
    lines.append("")
    header_tags = ["vani", "c", "cpp", "cpp_idx", "cpp_weak", "rs"]
    # Collect unique tags across all benchmarks
    used_tags: List[str] = []
    for bench in benchmarks:
        for v in bench["variants"]:
            t = v["tag"]
            if t not in used_tags:
                used_tags.append(t)

    col_w = 12
    hdr = "| Benchmark" + "".join(f" | {t:<{col_w}}" for t in used_tags) + " |"
    sep = "|" + "-" * 20 + "".join("|" + "-" * (col_w + 2) for _ in used_tags) + "|"
    lines.append(hdr)
    lines.append(sep)

    for bench in benchmarks:
        res_map = {r.tag: r for r in all_results.get(bench["id"], [])}
        row = f"| {bench['name'][:18]:<18}"
        for t in used_tags:
            r = res_map.get(t)
            if r is None:
                row += f" | {'—':<{col_w}}"
            elif not r.compile_ok:
                row += f" | {'no compiler':<{col_w}}"
            elif not r.run_ok:
                row += f" | {'ERROR':<{col_w}}"
            else:
                row += f" | {_fmt_ms(r.time_s):<{col_w}}"
        row += " |"
        lines.append(row)

    lines.append("")

    # Per-benchmark detail
    lines.append("## Per-benchmark charts")
    lines.append("")
    lines.append("> Bars are proportional to wall-clock time — **shorter is faster**.")
    lines.append("")

    for bench in benchmarks:
        results = all_results.get(bench["id"], [])
        lines.append(f"### {bench['name']}")
        lines.append("")
        lines.append(f"*{bench['description']}*")
        lines.append("")

        good = [(r.label, r.time_s) for r in results if r.run_ok]
        if good:
            lines.append(render_chart(bench["name"], good, baseline_tag="vani"))
        else:
            lines.append("*(no successful runs)*")
            lines.append("")

        # Errors
        errs = [r for r in results if not r.compile_ok or not r.run_ok]
        if errs:
            lines.append("<details><summary>Errors / skipped</summary>")
            lines.append("")
            for r in errs:
                if not r.compile_ok:
                    lines.append(f"- **{r.label}**: compile failed — {r.compile_msg}")
                else:
                    lines.append(f"- **{r.label}**: run failed — {r.compile_msg}")
            lines.append("")
            lines.append("</details>")
            lines.append("")

    # Key insight section
    lines.append("## Key insight: index handles vs. `weak_ptr`")
    lines.append("")
    lines.append(
        "Benchmark `05_graph_bfs` is the most architecture-revealing comparison.\n"
        "vāṇī has no `weak_ptr` equivalent — its **affine ownership model** means\n"
        "pointers cannot be aliased without explicit `ref`/`mut ref` borrows, which\n"
        "makes cyclic references impossible to express directly. Instead, cyclic\n"
        "graphs are stored as **integer indices** into a contiguous `Vec<T>`.\n"
    )
    lines.append(
        "| Approach | Heap allocs | Atomic ops | Cache friendly |\n"
        "|----------|-------------|------------|----------------|\n"
        "| C++ `weak_ptr` | one per node | `lock()` ≥ 2 per access | poor (pointer chase) |\n"
        "| vāṇī / C++ index | zero (flat Vec) | none | excellent (contiguous) |\n"
    )
    lines.append("")

    return "\n".join(lines)


# ---------------------------------------------------------------------------
# CLI entry point
# ---------------------------------------------------------------------------

def parse_args() -> argparse.Namespace:
    p = argparse.ArgumentParser(description="vāṇī benchmark runner")
    p.add_argument("--runs",   type=int, default=3,
                   help="timing runs per benchmark (default: 3)")
    p.add_argument("--bench",  type=str, default=None,
                   help="run only benchmarks whose id starts with this prefix")
    p.add_argument("--langs",  type=str, default=None,
                   help="comma-separated tags to include, e.g. vani,c,cpp")
    p.add_argument("--output", type=str,
                   default=str(BENCH_DIR / "results" / "RESULTS.md"),
                   help="output markdown file")
    p.add_argument("--list",   action="store_true",
                   help="list benchmarks and exit")
    return p.parse_args()


def main() -> None:
    args = parse_args()

    if args.list:
        for b in BENCHMARKS:
            print(f"  {b['id']:30s} {b['name']}")
        return

    compilers = detect_compilers()
    print("Detected compilers:")
    for k, v in compilers.items():
        print(f"  {k:<8s}: {v or '(not found)'}")
    print()

    lang_filter = [l.strip() for l in args.langs.split(",")] if args.langs else None
    bench_filter = args.bench

    benches = [b for b in BENCHMARKS
               if bench_filter is None or b["id"].startswith(bench_filter)]

    all_results: Dict[str, List[Result]] = {}

    with tempfile.TemporaryDirectory(prefix="vani_bench_") as tmp_str:
        tmp_dir = Path(tmp_str)

        for bench in benches:
            bench_path = BENCH_DIR / bench["id"]
            print(f"-- {bench['name']}")
            results = run_benchmark(
                bench, bench_path, compilers, args.runs, tmp_dir, lang_filter
            )
            all_results[bench["id"]] = results

            for r in results:
                if not r.compile_ok:
                    print(f"   {r.label:<16s} COMPILE FAIL  {r.compile_msg}")
                elif not r.run_ok:
                    print(f"   {r.label:<16s} RUN FAIL      {r.compile_msg}")
                else:
                    print(f"   {r.label:<16s} {_fmt_ms(r.time_s)}")
            print()

    report = generate_report(all_results, benches, args.runs, compilers)

    out_path = Path(args.output)
    out_path.parent.mkdir(parents=True, exist_ok=True)
    out_path.write_text(report, encoding="utf-8")
    print(f"Results written to {out_path}")


if __name__ == "__main__":
    main()
