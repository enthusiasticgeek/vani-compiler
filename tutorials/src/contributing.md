# Contributing to the tutorials

The tutorials site is an mdBook. Each lesson is a single
Markdown file under `tutorials/src/`. To contribute:

1. **Pick a lesson** from [SUMMARY.md](SUMMARY.md). Stub
   lessons (everything except *Beginner 1 â€" Hello, World*) are
   open for writing.
2. **Use the template** from
   [`beginner/01_hello_world.md`](beginner/01_hello_world.md):
   - Learning goal (one sentence)
   - Worked example (a small program that compiles)
   - Compile + run commands (literal `vanic` invocations)
   - Why it works that way (3-5 design notes)
   - A challenge (an exercise + a collapsed solution block)
   - Mascot markers on every code block that needs one (see
     [introduction.md](introduction.md) for the full system). Place
     the `<img>` tag on its own line immediately before the fenced
     code block it annotates:
     `<img class="manas" src="../images/mascot/manas_mascot_error.png" title="this code does not compile!"/>`.
     Swap the filename and title for the case that applies:
     - `manas_mascot_error.png`, "this code does not compile!" --
       broken code, intentional compile error.
     - `manas_mascot_caution.png`, "this code needs extra care" --
       compiles cleanly but is subtle or easy to misuse.
     - `manas_mascot_success.png`, "this is the correct, working
       version" -- the fixed/working counterpart to an error or
       caution example.
     - `manas_mascot_awesome.png`, "a good habit worth adopting" --
       best-practice tip.
     - `manas_mascot_waiting.png`, "work in progress / not yet
       implemented" -- a feature that isn't implemented yet.
     Not every code block needs a marker -- plain worked examples
     that just work don't need one -- but if your lesson shows
     broken code, a gotcha, a fix, a best practice, or a
     not-yet-implemented feature, mark it.
3. **Verify the example compiles**. Paste it into a `.vani`
   file and run `vanic run <file>` AND `vanic run <file>
   --backend=c`. If both backends print the expected output,
   the example is good.
4. **Build the site locally** with `mdbook serve --open` from
   the `tutorials/` directory. The page hot-reloads on save.
5. **Cross-reference v1 limitations**. If a lesson is about a
   feature with documented v1 deviations (see
   [`docs/v1_limitations.md`](https://github.com/enthusiasticgeek/vani-compiler/blob/main/docs/v1_limitations.md)),
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

GitHub Pages deployment is queued as **TUT-5** â€" see
[TODO.md](https://github.com/enthusiasticgeek/vani-compiler/blob/main/TODO.md).
