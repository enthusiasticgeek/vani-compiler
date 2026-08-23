# Beginner 14 -- Capstone: a class grade-report tool

> **Learning goal**: build one small, complete, *real* program out of
> everything from Sec.1-13a -- functions, `if`/`else`, `while` loops,
> named loop labels, tuples, `Vec<i64>`, `match`, `Option<T>`,
> `module`/`pub`, and a `requires`/`assert`/`prove` contract -- with
> nothing new to learn. This chapter is about combining what you
> already have into a program that does something a real teacher
> might actually want.

This is a walking tour of
[`examples/language/english/gradebook_capstone.vani`](https://github.com/enthusiasticgeek/vani-compiler/blob/main/examples/language/english/gradebook_capstone.vani),
a small class grade-report tool. Every code block below is the *actual*
shipped source -- run the real file alongside this chapter.

## Build & run

```bash
vanic run examples/language/english/gradebook_capstone.vani                     # LLVM backend
vanic run examples/language/english/gradebook_capstone.vani --backend=c         # C backend
```

Both backends print an identical report: a per-student score/grade
table, then class-wide statistics, then two small "detective" searches
over the roster, then one edge case (an empty class) handled without
crashing.

---

## The data: one `Vec<i64>`, nothing fancier

```vani
let roster: Vec<i64> = vec(92, 67, 74, 55, 88, 91, 40, 73, 85, 60);
```

Ten students, one score each. Just like the tic-tac-toe capstone gets
away with a flat `Vec<i64>` board instead of a `struct`, a roster of
"score per student" needs nothing fancier than a `Vec<i64>` -- the
*index* is the student's position (0-based internally, printed
1-based), and the *value* at that index is their score. No struct
needed for something this size ([Beginner 7](07_vec_arrays.md)).

---

## Organizing the logic: a `module`

Every piece of grade-book logic lives inside `module gradebook { ... }`
([Beginner 10](10_modules.md)), called from `main` as
`gradebook::letter_grade(s)`, `gradebook::min_max(ref roster)`, and so
on. One helper, `sum_scores`, has no `pub` -- it's an implementation
detail `class_average` needs and nothing outside the module should call
directly:

```vani
module gradebook {
  // Private helper -- not `pub`, so only code inside this module can
  // call it. class_average() below calls it bare (no `gradebook::`
  // prefix needed for an intra-module call).
  fn sum_scores(scores: ref Vec<i64>) -> i64 {
    let total: i64 = 0;
    let i: u64 = 0;
    while i < len(scores) {
      total = total + scores[i];
      i = i + 1;
    }
    return total;
  }
  // ...
}
```

This is the exact "private by default, `pub` marks the real API"
lesson from [Beginner 10](10_modules.md), used for a real reason
instead of a toy example: nothing outside `gradebook` has any business
adding up raw scores directly -- only `class_average`'s *meaning*
("the average, or none if there's no roster") is anyone else's
concern.

---

## Failure as a value: `class_average` returns `Option<i64>`

An average is undefined for zero students -- there's no honest number
to return. Rather than pick an arbitrary sentinel (`0`? `-1`? both are
lies a caller could mistake for a real average), `class_average`
returns `Option<i64>`, exactly the pattern from
[Beginner 8b -- Errors as values](08b_errors_primer.md):

```vani
pub fn class_average(scores: ref Vec<i64>) -> Option<i64> {
  if len(scores) == 0 {
    return Option.None;
  }
  return Option.Some(sum_scores(scores) / (len(scores) as i64));
}
```

The call site can't accidentally forget the empty case -- `match` is
exhaustive ([Beginner 8](08_match.md)), so both arms have to be
written:

```vani
let avg_msg: OwnedStr = match gradebook::class_average(ref roster) {
  Option.Some(avg) then "class average: " + i64_to_str(avg),
  Option.None then "no scores recorded" + "",
};
print avg_msg;
```

`match` is an expression, not a statement, so the message is built
first and printed once -- the same shape [Beginner 8b](08b_errors_primer.md)
used for `safe_div`'s result.

Near the bottom of `main`, the same function is called again on a
genuinely empty roster (`let empty: Vec<i64> = vec();`) to prove the
`Option.None` branch isn't just theoretical -- it's the branch that
actually runs for that call:

```
empty roster -- no average to report
```

No division-by-zero crash, no wrong number silently printed -- the
type system made the empty case impossible to skip.

---

## A contract: `requires`, `assert`, and `prove` together

`min_max` returns the smallest and largest score as a **tuple**
([Beginner 7a](07a_tuples_primer.md)) -- a quick "two values, no name
needed" case, exactly the `divmod` example from that chapter:

```vani
pub fn min_max(scores: ref Vec<i64>) -> (i64, i64)
requires len(scores) > 0;
{
  let lo: i64 = scores[0];
  let hi: i64 = scores[0];
  let i: u64 = 1;
  while i < len(scores) {
    if scores[i] < lo { lo = scores[i]; }
    if scores[i] > hi { hi = scores[i]; }
    i = i + 1;
  }
  assert lo <= hi;
  return (lo, hi);
}
```

Three contract keywords from [Beginner 9](09_smt_intro.md), all in one
small function:

- **`requires len(scores) > 0;`** -- a precondition. `scores[0]` on the
  very first line would be nonsense (or a bounds-check trap) on an
  empty Vec, so the function states up front what it needs.
- **`assert lo <= hi;`** -- a self-check before returning. It happens
  to be provable from the loop's own logic, so the SMT solver
  discharges it at compile time; no runtime cost.
- **`prove 100 == 100;`** appears once in `main` -- a trivial,
  compile-time-proved fact, included purely as a reminder that `prove`
  exists alongside `requires`/`assert`, the same way
  [Beginner 9](09_smt_intro.md) itself opened with `prove 2 + 2 == 4;`.

The call site destructures the tuple exactly like the primer taught:

```vani
let (lo, hi) = gradebook::min_max(ref roster);
print "min score:", lo;
print "max score:", hi;
```

---

## Grading: `match` with guards

```vani
pub fn letter_grade(score: i64) -> Str {
  return match score {
    _ if score >= 90 then "A",
    _ if score >= 80 then "B",
    _ if score >= 70 then "C",
    _ if score >= 60 then "D",
    _ then "F",
  };
}
```

vāṇी's `match` has no `1..99`-style range pattern, so a grading
scale is written as a chain of guarded wildcard arms -- exactly the
"range-like dispatch" idiom from
[Beginner 8a](08a_pattern_match_primer.md#range-like-dispatch-chained-guards),
checked here against real data instead of an abstract `code`.

---

## Two nested-loop searches, and why one needs a label

`find_duplicate_score` answers "did any two students score exactly
the same?" by comparing every pair -- the "everyone-against-everyone"
`O(n²)` shape from [Beginner 13a -- Big-O](13a_big_o_primer.md):

```vani
pub fn find_duplicate_score(scores: ref Vec<i64>) -> Option<i64> {
  let n: u64 = len(scores);
  let i: u64 = 0;
  while i < n {
    let j: u64 = i + 1;
    while j < n {
      if scores[i] == scores[j] {
        return Option.Some(i as i64);
      }
      j = j + 1;
    }
    i = i + 1;
  }
  return Option.None;
}
```

`return` inside the inner loop is enough here -- it exits both loops
and the whole function at once, so no label is needed.

`find_perfect_pair` asks a similar question ("do any two different
students' scores add up to exactly 100?") but answers it a different
way -- by setting a variable and then falling out of *both* loops with
a **labeled `break`**, the exact pattern
[Beginner 5c](05c_loop_labels_primer.md)'s own challenge used:

```vani
pub fn find_perfect_pair(scores: ref Vec<i64>) -> Option<i64> {
  let n: u64 = len(scores);
  let found_i: i64 = 0 - 1;
  let i: u64 = 0;
  search: while i < n {
    let j: u64 = i + 1;
    while j < n {
      if scores[i] + scores[j] == 100 {
        found_i = i as i64;
        break search;
      }
      j = j + 1;
    }
    i = i + 1;
  }
  if found_i < 0 {
    return Option.None;
  }
  return Option.Some(found_i);
}
```

Both functions solve the same *shape* of problem; showing them side by
side is the point. `find_duplicate_score` could have used a label too,
but doesn't need one -- `return` already does the job. `find_perfect_pair`
keeps working *after* the loop (turning `found_i` into an `Option`), so
it needs to get out of both loops without leaving the function --
that's exactly the situation [Beginner 5c](05c_loop_labels_primer.md)
introduced labels to solve cleanly, instead of an extra flag variable
checked after the loop.

On this ten-student roster, `find_duplicate_score` finds nothing
(every score is unique) and `find_perfect_pair` finds student 7
(score 40) and student 10 (score 60), whose scores sum to exactly 100.

---

## Tying it together

```vani
fn print_report(scores: ref Vec<i64>) -> i64 {
  let i: u64 = 0;
  while i < len(scores) {
    let rank: i64 = (i as i64) + 1;
    let s: i64 = scores[i];
    let g: Str = gradebook::letter_grade(s);
    print {
      "student", rank, ": score =", s, " grade =", g;
    }
    i = i + 1;
  }
  return 0;
}
```

`print_report` uses a `print` block ([Beginner 5b](05b_print_block_primer.md))
for the one line it emits per student, and reaches back into
`while` + `u64` indexing for the loop itself -- the same idiom
[Beginner 7](07_vec_arrays.md) used for `sum_vec`/`count_positive`,
picked deliberately over `for ... to` because the bound (`len(scores)`)
is a runtime `u64`, not a small compile-time range. `for` shows up
plenty elsewhere in the tracks; `while` is the natural fit for walking
a `Vec` end to end.

`main` calls every piece in turn -- report, average, min/max, passing
count, both searches, then the empty-roster edge case -- and every
`Option`-returning call goes through the same `match`-into-`OwnedStr`-
then-`print` shape. Nothing here is a new idea; it's the same handful
of tools from the last thirteen chapters, used together.

---

## Try it yourself

1. Add a `median(scores: ref Vec<i64>) -> Option<i64>` function
   (`None` for an empty roster, same as `class_average`). You'll need
   to sort a copy of the Vec first -- `sort(mut ref xs)` is a builtin;
   check [Beginner 13a](13a_big_o_primer.md) for its complexity.
2. Change `letter_grade`'s cutoffs to include `+`/`-` grades (`A-` at
   87, `B+` at 87... pick your own scale) -- notice how naturally the
   guarded-wildcard-chain shape absorbs more arms.
3. Rewrite `passing_count` recursively instead of with a `while` loop,
   then compare: which reads clearer for this problem? (
   [Beginner 5a](05a_recursion_primer.md) has an opinion on when
   recursion is and isn't the right tool -- see if you agree once
   you've written both versions.)
4. Add a `Str`-typed roster (student names, `Vec<Str>`, same length as
   `roster`) and print each student's *name* instead of "student N" in
   the report -- you'll need to index both Vecs with the same loop
   variable.
5. *(Bigger)* Read the roster from `stdin` instead of hardcoding it --
   one score per line, `stdin_read_line()` + `parse_int()` in a loop
   until a blank line, same pattern the
   [tic-tac-toe capstone](../intermediate/17_tic_tac_toe_capstone.md)
   uses to read a move. You'll be using an intermediate-track
   technique early -- that's fine, it's a good preview of what's next.

---

## Summary

- A flat `Vec<i64>` was enough state for a whole small program -- no
  struct needed yet.
- `Option<T>` turned "what's the average of zero students?" from a
  crash risk into a compile-time-enforced branch every caller has to
  handle.
- `requires`/`assert`/`prove` cost nothing at runtime here -- the SMT
  solver discharges every one of them at compile time.
- Two nested-loop searches solved similar-shaped problems two
  different ways: plain `return` when exiting the function
  immediately is enough, a labeled `break` when the code needs to keep
  running *after* leaving both loops.
- None of this used a single language feature beyond
  [Beginner 13a](13a_big_o_primer.md) -- the point of a capstone is
  that you already had everything you needed.

---

**Previous**: [Sec.13a -- Big-O notation primer ->](13a_big_o_primer.md)
**Next**: [Intermediate track: Structs and methods ->](../intermediate/01_struct_methods.md)
