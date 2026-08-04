# Fix attempt (drafted by qwen2.5-coder:1.5b, UNVERIFIED unless marked otherwise)

Heuristic likely-area hint given to the model: src/backend_c.rs and src/backend_llvm.rs -- compare their codegen for the construct involved, one of them is wrong

## qwen's response

The observed bug seems to lie in the backend code generation for handling `ref CancelToken` parameters. The v3.1 transform auto-injects a check `if token.cancelled { return 0 - 1; }` before every suspend point, but it is being incorrectly handled by either the C or LLVM backends.

To resolve this issue, we need to ensure that the check for the cancellation state (`token.cancelled`) is correctly injected and not inadvertently removed during the backend code generation process. We can do this by inspecting the `src/backend_c.rs` and `src/backend_llvm.rs` files and making necessary adjustments.

**CONFIDENCE: high**

## Outcome

No patch attempted -- needs frontier-model or human review from scratch.
