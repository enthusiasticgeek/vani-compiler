# Intermediate 15a — Math library deep-dive

> **Learning goal**: use the full math library — special functions,
> ML activations, bit manipulation, and extended number theory —
> that ships as compiler builtins in every vāṇī program.

> **Prerequisites**: [Intermediate 15 — Math, random numbers, and clone](15_math_rng.md).

---

## Logarithms and exponentials

vāṇी ships all standard transcendental functions:

```vani
intent "logs and exps";

fn main() -> i64 {
  // Standard logs (base-2, base-10 — unqualified names)
  let l2:  f64 = log2(8.0);          // 3.0
  let l10: f64 = log10(1000.0);      // 3.0

  // f64_ qualified variants
  let lp:  f64 = f64_log1p(1.0);     // ln(2) ≈ 0.693 (numerically stable near 0)
  let lb:  f64 = f64_log_b(8.0, 2.0); // arbitrary base: log₂(8) = 3.0
  let e1:  f64 = f64_expm1(1.0);     // e^1 - 1 (numerically stable near 0)
  let e2:  f64 = f64_exp2(10.0);     // 2^10 = 1024.0
  let e10: f64 = f64_exp10(3.0);     // 10^3 = 1000.0

  print "log2(8):", l2;
  print "log10(1000):", l10;
  print "log_b(8, 2):", lb;
  print "expm1(1):", e1;
  return 0;
}
```

| Builtin | Description |
|---------|-------------|
| `log2(x: f64) -> f64` | Base-2 logarithm |
| `log10(x: f64) -> f64` | Base-10 logarithm |
| `f64_log1p(x: f64) -> f64` | `ln(1+x)` — numerically stable for small x |
| `f64_log_b(x: f64, base: f64) -> f64` | `log(x)` in arbitrary base |
| `f64_expm1(x: f64) -> f64` | `e^x - 1` — numerically stable for small x |
| `f64_exp2(x: f64) -> f64` | `2^x` |
| `f64_exp10(x: f64) -> f64` | `10^x` |

---

## Special functions

These cover the full C99 `<math.h>` special-function set:

```vani
intent "special functions";

fn main() -> i64 {
  let h: f64 = f64_hypot(3.0, 4.0);     // 5.0 — Euclidean distance, no overflow
  let c: f64 = f64_cbrt(27.0);          // 3.0 — cube root
  let er: f64 = f64_erf(1.0);           // ≈ 0.843 — Gauss error function
  let ec: f64 = f64_erfc(1.0);          // ≈ 0.157 — complementary error function
  let g:  f64 = f64_tgamma(5.0);        // 24.0 — Γ(5) = 4!
  let lg: f64 = f64_lgamma(10.0);       // ln(Γ(10)) ≈ 12.802

  print "hypot(3,4):", h;
  print "cbrt(27):", c;
  print "erf(1):", er;
  print "tgamma(5):", g;
  return 0;
}
```

| Builtin | Description |
|---------|-------------|
| `f64_hypot(a: f64, b: f64) -> f64` | `√(a²+b²)` without intermediate overflow |
| `f64_cbrt(x: f64) -> f64` | Cube root |
| `f64_erf(x: f64) -> f64` | Gauss error function |
| `f64_erfc(x: f64) -> f64` | Complementary error function (`1 - erf(x)`) |
| `f64_tgamma(x: f64) -> f64` | Gamma function Γ(x) |
| `f64_lgamma(x: f64) -> f64` | Natural log of Γ(x) |

---

## ML activation functions

These are available as single-call builtins for scalar inputs:

```vani
intent "ML activations";

fn main() -> i64 {
  let x: f64 = 1.5;

  let r:  f64 = f64_relu(x);                 // max(0, x) = 1.5
  let lr: f64 = f64_leaky_relu(0.0 - 1.5, 0.01); // 0.01 * (-1.5) = -0.015
  let sp: f64 = f64_softplus(x);             // ln(1 + e^x) ≈ 1.701
  let sw: f64 = f64_swish(x);               // x * σ(x) ≈ 1.253
  let sg: f64 = f64_sigmoid(0.0);           // 1/(1+e^0) = 0.5
  let ss: f64 = f64_softsign(x);            // x/(1+|x|) = 1.5/2.5 = 0.6
  let lo: f64 = f64_logit(0.5);             // ln(p/(1-p)) = 0.0

  print "relu(1.5):", r;
  print "sigmoid(0):", sg;
  print "swish(1.5):", sw;
  print "logit(0.5):", lo;
  return 0;
}
```

| Builtin | Formula | Use case |
|---------|---------|----------|
| `f64_relu(x: f64) -> f64` | `max(0, x)` | Standard hidden-layer activation |
| `f64_leaky_relu(x: f64, alpha: f64) -> f64` | `x ≥ 0 → x; else alpha*x` | Avoids dead neurons |
| `f64_softplus(x: f64) -> f64` | `ln(1 + e^x)` | Smooth relu approximation |
| `f64_swish(x: f64) -> f64` | `x * σ(x)` | Self-gated; often outperforms ReLU |
| `f64_sigmoid(x: f64) -> f64` | `1 / (1 + e^(-x))` | Binary classification output |
| `f64_softsign(x: f64) -> f64` | `x / (1 + |x|)` | Bounded alternative to tanh |
| `f64_logit(x: f64) -> f64` | `ln(x / (1-x))` | Inverse sigmoid; probability → log-odds |

For vector inputs, apply with `map(ref xs, f64_relu)` (closures chapter).

---

## Extended number theory

These extend the integer math covered in [§15](15_math_rng.md):

```vani
intent "extended number theory";

fn main() -> i64 {
  // Modular arithmetic
  let pm: i64 = i64_mod_pos(0 - 7, 3);        // 2 (always non-negative mod)
  let mi: i64 = i64_mod_inverse(3, 7);         // 5 (3 * 5 ≡ 1 mod 7)

  // Roots
  let cr: i64 = i64_cube_root(27);             // 3
  let rd: i64 = i64_radical(60);               // 30 (product of distinct prime factors)

  // Euler's totient
  let phi: i64 = i64_totient(12);              // 4 (numbers < 12 coprime to 12)

  // Float extras
  let pi: f64 = f64_pow_int(2.0, 10);          // 1024.0 — integer exponent (faster)
  let rm: f64 = f64_round_to_multiple(3.7, 0.5); // 3.5
  let qr: f64 = f64_quadratic_root(1.0, 0.0 - 3.0, 2.0); // smaller root of x²-3x+2=0 → 1.0

  print "mod_pos(-7, 3):", pm;
  print "mod_inverse(3, 7):", mi;
  print "totient(12):", phi;
  print "quadratic_root(1,-3,2):", qr;
  return 0;
}
```

| Builtin | Description |
|---------|-------------|
| `i64_mod_pos(a: i64, m: i64) -> i64` | `a mod m`, always non-negative (unlike `%` on negatives) |
| `i64_mod_inverse(a: i64, m: i64) -> i64` | Modular inverse: `x` s.t. `a*x ≡ 1 (mod m)` |
| `i64_cube_root(n: i64) -> i64` | Integer cube root |
| `i64_radical(n: i64) -> i64` | Product of distinct prime factors of n |
| `i64_totient(n: i64) -> i64` | Euler's totient φ(n) |
| `i64_parity(n: i64) -> i64` | 1 if odd number of set bits, 0 otherwise |
| `f64_pow_int(base: f64, exp: i64) -> f64` | Faster than `pow` when exponent is integer |
| `f64_round_to_multiple(x: f64, m: f64) -> f64` | Round x to the nearest multiple of m |
| `f64_quadratic_root(a: f64, b: f64, c: f64) -> f64` | Smaller root of ax²+bx+c=0 |

---

## Bit manipulation

All bit operations work on `i64` values:

```vani
intent "bit manipulation";

fn main() -> i64 {
  let x: i64 = 5;    // binary: 0101

  // Single-bit operations
  let s: i64  = i64_set_bit(x, 1);    // set bit 1  → 0111 = 7
  let c: i64  = i64_clear_bit(7, 1);  // clear bit 1 → 0101 = 5
  let t: i64  = i64_toggle_bit(x, 1); // toggle bit 1 → 0111 = 7
  let b: bool = i64_test_bit(x, 0);   // test bit 0 → true (bit is set)

  // Bit counting and analysis
  let p: i64 = i64_parity(7);             // 1 (three 1-bits → odd)
  let l: i64 = i64_leading_zeros(1);      // 63 (only bit 0 set)
  let tr: i64 = i64_trailing_zeros(8);    // 3 (1000 in binary)
  let rv: i64 = i64_reverse_bits(1);      // 1 → high bit becomes low bit

  // Rotation and byte swap
  let rl: i64 = i64_rotate_left(1, 4);   // 0001 → 0001_0000 = 16
  let rr: i64 = i64_rotate_right(16, 4); // back to 1
  let bs: i64 = i64_bswap(1);            // byte-reverse (big↔little endian)

  print "set_bit(5, 1):", s;
  print "test_bit(5, 0):", b;
  print "leading_zeros(1):", l;
  print "rotate_left(1, 4):", rl;
  return 0;
}
```

| Builtin | Description |
|---------|-------------|
| `i64_set_bit(x: i64, n: i64) -> i64` | Set bit `n` of `x` |
| `i64_clear_bit(x: i64, n: i64) -> i64` | Clear bit `n` of `x` |
| `i64_toggle_bit(x: i64, n: i64) -> i64` | Flip bit `n` of `x` |
| `i64_test_bit(x: i64, n: i64) -> bool` | True if bit `n` is set |
| `i64_parity(x: i64) -> i64` | 1 if odd popcount, 0 if even |
| `i64_leading_zeros(x: i64) -> i64` | Count leading 0-bits (clz) |
| `i64_trailing_zeros(x: i64) -> i64` | Count trailing 0-bits (ctz) |
| `i64_reverse_bits(x: i64) -> i64` | Bit-reverse (via `@llvm.bitreverse.i64`) |
| `i64_rotate_left(x: i64, n: i64) -> i64` | Rotate left by n positions |
| `i64_rotate_right(x: i64, n: i64) -> i64` | Rotate right by n positions |
| `i64_bswap(x: i64) -> i64` | Byte-swap (big↔little endian) |

**When to use bit ops**: hash functions, compact flags, protocol parsing,
SIMD-style packing, embedded register manipulation alongside MMIO.

---

## Putting it all together: fixed-point sigmoid approximation

```vani
intent "fixed-point sigmoid";

fn main() -> i64 {
  // Approximate sigmoid via fixed-point bit tricks for embedded targets.
  // For full-precision, use f64_sigmoid.
  let x: f64 = 2.0;
  let sg: f64 = f64_sigmoid(x);
  print "sigmoid(2.0):", sg;          // ≈ 0.880

  // Count set bits as a proxy for "weight" in a bitmask classifier
  let mask: i64 = 0b10110111;         // 6 bits set
  // i64_parity tells even/odd; use leading/trailing zeros for priority
  let lo: i64 = i64_trailing_zeros(mask);  // lowest set bit position = 0
  let hi: i64 = 63 - i64_leading_zeros(mask); // highest set bit = 7
  print "lowest set bit:", lo;
  print "highest set bit:", hi;

  return 0;
}
```

---

**Next**: [§15b — Vec statistics and combinators →](15b_vec_stats.md)
