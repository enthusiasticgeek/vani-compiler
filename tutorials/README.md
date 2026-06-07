# vāṇी tutorials (mdBook)

This is the mdBook source for the vāṇी (वाणी) tutorial site.
The compiled HTML lives in `tutorials/book/` after a build and
is the artifact that gets deployed (eventually — see TUT-5 in
[TODO.md](../TODO.md)).

## Status

- **TUT-1** ✅ SHIPPED 2026-06-07 — scaffolding (this dir
  layout + `book.toml` + SUMMARY + introduction + stub lessons +
  the first Beginner lesson as a template).
- **TUT-2** queued — write the 12 Beginner lessons.
- **TUT-3** queued — write the 12 Intermediate lessons.
- **TUT-4** queued — write the 10 Advanced lessons.
- **TUT-5** queued — GitHub Actions deploy to `gh-pages`.

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
├── src/
│   ├── SUMMARY.md           # Table of contents — mdBook entry
│   ├── introduction.md      # Landing page
│   ├── contributing.md      # How to write a lesson
│   ├── beginner/            # 12 lessons (1 written, 11 stubs)
│   ├── intermediate/        # 12 stubs
│   └── advanced/            # 10 stubs
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
