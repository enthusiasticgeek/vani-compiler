# Beginner 5c -- Named Loop Labels

> **Learning goal**: label a loop by name and use `break label` /
> `continue label` to exit or skip iterations of a *specific* enclosing
> loop when loops are nested.

---

## Why plain `break` is sometimes not enough

Plain `break` always exits the **innermost** loop. That's the right
behavior most of the time -- but when loops are nested, you sometimes
want to exit a *specific* outer loop, not just the one you're currently
inside:

```vani
for i from 0 to 5 {
  for j from 0 to 5 {
    if some_condition {
      break;   /* only exits the j-loop; i-loop keeps running */
    }
  }
}
```

If `some_condition` means "we're completely done -- stop everything,"
plain `break` doesn't cut it. You'd need a flag variable and an extra
check after the inner loop, which is messy.

**Named loop labels** solve this cleanly.

---

## Labeling a loop

Put an identifier followed by `:` directly before the loop keyword:

```vani
outer: for i from 0 to 5 {
  /* this loop is now named "outer" */
}

search: while has_more_data {
  /* this loop is now named "search" */
}
```

Any valid identifier works as a label name. Common choices are `outer`,
`inner`, `middle`, `search`, `retry`, `scan` -- whatever is meaningful
at the call site.

---

## `break label` -- exit a specific loop

`break label_name;` exits the loop *named* `label_name` and everything
nested inside it. Execution continues after that loop's closing `}`:

```vani
fn main() -> i64 {
  outer: for i from 0 to 5 {
    inner: for j from 0 to 5 {
      if i == 2 {
        break outer;   /* exits both loops immediately */
      }
      print i, j;
    }
  }
  print "done";
  return 0;
}
```

Output (only `i=0` and `i=1` rows run; `i=2` fires `break outer`):

```
0 0
0 1
0 2
0 3
0 4
1 0
1 1
1 2
1 3
1 4
done
```

---

## `continue label` -- skip to next iteration of a specific loop

`continue label_name;` skips the rest of the **labeled loop's** body
(including everything nested inside it) and starts that loop's next
iteration:

```vani
fn main() -> i64 {
  outer: for i from 0 to 4 {
    for j from 0 to 4 {
      if j == 2 {
        continue outer;   /* skip j=2, j=3, and any outer tail code; go to i+1 */
      }
      print i, j;
    }
    print "tail of outer";   /* never reached -- continue outer skips this */
  }
  return 0;
}
```

Output (each `i` stops at `j=2` and jumps to the next `i`):

```
0 0
0 1
1 0
1 1
2 0
2 1
3 0
3 1
```

Notice "tail of outer" never prints -- `continue outer` skips all
remaining code in the outer loop body, not just the inner loop.

---

## Three nested loops -- the real power

With three or more nesting levels, named labels let you pick exactly
which layer to break or continue without any flag variables:

```vani
fn main() -> i64 {
  let count: i64 = 0;

  outer: for i from 0 to 5 {
    middle: for j from 0 to 5 {
      inner: for k from 0 to 10 {
        if k == 3 { break inner; }    /* exits k-loop; j and i continue  */
        if j == 2 { continue outer; } /* skips to next i; middle + inner exit */
        if i == 4 { break middle; }   /* exits middle + inner; i continues     */
        count = count + 1;
      }
    }
  }

  print count;
  return 0;
}
```

| Statement | Exits / skips |
|-----------|--------------|
| `break inner` | k-loop only |
| `break middle` | middle-loop + k-loop |
| `break outer` | all three loops |
| `continue inner` | next k iteration |
| `continue middle` | next j iteration (skips remaining k iterations) |
| `continue outer` | next i iteration (skips remaining j and k iterations) |

---

## Undefined label -> compile error

`break label_name;` only takes the *label* interpretation when
`label_name` actually names a loop that's currently in scope. If it
doesn't match any enclosing loop's label, the compiler instead treats
it as a **break value** (a `while`/`for` loop used as an expression can
return a value via `break val;`, e.g. `let x = while ... { break val; };`)
-- and since a plain `for`/`while` *statement* isn't being used as an
expression, that's caught at compile time too, just with a different
message:

<img class="manas" src="../images/mascot/manas_mascot_error.png" title="this code does not compile!"/>

```vani
outer: for i from 0 to 3 {
  break nowhere;   /* `nowhere` isn't a label in scope here, so this
                       is read as a break VALUE instead */
}
```

```
error: 'break value' is only valid when the loop is used as an expression (e.g. `let x = while ... { break val; }`)
    break nowhere;
```

**Fix**: break by a label that's actually defined on an enclosing loop:

<img class="manas" src="../images/mascot/manas_mascot_success.png" title="this is the correct, working version"/>

```vani
outer: for i from 0 to 3 {
  break outer;   /* "outer" is a real label on an enclosing loop */
}
```

---

## Works on `while` loops too

Labels attach to `while` as naturally as to `for`:

```vani
fn search(target: i64) -> i64 {
  let i: i64 = 0;
  found: while i < 100 {
    let j: i64 = 0;
    while j < 100 {
      if i * j == target {
        break found;   /* done -- exit both loops */
      }
      j = j + 1;
    }
    i = i + 1;
  }
  return i;
}
```

---

## Quick reference

```vani
/* Label syntax */
name: for var from start to end { ... }
name: while condition { ... }

/* Targeted break -- exits named loop and everything inside it */
break name;

/* Targeted continue -- skips to next iteration of named loop */
continue name;

/* Plain break/continue still work -- they target the innermost loop */
break;
continue;
```

---

## Challenge

Write a program that finds the first pair `(i, j)` where `i * j == 42`
and `0 <= i, j < 20`. Print the pair and stop -- don't print any other pairs.
Use a labeled `break` to exit both loops at once.

<details>
<summary>Solution</summary>

```vani
fn main() -> i64 {
  search: for i from 0 to 20 {
    for j from 0 to 20 {
      if i * j == 42 {
        print "found:", i, "*", j, "= 42";
        break search;
      }
    }
  }
  return 0;
}
```

Output: `found: 3 * 14 = 42` -- `j` runs fully for each `i` before `i` advances, so the *first* pair found in iteration order is `(3, 14)`, not `(6, 7)` (which `search` never even reaches -- it breaks out at `i == 3` first).

</details>

---

**Previous**: [Sec.5b -- Print blocks ->](05b_print_block_primer.md)
**Next**: [Sec.6a -- Pointers and references primer ->](06a_pointers_refs_primer.md)
