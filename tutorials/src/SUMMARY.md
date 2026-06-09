# Summary

[Introduction](introduction.md)

# Beginner

- [Hello, World](beginner/01_hello_world.md)
- [Variables, types, operators](beginner/02_variables.md)
- [Functions and the four return aliases](beginner/03_functions.md)
- [`if` / `else`](beginner/04_if_else.md)
- [`while` and `for` loops](beginner/05_loops.md)
- [Strings (`Str` vs `OwnedStr`)](beginner/06_strings.md)
- [Pointers and references — intuition primer](beginner/06a_pointers_refs_primer.md)
- [Heap and stack — intuition primer](beginner/06b_heap_vs_stack_primer.md)
- [Ownership and move — intuition primer](beginner/06c_ownership_primer.md)
- [Arrays and `Vec<T>` basics](beginner/07_vec_arrays.md)
- [Pattern match on integers + booleans](beginner/08_match.md)
- [First contract: `assert` / `prove` / `requires`](beginner/09_smt_intro.md)
- [Modules and namespaces — intuition primer](beginner/09a_modules_primer.md)
- [Modules and `pub`](beginner/10_modules.md)
- [Challenges](beginner/11_challenges.md)
- [Devanagari surface — optional intro](beginner/12_devanagari.md)

# Intermediate

- [Structs and methods](intermediate/01_struct_methods.md)
- [Enums with payloads + match arms](intermediate/02_enums_payloads.md)
- [Affine ownership: `ref` / `mut ref`](intermediate/03_affine.md)
- [`Box<T>` and RAII — intuition primer](intermediate/03a_box_raii_primer.md)
- [Generics and interfaces](intermediate/04_generics_iface.md)
- [What's a `dyn Iface`? — intuition primer](intermediate/04a_dyn_iface_primer.md)
- [Interfaces and static dispatch — intuition primer](intermediate/04b_interfaces_primer.md)
- [Generics and monomorphization — intuition primer](intermediate/04c_generics_primer.md)
- [Dynamic dispatch: `dyn Iface` + `Vec<dyn Iface>`](intermediate/05_dyn.md)
- [Closures and lambda lifting — intuition primer](intermediate/06a_closures_primer.md)
- [Closures and iterator combinators](intermediate/06_closures.md)
- [Tuples and tuple destructure](intermediate/07_tuples.md)
- [Multi-file projects + `vani.toml`](intermediate/08_manifest.md)
- [FFI: `extern "C"` + `--link-with`](intermediate/09_ffi.md)
- [Error handling: `Result<T, E>` + `try`](intermediate/10_result_try.md)
- [The 22 GoF design patterns](intermediate/11_design_patterns.md)
- [SMT — `requires` / `ensures` intuition primer](intermediate/12a_smt_primer.md)
- [SMT verification deep-dive](intermediate/12_smt_deepdive.md)

# Advanced

- [Async / await and the `Task` transform](advanced/01_async.md)
- [Parallelism and race-freedom — intuition primer](advanced/02a_parallelism_primer.md)
- [`parallel for` + reductions + race-freedom](advanced/02_parallel.md)
- [`task` / `join` + atomics / mutexes / channels](advanced/03_concurrency.md)
- [Embedded targets + `unsafe` + region typing](advanced/04_embedded.md)
- [The `dyn` vtable layout + safety boundary](advanced/05_vtables.md)
- [SMT trace debugging](advanced/06_smt_debug.md)
- [Devanagari purity arc](advanced/07_devanagari_purity.md)
- [Writing a cross-language translator extension](advanced/08_translator.md)
- [Adding a new dialect (Tier I rollout)](advanced/09_new_dialect.md)
- [Compiler internals tour](advanced/10_internals.md)

---

[Contributing to the tutorials](contributing.md)
