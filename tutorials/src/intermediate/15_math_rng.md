# Intermediate 15 -- Math, random numbers, and `clone`

> **Learning goal**: use the math, random-number, and `clone`
> builtins that are available in every vāṇī program without an
> import.

## Math builtins

vāṇī ships a large math library as compiler builtins. The most
commonly used ones:

### Basic arithmetic helpers

| Builtin | Returns |
|---|---|
| `abs(n: i64) -> i64` | absolute value |
| `pow(base: f64, exp: f64) -> f64` | floating-point power |
| `sqrt(x: f64) -> f64` | square root |
| `floor(x: f64) -> f64` | round down |
| `ceil(x: f64) -> f64` | round up |
| `f64_round(x: f64) -> f64` | round to nearest |
| `f64_trunc_to_i64(x: f64) -> i64` | truncate to integer |
| `i64_abs_diff(a: i64, b: i64) -> i64` | `|a - b|` without overflow |
| `i64_min(a, b) -> i64` / `i64_max(a, b) -> i64` | integer min/max |
| `f64_min(a, b) -> f64` / `f64_max(a, b) -> f64` | float min/max |
| `i64_clamp(v, lo, hi) -> i64` | clamp to range |

### Trigonometry (radians)

`sin`, `cos`, `tan`, `asin`, `acos`, `atan`, `atan2`, `sinh`,
`cosh`, `tanh`. Degree variants: `f64_to_radians`, `f64_to_degrees`,
`f64_atan2_deg`.

### Constants and predicates

| Builtin | Value |
|---|---|
| `f64_pi()` | pi |
| `f64_e()` | e (Euler's number) |
| `f64_inf()` | positive infinity |
| `f64_nan()` | NaN |
| `f64_is_nan(x)` | NaN test |
| `f64_is_inf(x)` | infinity test |
| `f64_is_finite(x)` | finite test |

### Integer number theory

`i64_gcd`, `i64_lcm`, `i64_pow`, `i64_isqrt`, `i64_is_prime`,
`i64_factorial`, `i64_fibonacci`, `i64_binomial`, `i64_next_prime`,
`i64_prev_prime`, `i64_mod_inverse`, `i64_totient`.

### Quick example

```vani
intent "Intermediate 15 -- math builtins.";

fn main() -> i64 {
  print "sqrt(2):", sqrt(2.0);
  print "pi:", f64_pi();
  print "sin(pi/6):", sin(f64_pi() / 6.0);   // 0.5
  print "floor(3.7):", floor(3.7);            // 3.0
  print "gcd(48, 18):", i64_gcd(48, 18);      // 6
  print "is_prime(17):", i64_is_prime(17);    // true
  print "fibonacci(10):", i64_fibonacci(10);  // 55
  return 0;
}
```

## Random numbers

Seed the RNG once with `seed_rng(seed: i64)`, then draw values:

| Builtin | Returns |
|---|---|
| `seed_rng(seed: i64)` | seeds the global RNG |
| `rand_i64() -> i64` | any i64 |
| `rand_in_range(lo: i64, hi: i64) -> i64` | i64 in `[lo, hi)` |
| `rand_f64() -> f64` | uniform float in `[0.0, 1.0)` |
| `rand_in_range_f64(lo: f64, hi: f64) -> f64` | float in `[lo, hi)` |
| `rand_bool() -> bool` | 50/50 coin flip |
| `rand_normal(mean: f64, std: f64) -> f64` | Gaussian sample |

```vani
intent "Intermediate 15 -- RNG.";

fn main() -> i64 {
  seed_rng(42);

  let r: i64 = rand_in_range(1, 7);   // simulated d6 roll
  print "d6 roll:", r;

  let f: f64 = rand_f64();
  print "uniform [0,1):", f;

  let b: bool = rand_bool();
  print "coin flip:", b;

  return 0;
}
```

Without `seed_rng`, the RNG is seeded to 0 and produces
deterministic values -- useful for tests. Call `seed_rng` with a
time-based or hardware seed for unpredictable output.

## `clone`

In vāṇī, affine ownership means you can't use a value twice after
moving it. For `Vec<T>`, `HashMap<K,V>`, `HashSet<T>`, and
`OwnedStr`, you can make an explicit deep copy with `clone`:

```vani
let original: Vec<i64> = vec(1, 2, 3);
let copy: Vec<i64> = clone(original);   // deep copy; both are valid
push(mut ref copy, 99);
print "original len:", len(original);   // still 3
print "copy len:", len(copy);           // 4
```

- `clone(x)` copies the heap data; the caller owns the copy.
- `clone_at(v, i)` clones only the element at index `i` in a
  `Vec<T>` -- useful for extracting an `OwnedStr` field without
  consuming the whole vector.
- Only heap-allocated types need `clone`; `i64`, `f64`, `bool`
  copy automatically (they're `Copy`).

## Challenge

Write a Monte Carlo pi estimator: draw N pairs of random floats
in `[0, 1)`, count how many fall inside the unit circle
(`x*x + y*y < 1.0`), and print `4 * count / N` as an
approximation of pi. Use `rand_f64()` and verify the estimate
converges as N grows.

---

**Previous**: [Sec.14 -- `HashMap<K,V>` and `HashSet<T>` ->](14_collections.md)
**Next**: [Sec.15a -- Math library deep-dive ->](15a_math_deep.md)
