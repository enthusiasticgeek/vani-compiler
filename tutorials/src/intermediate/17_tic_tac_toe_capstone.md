# Intermediate 17 -- Capstone: a terminal tic-tac-toe game

> **Learning goal**: build one small, complete, *real* program from
> pieces you already have -- `Vec<i64>` as mutable state, `ref`
> parameters, `stdin_read_line()` + `parse_int()` + `if let`,
> string concatenation, ANSI color via the `\x` hex escape, and a
> `requires`/`ensures` contract the SMT solver actually proves. No
> new language features are introduced here; this chapter is about
> combining what you already know.

This is a walking tour of
[`examples/language/english/tic_tac_toe.vani`](https://github.com/enthusiasticgeek/vani-compiler/blob/main/examples/language/english/tic_tac_toe.vani),
a two-player, one-terminal tic-tac-toe game, built and shipped as
part of the example corpus. Every code block below is the *actual*
shipped source, not a simplified rewrite -- run the real file
alongside this chapter.

## Build & run

```bash
vanic run examples/language/english/tic_tac_toe.vani                          # LLVM backend, JIT via lli
vanic run examples/language/english/tic_tac_toe.vani --backend=c              # C backend, gcc
vanic build examples/language/english/tic_tac_toe.vani -o /tmp/ttt && /tmp/ttt   # native binary
```

X and O take turns typing a number 1-9. Empty cells display their
own position number as a placement guide -- exactly like the boxes
printed on a real tic-tac-toe sheet:

```
 1 | 2 | 3
---+---+---
 4 | 5 | 6
---+---+---
 7 | 8 | 9
```

Play a full game and X's marks render in red, O's in blue, and the
win/draw banner in the matching color (yellow for a draw). That
color is the newest piece of this file -- more on it below.

---

## The board: `Vec<i64>`, nothing fancier

```vani
fn main() -> i64 {
  let board: Vec<i64> = vec_fill(9, 0);
  ...
```

The whole board is one flat `Vec<i64>` of length 9: `0` = empty,
`1` = X, `2` = O. No struct, no 2D array -- cell `(row, col)` is
just index `row * 3 + col`. This is a deliberate simplicity choice:
a `Board` struct with a `cells: Vec<i64>` field would work too, but
would add a type this chapter doesn't need to teach.

Placing a move is a direct index-assignment on the owned local,
straight from [Intermediate 3 -- Affine ownership](03_affine.md)'s
territory:

```vani
board[idx as u64] = player;
```

No `mut ref`, no builtin call needed for this -- `board` is a
uniquely-owned local, so writing through an index it owns is just a
plain assignment. (Contrast this with `set(mut ref xs, i, v)`, which
exists for writing through a Vec you *don't* uniquely own -- a
struct field, or a Vec borrowed into a helper function. This game
never needs that form.)

---

## Printing the board, and the new `\x` color escape

```vani
fn ansi_red() -> Str { return "\x1b[31m"; }
fn ansi_blue() -> Str { return "\x1b[34m"; }
fn ansi_yellow() -> Str { return "\x1b[33m"; }
fn ansi_reset() -> Str { return "\x1b[0m"; }

fn player_label(player: i64) -> OwnedStr {
  if player == 1 {
    return ansi_red() + "X" + ansi_reset();
  }
  return ansi_blue() + "O" + ansi_reset();
}
```

`\x1b` is the ANSI **ESC** control byte -- the first byte of every
terminal color code. It's written with `\xHH`, a 2-digit hex escape
added to the lexer specifically to make this possible: string
literals previously only supported `\" \\ \n \t \r \0`, with no way
to emit a raw control byte short of pasting an invisible character
into the source file. See the
[Strings section](https://github.com/enthusiasticgeek/vani-compiler/blob/main/docs/language_manual.md#strings)
of the language manual for the full escape table -- `\x` is
restricted to `00`-`7f` (ASCII range) since `Str` is UTF-8 text with
no separate byte-string form; `\xff` is a compile error, not a
silently-reinterpreted byte.

<img class="manas" src="../images/mascot/manas_mascot_caution.png" title="const can't hold this"/>

**Why functions instead of `const`?** The natural first instinct is
`const ANSI_RED: Str = "\x1b[31m";`. It doesn't compile: v1's `const`
initializers must be a bare literal (`42`, `true`, ...) -- no string
literals, function calls, or concatenation. A tiny `fn` returning the
literal is the idiomatic substitute: still one named place to change
a color, just spelled `fn` instead of `const`.

Building each row is plain string concatenation (`+`, which produces
an `OwnedStr` from any mix of `Str`/`OwnedStr` operands), one `let`
per row:

```vani
fn print_board(board: ref Vec<i64>) -> i64 {
  let row1: OwnedStr = " " + symbol_for(board[0], 1) + " | " + symbol_for(board[1], 2) + " | " + symbol_for(board[2], 3);
  let row2: OwnedStr = " " + symbol_for(board[3], 4) + " | " + symbol_for(board[4], 5) + " | " + symbol_for(board[5], 6);
  let row3: OwnedStr = " " + symbol_for(board[6], 7) + " | " + symbol_for(board[7], 8) + " | " + symbol_for(board[8], 9);
  print row1;
  print "---+---+---";
  print row2;
  print "---+---+---";
  print row3;
  return 0;
}
```

<img class="manas" src="../images/mascot/manas_mascot_caution.png" title="print has a grammar quirk"/>

**A `print` quirk worth knowing:** `print "a" + "b";` is a *parse*
error ("expected ';'') -- `print`'s argument position doesn't accept
a `+` binary expression directly. Build the string into a `let`
first (as every `print` call above does), then print the binding.
This is exactly why `print_board`, the win message, and the draw
message all have a `let ... ; print ...;` shape instead of an inline
concatenation.

---

## Detecting a winner

```vani
fn winner_of(board: ref Vec<i64>) -> i64 {
  let lines: Vec<i64> = vec(
    0, 1, 2,  3, 4, 5,  6, 7, 8,
    0, 3, 6,  1, 4, 7,  2, 5, 8,
    0, 4, 8,  2, 4, 6
  );
  let li: u64 = 0;
  while li < 8 as u64 {
    let base: u64 = li * 3 as u64;
    let a: i64 = board[lines[base] as u64];
    let b: i64 = board[lines[base + 1 as u64] as u64];
    let c: i64 = board[lines[base + 2 as u64] as u64];
    if a != 0 {
      if a == b {
        if b == c {
          return a;
        }
      }
    }
    li = li + 1 as u64;
  }
  return 0;
}
```

`lines` is the flat list of all 8 winning triples -- 3 rows, 3
columns, 2 diagonals -- 3 board indices each, 24 numbers total. The
loop walks all 8 and checks whether the 3 cells they name are equal
and non-empty. Returning `0` (no winner) after the loop, `1` for X,
or `2` for O reuses the same encoding as the board cells themselves
-- one value space, no extra enum needed.

---

## Reading a move, safely

```vani
fn read_move(board: ref Vec<i64>, player: i64) -> i64
requires len(board) == 9 as u64;
ensures _return == 0 - 1 || (_return >= 0 && _return <= 8);
{
  while true {
    let prompt: OwnedStr = "Player " + player_label(player) + ", enter a position (1-9), or 'quit' to exit:";
    print prompt;
    flush_stdout();
    let input: OwnedStr = stdin_read_line();
    let trimmed: OwnedStr = str_trim(input);
    if trimmed == "" {
      return 0 - 1;
    }
    if trimmed == "quit" {
      return 0 - 1;
    }
    if let Option.Some(n) = parse_int(trimmed) {
      if n >= 1 {
        if n <= 9 {
          let idx: i64 = n - 1;
          if board[idx as u64] == 0 {
            return idx;
          } else {
            print "That cell is already taken -- try again.";
          }
        } else {
          print "Please enter a number from 1 to 9.";
        }
      } else {
        print "Please enter a number from 1 to 9.";
      }
    } else {
      print "That's not a number -- try again.";
    }
  }
  return 0 - 1;
}
```

This is where three tracks converge: `stdin_read_line()` +
`str_trim()` for raw input, `parse_int()` returning `Option<i64>`,
and `if let Option.Some(n) = ...` (from
[Intermediate 2b -- Match enhancements](02b_match_enhancements.md))
to unwrap it. A bad line (non-numeric, out of range, or an already-
occupied cell) re-prints an error and loops back to the prompt
instead of crashing.

<img class="manas" src="../images/mascot/manas_mascot_caution.png" title="EOF has no separate signal"/>

**A real gotcha, not a hypothetical one: blank input means quit.**
`trimmed == ""` returns immediately rather than re-prompting. Two
different situations produce an empty trimmed string here, and
vāṇी's runtime doesn't distinguish them:

1. A player presses Enter on a genuinely blank line.
2. `stdin_read_line()` hits real end-of-input on a closed or piped
   stdin -- there's no separate EOF signal, it just returns `""`,
   the same as case 1.

Treating both as "quit" is what keeps this loop from spinning
forever the moment stdin isn't an interactive terminal. This matters
beyond just being polite to a human at a keyboard:
`tools/leak_sweep.py` globs every `*.vani` file under `examples/**`
and runs each one with a timeout but *without* redirecting its own
process's stdin away -- so a naive "read forever until a valid
number shows up" loop here would have hung every leak-sweep run
until the timeout, on every CI run, forever. A demo this small can
still hang a build if its I/O loop doesn't account for who might be
running it.

### The contract: proven, not checked

The `requires`/`ensures` clause is the newest idea in this file if
you've just come from [Intermediate 12 -- SMT verification deep-dive](12_smt_deepdive.md)
(or its
[primer](12a_smt_primer.md), if you read that instead). It isn't a
runtime check -- it's a claim the SMT solver must **prove** holds on
*every* return path through this function, at compile time, before
the program is allowed to exist. Every arm that returns a real move
index is only reachable after `n >= 1` and `n <= 9` are both known
facts, so the solver derives `idx` (`= n - 1`) is in `[0, 8]` without
ever running the code.

You can watch the proof actually fail. Open the file and tighten the
postcondition:

```vani
ensures _return == 0 - 1 || (_return >= 0 && _return <= 5);
```

Re-run `vanic check` on it, and the solver rejects the function with
a concrete counterexample instead of silently shipping the bug:

```
error: function 'read_move' ensures clause does not hold at this return [counterexample: idx = 6, n = 7, ...]
```

`n = 7` is a completely ordinary, legal move (cell 7) that the
tightened clause simply doesn't cover -- exactly the kind of mistake
a human reviewer might miss and a fuzzer might take a while to find,
caught here in milliseconds by construction.

---

## Tying it together

```vani
fn main() -> i64 {
  let board: Vec<i64> = vec_fill(9, 0);
  let player: i64 = 1;
  let winner: i64 = 0;
  let moves_made: i64 = 0;

  print "vani tic-tac-toe -- two players, one terminal";
  print "";

  while true {
    print_board(ref board);
    print "";
    let idx: i64 = read_move(ref board, player);
    if idx < 0 {
      print "Goodbye!";
      return 0;
    }
    board[idx as u64] = player;
    moves_made = moves_made + 1;

    winner = winner_of(ref board);
    if winner != 0 {
      print "";
      print_board(ref board);
      let win_msg: OwnedStr = player_label(winner) + " wins!";
      print win_msg;
      break;
    }
    if moves_made >= 9 {
      print "";
      print_board(ref board);
      let draw_msg: OwnedStr = ansi_yellow() + "It's a draw." + ansi_reset();
      print draw_msg;
      break;
    }

    if player == 1 {
      player = 2;
    } else {
      player = 1;
    }
  }

  return 0;
}
```

Nothing new here -- a `while true` loop
([Beginner 5 -- while/for loops](../beginner/05_loops.md)) that
prints the board, reads one validated move, checks for a winner,
checks for a draw, and flips `player` between `1` and `2`. The
`idx < 0` check right after `read_move` is the quit path from the
gotcha above, handled at the one call site that needs to care about
it.

---

## Try it yourself

1. Add a "play again?" prompt after a win/draw instead of exiting,
   looping the whole game in an outer `while true`.
2. Track and print a running win count for X and O across rounds
   (you'll need a `Vec<i64>` or two `let mut`-style counters that
   survive the outer loop).
3. Color the empty-cell position numbers too (dim gray,
   `\x1b[2m...\x1b[0m`) so the placement guide is visually distinct
   from a placed mark.
4. *(Bigger)* Generalize `winner_of`'s `lines` table to an N×N board
   with a `k`-in-a-row win condition, and thread `N`/`k` through
   `print_board` and `read_move` as parameters.

---

## Summary

- A flat `Vec<i64>` was enough state for the whole game -- no struct
  needed for something this size.
- `stdin_read_line()` + `parse_int()` + `if let Option.Some(n) = ...`
  is the standard shape for "read one validated number from a
  human," and it's worth handling blank/EOF input explicitly rather
  than assuming a human is always on the other end of stdin.
- `requires`/`ensures` on `read_move` is proven at compile time, not
  checked at runtime -- tightening the postcondition to something
  false gets rejected with a concrete counterexample, not a passing
  build and a 3am bug report.
- The `\x` hex escape exists specifically because this file needed
  ANSI color and there was no way to write the ESC byte otherwise --
  a small, generic lexer feature that outlived the one example that
  motivated it.

---

**Previous**: [Sec.16 -- Packages with Kosh ->](16_packages.md)
**Next**: [Advanced track: Async / await -- intuition primer ->](../advanced/01a_async_primer.md)
