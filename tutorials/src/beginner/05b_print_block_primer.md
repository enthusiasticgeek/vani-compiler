# Beginner 5b -- Print Blocks `print { ... }`

> **Learning goal**: group multiple print lines under a single `print`
> keyword so you don't have to repeat `print` on every line.

---

## The problem: repetitive `print`

When you need to print several related values in a row, the straightforward
approach repeats `print` every line:

```vani
print "name:  ", name;
print "score: ", score;
print "rank:  ", rank;
print "level: ", level;
```

This works, but the repeated keyword is noise when the intent is clearly
"print a block of related output."

---

## Print block syntax

A **print block** groups all those lines under one `print` keyword.
Each `;`-terminated group inside the braces becomes one output line:

```vani
print {
  "name:  ", name;
  "score: ", score;
  "rank:  ", rank;
  "level: ", level;
}
```

This is *exactly* equivalent to the four separate `print` statements above --
same output, same order, same newline after each group. The `print { }` form
is just tidier when you're printing a block of related data.

---

## Each group is one line

The semicolons separate independent output lines. Each `;`-terminated group
can contain as many comma-separated items as a regular `print`:

```vani
fn report(label: Str, a: i64, b: i64, total: i64) -> i64 {
  print {
    label + ":";
    "  a     =", a;
    "  b     =", b;
    "  total =", total;
  }
  return 0;
}
```

`print` always inserts its own single space between comma-separated
items, on top of whatever's already in the string -- so the label
strings above end at `=` (no trailing space) rather than `= ` (with
one), and `label` is concatenated with `":"` via `+` rather than
comma-joined, or the output would come out double-spaced (`a     =  3`,
`test :`).

Output when called with `report("test", 3, 4, 7)`:

```
test:
  a     = 3
  b     = 4
  total = 7
```

---

## Works inside loops

Print blocks work anywhere a regular `print` does -- including inside `for`
and `while` loops:

```vani
fn main() -> i64 {
  for i from 0 to 3 {
    let sq: i64 = i * i;
    print {
      "i  =", i;
      "i^2 =", sq;
      "---";
    }
  }
  return 0;
}
```

Output:

```
i  = 0
i^2 = 0
---
i  = 1
i^2 = 1
---
i  = 2
i^2 = 4
---
```

---

## `eprint` doesn't have a block form (yet)

Unlike `print`, `eprint` (which writes to stderr) does **not**
support the `{ ... }` block form -- `eprint { ... }` is a parse
error today. For multiple related stderr lines, repeat `eprint`:

```vani
eprint "[ERROR] file not found";
eprint "  path = ", path;
```

---

<img class="manas" src="../images/mascot/manas_mascot_awesome.png" title="a good habit worth adopting"/>

## When to use print block vs plain print

| Use | Form |
|-----|------|
| Single item or quick debug line | `print value;` |
| Two items on the same output line | `print "label: ", value;` |
| Three or more related output lines | `print { ... }` |

---

## Full example

```vani
fn show_stats(name: Str, min: i64, max: i64, avg: i64) -> i64 {
  print {
    "===", name, "===";
    "  min =", min;
    "  max =", max;
    "  avg =", avg;
  }
  return 0;
}

fn main() -> i64 {
  show_stats("dataset A", 3, 97, 42);
  show_stats("dataset B", 11, 88, 55);
  return 0;
}
```

Expected output:

```
=== dataset A ===
  min = 3
  max = 97
  avg = 42
=== dataset B ===
  min = 11
  max = 88
  avg = 55
```

---

**Previous**: [Sec.5a -- Recursion intuition primer ->](05a_recursion_primer.md)
**Next**: [Sec.5c -- Named loop labels ->](05c_loop_labels_primer.md)
