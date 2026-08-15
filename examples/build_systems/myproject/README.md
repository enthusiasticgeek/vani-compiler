# myproject -- build-system integration example

A real, buildable project matching the layout
[`tutorials/src/intermediate/09d_build_systems.md`](../../../tutorials/src/intermediate/09d_build_systems.md)
documents: a multi-file vāṇी program (`src/main.vani` imports
`src/math.vani`) with two functions (`vani_square`, `vani_cube`)
exposed via `#[no_mangle]` so a plain C file (`c_helper.c`) can call
into them. All four files below build the *same* sources into the
*same* `myproject` binary -- pick whichever matches your own
project's toolchain, or diff them to see how the same 5-step
pipeline (vāṇी → C, compile, strip vāṇी's own `main`, compile the C
helper, link) maps onto each tool's own idioms.

```bash
make run                                   # -> build-make/myproject
cmake -B build-cmake && cmake --build build-cmake && ./build-cmake/myproject
meson setup build-meson && ninja -C build-meson && ./build-meson/myproject
ninja                                      # -> build-ninja/myproject (build.ninja, hand-written)
```

Every variant prints:

```
square(4) = 16
cube(3) = 27
```

Each build system gets its own output directory (`build-make/`,
`build-cmake/`, `build-meson/`, `build-ninja/`) so you can build all
four in the same checkout without them stepping on each other --
that's also why CMake's own default `build/` name isn't used here.

**The `main`-collision step, explained**: every vāṇी entry point
needs exactly one `fn main()`, and `vanic emit --backend=c` always
lowers it to a literal `int main(void)` in the generated C (that's
what makes `vanic build`'s own output directly runnable on its own).
Since `c_helper.c` has its own `main` too, linking the two objects
together as-is fails with `multiple definition of 'main'` -- every
build file here strips the symbol out of the vāṇī object first with
`objcopy -N main` before the final link. See `src/main.vani`'s own
comment and the tutorial's "Calling vāṇी functions from C" section
for the full writeup (verified there directly: link without the
`objcopy` step and you get exactly that linker error).

This directory is exercised by CI (`.github/workflows/ci.yml`'s
`build-systems-example` job) precisely so the tutorial's claims stay
true instead of rotting as markdown nobody runs.
