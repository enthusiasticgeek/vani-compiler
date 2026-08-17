# vāṇी tutorials (mdBook)

This is the mdBook source for the vāṇī (वाणी) tutorial site.
The compiled HTML lives in `tutorials/book/` after a build and
is the artifact that gets deployed (eventually — see TUT-5 in
[TODO.md](../TODO.md)).

## Status

- **TUT-1** ✅ SHIPPED 2026-06-07 — scaffolding (this dir
  layout + `book.toml` + SUMMARY + introduction + stub lessons +
  the first Beginner lesson as a template).
- **TUT-2** ✅ SHIPPED 2026-06-07 — 12 Beginner lessons (Hello
  World → variables → functions → if/else → loops → strings →
  Vec → match → SMT contracts → modules → challenges →
  Devanagari intro). Every worked example is compile-verified.
- **TUT-3** ✅ SHIPPED 2026-06-07 — 12 Intermediate lessons
  (structs/methods → enums-with-payloads → affine ownership →
  generics → dyn → closures → tuples → multi-file → FFI →
  Result+try → GoF patterns → SMT deep-dive).
- **TUT-4** ✅ SHIPPED 2026-06-07 — 10 Advanced lessons
  (async → parallel → concurrency → embedded → vtable layout →
  SMT debug → Devanagari purity → translator → new-dialect
  walkthrough → compiler internals tour).
- **TUT-5** ✅ SHIPPED 2026-06-17 — GitHub Actions workflow
  (`.github/workflows/deploy-tutorials.yml`) builds with mdBook 0.4.40
  and deploys to GitHub Pages on every push that touches `tutorials/`.
  Live at <https://enthusiasticgeek.github.io/vani-compiler/>.
- **TUT-6** ✅ SHIPPED 2026-08-17 — a "Translate this page" language-picker
  dropdown in the menu bar (`theme/translate-link.js`), for readers not
  fluent in English. Links directly to Google's `<host>.translate.goog`
  proxy (not `translate.google.com/translate`'s own redirect page, which
  Chrome for Android intercepts with a native flow that fails
  independently of the link) for whichever of ~34 languages the reader
  picks — not auto-guessed from the browser locale, so a reader fluent in
  more than one language isn't stuck with just their default. CI's pinned
  mdBook 0.4.40 renders the menu bar as `#menu-bar`, not the newer
  `#mdbook-menu-bar` a local 0.5.x build produces — the script matches
  both.

## Build it locally

You need `mdbook` (>= 0.4) on your `PATH`. Install via:

```bash
# Rust toolchain (cleanest)
cargo install mdbook

# Snap (no Rust toolchain needed)
sudo snap install mdbook

# Homebrew (macOS)
brew install mdbook
```

Then from this directory:

```bash
mdbook serve --open    # live-reload dev server
mdbook build           # one-shot build → ./book/
mdbook clean           # delete the build output
```

The first command opens the rendered book in your browser; any
edit under `src/` triggers a rebuild + page reload.

## Directory layout

```
tutorials/
├── README.md                # this file
├── book.toml                # mdBook config (theme / search / git URLs)
├── theme/                   # additional-css/additional-js (manas mascot,
│                             # the "Translate this page" language-picker
│                             # dropdown in the menu bar)
├── src/
│   ├── SUMMARY.md           # Table of contents — mdBook entry
│   ├── introduction.md      # Landing page
│   ├── contributing.md      # How to write a lesson
│   ├── why_vani.md, glossary.md, installation.md
│   ├── beginner/            # 26 lessons, all written (Hello World through
│   │                         # the Big-O primer)
│   ├── intermediate/        # 45 lessons, all written (struct methods
│   │                         # through the tic-tac-toe capstone)
│   └── advanced/            # 25 lessons, all written (async through the
│                             # job-scheduler capstone)
└── book/                    # gitignored — produced by `mdbook build`
```

## Contributing

See [`src/contributing.md`](src/contributing.md) for the lesson
template, style guide, and the build/deploy loop.

## Why mdBook?

`docs/v1_limitations.md`-style flat Markdown isn't enough for a
progressive tutorial — you need lesson-by-lesson navigation,
search, and a stable URL per lesson. mdBook gives all three with
~0 setup: single config file, one binary, GitHub-Pages-friendly
output. The cost of switching to Docusaurus / VitePress later is
"rename `book.toml` and tweak the headers"; we're not painting
ourselves into a corner.
