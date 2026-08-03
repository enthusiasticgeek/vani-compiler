# Fix attempt (drafted by qwen2.5-coder:1.5b, UNVERIFIED unless marked otherwise)

Heuristic likely-area hint given to the model: src/checker.rs, or both src/backend_c.rs and src/backend_llvm.rs (both crashed)

## qwen's response

Based on the failing program and symptom data, it is most likely that the issue lies in the logic within the `nedtaelling` function. The function should iterate from `fra_n` down to 1, adding each number to a sum until reaching 0. However, there are several potential issues with this implementation:

1. **Loop Condition**: The loop condition `i > 0` is incorrect. It should be `i >= 0` to ensure the loop continues as long as there are still numbers left to subtract from `fra_n`.

2. **Correct Loop Body**: The current code is missing a return statement after the first addition, which could lead to unexpected behavior.

3. **Loop Invariant Check**: There's no check that ensures the loop invariant `i >= 0` holds throughout the loop.

Here is the corrected version of the `nedtaelling` function:

```vani
// vani-lang: danish
//
// build & run:
//   vanic run examples/language/danish/control_flow.vani              # LLVM
//   vanic run examples/language/danish/control_flow.vani --backend=c  # C

formaal "Danish control flow — hvis/ellers/mens";

funktion tegn(n: i64) -> i64 {
  hvis n > 0 {
    returner 1;
  } ellers hvis n < 0 {
    returner -1;
  } ellers {
    returner 0;
  }
}

funktion nedtaelling(fra_n: i64) -> i64 {
  lad i: i64 = fra_n;
  lad sum: i64 = 0;
  mens i >= 0 {  // Correct loop condition
    sum = sum + i;
    i = i - 1;  // Correct loop body
  }
  returner sum;
}

fn main() -> i64 {
  bekraeft tegn(5)  == 1;
  bekraeft tegn(-3) == -1;
  bekraeft tegn(0)  == 0;
  bekraeft nedtaelling

## Outcome

No patch attempted -- needs frontier-model or human review from scratch.
