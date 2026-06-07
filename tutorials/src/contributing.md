# Contributing to the tutorials

The tutorials site is an mdBook. Each lesson is a single
Markdown file under `tutorials/src/`. To contribute:

1. **Pick a lesson** from [SUMMARY.md](SUMMARY.md). Stub
   lessons (everything except *Beginner 1 — Hello, World*) are
   open for writing.
2. **Use the template** from
   [`beginner/01_hello_world.md`](beginner/01_hello_world.md):
   - Learning goal (one sentence)
   - Worked example (a small program that compiles)
   - Compile + run commands (literal `vanic` invocations)
   - Why it works that way (3-5 design notes)
   - A challenge (an exercise + a collapsed solution block)
3. **Verify the example compiles**. Paste it into a `.vani`
   file and run `vanic run <file>` AND `vanic run <file>
   --backend=c`. If both backends print the expected output,
   the example is good.
4. **Build the site locally** with `mdbook serve --open` from
   the `tutorials/` directory. The page hot-reloads on save.
5. **Cross-reference v1 limitations**. If a lesson is about a
   feature with documented v1 deviations (see
   [`docs/v1_limitations.md`](https://github.com/anthropics/claude-code/blob/main/docs/v1_limitations.md)),
   call them out in the *"Why it works that way"* section so
   readers don't get surprised when their textbook version
   doesn't compile.

## Style guide

- **Compile-tested code** only. Examples that don't compile are
  worse than no example.
- **English keywords by default**. The Devanagari surface gets
  its own dedicated lessons (`beginner/12` and `advanced/07`).
- **One concept per lesson**. Resist the urge to introduce
  generics in the strings lesson.
- **Link forward freely**. If a topic comes back later (e.g.
  `print` re-explained in the dialect lesson), link there.
- **No prose padding**. Tight, terse, code-forward.

## Building + deploying

```bash
# Build (output lands in `tutorials/book/`)
cd tutorials/
mdbook build

# Live-reload dev server
mdbook serve --open

# Cleanup
mdbook clean
```

GitHub Pages deployment is queued as **TUT-5** — see
[TODO.md](https://github.com/anthropics/claude-code/blob/main/TODO.md).
