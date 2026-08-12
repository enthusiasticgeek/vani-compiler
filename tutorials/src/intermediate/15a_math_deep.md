# Intermediate 15a -- Math library deep-dive

> **Learning goal**: use the full math library -- special functions,
> ML activations, bit manipulation, and extended number theory --
> that ships as compiler builtins in every vāṇī program.

> **Prerequisites**: [Intermediate 15 -- Math, random numbers, and clone](15_math_rng.md).

---

## Logarithms and exponentials

vāṇी ships all standard transcendental functions:

```vani
intent "logs and exps";

fn main() -> i64 {
  // Standard logs (base-2, base-10 -- unqualified names)
  let l2:  f64 = log2(8.0);          // 3.0
  let l10: f64 = log10(1000.0);      // 3.0

  // f64_ qualified variants
  let lp:  f64 = f64_log1p(1.0);     // ln(2) ~= 0.693 (numerically stable near 0)
  let lb:  f64 = f64_log_b(8.0, 2.0); // arbitrary base: log_2(8) = 3.0
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
| `f64_log1p(x: f64) -> f64` | `ln(1+x)` -- numerically stable for small x |
| `f64_log_b(x: f64, base: f64) -> f64` | `log(x)` in arbitrary base |
| `f64_expm1(x: f64) -> f64` | `e^x - 1` -- numerically stable for small x |
| `f64_exp2(x: f64) -> f64` | `2^x` |
| `f64_exp10(x: f64) -> f64` | `10^x` |

---

## Special functions

These cover the full C99 `<math.h>` special-function set:

```vani
intent "special functions";

fn main() -> i64 {
  let h: f64 = f64_hypot(3.0, 4.0);     // 5.0 -- Euclidean distance, no overflow
  let c: f64 = f64_cbrt(27.0);          // 3.0 -- cube root
  let er: f64 = f64_erf(1.0);           // ~= 0.843 -- Gauss error function
  let ec: f64 = f64_erfc(1.0);          // ~= 0.157 -- complementary error function
  let g:  f64 = f64_tgamma(5.0);        // 24.0 -- Gamma(5) = 4!
  let lg: f64 = f64_lgamma(10.0);       // ln(Gamma(10)) ~= 12.802

  print "hypot(3,4):", h;
  print "cbrt(27):", c;
  print "erf(1):", er;
  print "tgamma(5):", g;
  return 0;
}
```

| Builtin | Description |
|---------|-------------|
| `f64_hypot(a: f64, b: f64) -> f64` | `√(a^2+b^2)` without intermediate overflow |
| `f64_cbrt(x: f64) -> f64` | Cube root |
| `f64_erf(x: f64) -> f64` | Gauss error function |
| `f64_erfc(x: f64) -> f64` | Complementary error function (`1 - erf(x)`) |
| `f64_tgamma(x: f64) -> f64` | Gamma function Gamma(x) |
| `f64_lgamma(x: f64) -> f64` | Natural log of Gamma(x) |
| `f64_inv_sqrt(x: f64) -> f64` | `1/√x` |
| `f64_fma(a: f64, b: f64, c: f64) -> f64` | `a*b + c`, rounded once (no intermediate rounding error) |
| `f64_remainder(a: f64, b: f64) -> f64` | IEEE remainder of `a/b` (can be negative, unlike `f64_mod_floor` below) |
| `f64_l1_norm(a: f64, b: f64) -> f64` | Manhattan distance: `\|a\| + \|b\|` |
| `f64_chebyshev(a: f64, b: f64) -> f64` | Chebyshev distance: `max(\|a\|, \|b\|)` |
| `f64_quadratic_mean(a: f64, b: f64) -> f64` | root-mean-square of two values: `√((a^2+b^2)/2)` |
| `f64_normal_pdf(x: f64, mean: f64, stddev: f64) -> f64` | standard normal probability density |
| `f64_normal_cdf(x: f64, mean: f64, stddev: f64) -> f64` | standard normal cumulative distribution (probability `X <= x`) |

```vani
intent "more special functions";

fn main() -> i64 {
  print "fma(2,3,1):", f64_fma(2.0, 3.0, 1.0);                   // 7
  print "quadratic_mean(3,4):", f64_quadratic_mean(3.0, 4.0);    // 3.53553
  print "normal_cdf(0,0,1):", f64_normal_cdf(0.0, 0.0, 1.0);     // 0.5 (at the mean)
  print "l1_norm(3,-4):", f64_l1_norm(3.0, 0.0 - 4.0);           // 7
  print "inv_sqrt(4):", f64_inv_sqrt(4.0);                       // 0.5
  return 0;
}
```

---

## ML activation functions

These are available as single-call builtins for scalar inputs:

```vani
intent "ML activations";

fn main() -> i64 {
  let x: f64 = 1.5;

  let r:  f64 = f64_relu(x);                 // max(0, x) = 1.5
  let lr: f64 = f64_leaky_relu(0.0 - 1.5, 0.01); // 0.01 * (-1.5) = -0.015
  let sp: f64 = f64_softplus(x);             // ln(1 + e^x) ~= 1.701
  let sw: f64 = f64_swish(x);               // x * sigma(x) ~= 1.226
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
| `f64_leaky_relu(x: f64, alpha: f64) -> f64` | `x >= 0 -> x; else alpha*x` | Avoids dead neurons |
| `f64_softplus(x: f64) -> f64` | `ln(1 + e^x)` | Smooth relu approximation |
| `f64_swish(x: f64) -> f64` | `x * sigma(x)` | Self-gated; often outperforms ReLU |
| `f64_sigmoid(x: f64) -> f64` | `1 / (1 + e^(-x))` | Binary classification output |
| `f64_softsign(x: f64) -> f64` | `x / (1 + |x|)` | Bounded alternative to tanh |
| `f64_logit(x: f64) -> f64` | `ln(x / (1-x))` | Inverse sigmoid; probability -> log-odds |

For vector inputs, apply with `map(ref xs, f64_relu)` (closures chapter).

---

## Clamping, interpolation, and remapping

The everyday "keep this value in range" / "blend between two values"
/ "convert a value from one range to another" toolkit -- the same
operations most game/graphics/DSP code reaches for constantly:

```vani
intent "clamping, interpolation, remapping";

fn main() -> i64 {
  let cl:  f64 = f64_clamp(1.5, 0.0, 1.0);        // 1.0 -- outside [0,1], clamped
  let c01: f64 = f64_clamp01(1.5);                // 1.0 -- clamp(x, 0, 1) shorthand
  let lp:  f64 = f64_lerp(0.0, 10.0, 0.5);        // 5.0 -- halfway between 0 and 10
  let ilp: f64 = f64_inv_lerp(0.0, 10.0, 5.0);    // 0.5 -- inverse: what t gives 5?
  let lpc: f64 = f64_lerp_clamp(0.0, 10.0, 1.5);  // 10.0 -- t=1.5 clamped to 1.0 first
  let rm:  f64 = f64_remap(5.0, 0.0, 10.0, 0.0, 100.0); // 50.0 -- rescale between ranges
  let sm:  f64 = f64_smoothstep(0.0, 1.0, 0.5);   // 0.5 -- S-curve interpolation
  let im3: i64 = i64_min_3(5, 2, 8);              // 2
  let ix3: i64 = i64_max_3(5, 2, 8);              // 8

  print "clamp(1.5, 0, 1):", cl;
  print "lerp(0, 10, 0.5):", lp;
  print "remap(5, [0,10] -> [0,100]):", rm;
  print "smoothstep(0.5):", sm;
  return 0;
}
```

| Builtin | Description |
|---------|-------------|
| `f64_clamp(x, lo, hi: f64) -> f64` | restrict `x` to `[lo, hi]` |
| `f64_clamp01(x: f64) -> f64` | shorthand for `f64_clamp(x, 0.0, 1.0)` |
| `f64_lerp(a, b, t: f64) -> f64` | linear interpolation: `a + t*(b-a)` |
| `f64_inv_lerp(a, b, x: f64) -> f64` | inverse of `f64_lerp` -- what `t` produces `x`? |
| `f64_lerp_clamp(a, b, t: f64) -> f64` | `f64_lerp`, but `t` is clamped to `[0,1]` first |
| `f64_remap(x, in_lo, in_hi, out_lo, out_hi: f64) -> f64` | rescale `x` from one range to another |
| `f64_step(edge, x: f64) -> f64` | `0.0` if `x < edge`, else `1.0` |
| `f64_smoothstep(edge0, edge1, x: f64) -> f64` | smooth S-curve between the two edges |
| `f64_smoothstep5` | same as `f64_smoothstep` but a steeper (quintic) curve |
| `f64_inv_smoothstep(x: f64) -> f64` | approximate inverse of `f64_smoothstep(0, 1, x)` |
| `i64_min_3` / `i64_max_3` (and `f64_` variants) | min/max of three values in one call |

---

## Extended number theory

These extend the integer math covered in [Sec.15](15_math_rng.md):

```vani
intent "extended number theory";

fn main() -> i64 {
  // Modular arithmetic
  let pm: i64 = i64_mod_pos(0 - 7, 3);        // 2 (always non-negative mod)
  let mi: i64 = i64_mod_inverse(3, 7);         // 5 (3 * 5 === 1 mod 7)

  // Roots
  let cr: i64 = i64_cube_root(27);             // 3
  let rd: i64 = i64_radical(60);               // 30 (product of distinct prime factors)

  // Euler's totient
  let phi: i64 = i64_totient(12);              // 4 (numbers < 12 coprime to 12)

  // Float extras
  let pi: f64 = f64_pow_int(2.0, 10);          // 1024.0 -- integer exponent (faster)
  let rm: f64 = f64_round_to_multiple(3.7, 0.5); // 3.5
  let qr: f64 = f64_quadratic_root(1.0, 0.0 - 3.0, 2.0); // larger root of x^2-3x+2=0 -> 2.0

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
| `i64_mod_inverse(a: i64, m: i64) -> i64` | Modular inverse: `x` s.t. `a*x === 1 (mod m)` |
| `i64_cube_root(n: i64) -> i64` | Integer cube root |
| `i64_radical(n: i64) -> i64` | Product of distinct prime factors of n |
| `i64_totient(n: i64) -> i64` | Euler's totient phi(n) |
| `i64_parity(n: i64) -> i64` | 1 if odd number of set bits, 0 otherwise |
| `f64_pow_int(base: f64, exp: i64) -> f64` | Faster than `pow` when exponent is integer |
| `f64_round_to_multiple(x: f64, m: f64) -> f64` | Round x to the nearest multiple of m |
| `f64_quadratic_root(a: f64, b: f64, c: f64) -> f64` | Larger root of ax^2+bx+c=0 (`(-b + sqrt(b^2-4ac)) / 2a`) |
| `i64_div_floor(a, b: i64) -> i64` | Floor division (rounds toward `-∞`, unlike `/`'s truncation) |
| `i64_mod_floor(a, b: i64) -> i64` | The remainder that matches `i64_div_floor` |
| `f64_mod_floor(a, b: f64) -> f64` | Floating-point floor-mod (always same sign as `b`) |
| `i64_div_ceil(a, b: i64) -> i64` | Division rounded toward `+∞` |
| `i64_div_round(a, b: i64) -> i64` | Division rounded to nearest (ties away from zero) |
| `i64_log2_floor(n: i64) -> i64` | `⌊log2(n)⌋` -- position of the highest set bit |
| `i64_log2_ceil(n: i64) -> i64` | `⌈log2(n)⌉` -- bits needed to represent `n` |
| `i64_log10_floor(n: i64) -> i64` | `⌊log10(n)⌋` -- one less than the decimal digit count |
| `i64_log10_ceil(n: i64) -> i64` | `⌈log10(n)⌉` |
| `f64_trunc(x: f64) -> f64` | Truncate toward zero (drop the fractional part) |
| `f64_frac(x: f64) -> f64` | Just the fractional part (`x - trunc(x)`) |
| `i64_pow_mod(base, exp, m: i64) -> i64` | `base^exp mod m`, without overflowing for large exponents |
| `i64_perm(n, k: i64) -> i64` | Permutations: `n! / (n-k)!` |

```vani
intent "floor/ceil division and logs";

fn main() -> i64 {
  print "div_floor(-7, 2):", i64_div_floor(0 - 7, 2);   // -4
  print "div_ceil(7, 2):", i64_div_ceil(7, 2);          // 4
  print "log2_floor(9):", i64_log2_floor(9);            // 3
  print "log2_ceil(9):", i64_log2_ceil(9);               // 4
  print "pow_mod(2, 10, 1000):", i64_pow_mod(2, 10, 1000);   // 24
  print "perm(5, 2):", i64_perm(5, 2);                   // 20 (5*4)
  return 0;
}
```

---

## Overflow-safe integer arithmetic

`i64` arithmetic normally traps on overflow (the L4 runtime guard --
see [Intermediate 10b](10b_runtime_errors_primer.md)). These
builtins give you an alternative for the cases where saturating or
wrapping is the actually-intended behavior, not a bug:

```vani
intent "overflow-safe arithmetic";

fn main() -> i64 {
  // Saturating: clamps to the type's min/max instead of trapping.
  let sa: i64 = i64_saturating_add(i64_max_value(), 1);   // stays at i64::MAX
  let ss: i64 = i64_saturating_sub(i64_min_value(), 1);   // stays at i64::MIN
  let sm: i64 = i64_saturating_mul(i64_max_value(), 2);   // stays at i64::MAX

  // Wrap into a range, cyclic-index / angle-normalization style.
  let wr: i64 = i64_wrap(9, 0, 5);              // 4 -- wraps 9 into [0, 5)
  let fwr: f64 = f64_wrap(370.0, 0.0, 360.0);   // 10.0 -- angle normalized to [0, 360)

  let av: i64 = i64_avg(4, 8);   // 6 -- (a+b)/2 without the intermediate a+b overflowing

  print "saturating_add at MAX:", sa == i64_max_value();
  print "wrap(9, [0,5)):", wr;
  print "wrap(370deg, [0,360)):", fwr;
  print "avg(4, 8):", av;
  return 0;
}
```

| Builtin | Description |
|---------|-------------|
| `i64_saturating_add(a, b: i64) -> i64` | `a+b`, clamped to `i64::MIN`/`MAX` instead of trapping |
| `i64_saturating_sub(a, b: i64) -> i64` | `a-b`, clamped |
| `i64_saturating_mul(a, b: i64) -> i64` | `a*b`, clamped |
| `i64_wrap(x, lo, hi: i64) -> i64` | wrap `x` into `[lo, hi)` -- for cyclic indices |
| `f64_wrap(x, lo, hi: f64) -> f64` | same, for floats -- e.g. wrapping an angle into `[0, 360)` |
| `i64_avg(a, b: i64) -> i64` | average of two values without `a+b` overflowing first |

---

## Number properties and limits

```vani
intent "number properties";

fn main() -> i64 {
  print "signum(-5):", i64_signum(0 - 5);              // -1
  print "is_power_of_2(16):", i64_is_power_of_2(16);   // true
  print "next_power_of_2(17):", i64_next_power_of_2(17); // 32
  print "is_perfect_square(16):", i64_is_perfect_square(16); // true
  print "count_digits(12345):", i64_count_digits(12345);     // 5
  print "divisor_count(12):", i64_divisor_count(12);         // 6
  print "divisor_sum(12):", i64_divisor_sum(12);             // 28 (1+2+3+4+6+12)
  print "i64 range:", i64_min_value(), i64_max_value();
  return 0;
}
```

| Builtin | Description |
|---------|-------------|
| `i64_signum(x: i64) -> i64` | `-1`, `0`, or `1` |
| `f64_signum(x: f64) -> f64` | same, for floats |
| `i64_is_power_of_2(x: i64) -> bool` | true if `x` is a power of two |
| `i64_next_power_of_2(x: i64) -> i64` | smallest power of two `>= x` |
| `i64_is_perfect_square(x: i64) -> bool` | true if `x` is a perfect square |
| `i64_count_digits(x: i64) -> i64` | decimal digit count |
| `i64_divisor_count(n: i64) -> i64` | how many divisors `n` has |
| `i64_divisor_sum(n: i64) -> i64` | sum of `n`'s divisors (including 1 and n) |
| `i64_min_value()` / `i64_max_value()` | the `i64` range's endpoints |
| `f64_max_finite()` | the largest finite `f64` |
| `f64_epsilon()` | the smallest `f64` such that `1.0 + eps != 1.0` |
| `f64_min_positive()` / `f64_min_subnormal()` | smallest positive normal / subnormal `f64` |

---

## Float bit-level inspection

For when you need to reason about a float's actual IEEE-754 bit
pattern rather than its numeric value -- serialization, hashing,
ULP-based comparisons:

```vani
intent "float bit-level inspection";

fn main() -> i64 {
  let bits: i64 = f64_to_bits(1.0);
  let back: f64 = f64_from_bits(bits);           // 1.0 -- round-trips exactly
  print "is_normal(1.0):", f64_is_normal(1.0);         // true
  print "is_subnormal(1.0):", f64_is_subnormal(1.0);   // false
  print "sign_bit(-1.0):", f64_sign_bit(0.0 - 1.0);    // true
  print "copysign(3, -1):", f64_copysign(3.0, 0.0 - 1.0);   // -3.0
  print "next_up(1.0) != 1.0:", f64_next_up(1.0) != 1.0;    // true
  print "round-trip:", back;
  return 0;
}
```

| Builtin | Description |
|---------|-------------|
| `f64_to_bits(x: f64) -> i64` | the raw IEEE-754 bit pattern, reinterpreted as `i64` |
| `f64_from_bits(bits: i64) -> f64` | inverse of `f64_to_bits` |
| `f64_is_normal(x: f64) -> bool` | true unless `x` is zero, subnormal, infinite, or NaN |
| `f64_is_subnormal(x: f64) -> bool` | true if `x` is a subnormal ("denormal") float |
| `f64_sign_bit(x: f64) -> bool` | the sign bit, even for `-0.0` (which `x < 0.0` misses) |
| `f64_copysign(magnitude, sign: f64) -> f64` | `magnitude` with `sign`'s sign bit |
| `f64_next_up(x: f64) -> f64` / `f64_next_down(x: f64) -> f64` | the next representable `f64` above/below `x` (one ULP) |

---

## Extra trigonometry: reciprocal and degree-based

The three reciprocal trig functions, plus degree-unit variants of
the functions already covered in [Sec.15](15_math_rng.md) (which are
all radian-based):

```vani
intent "reciprocal and degree-based trig";

fn main() -> i64 {
  print "sec(0):", f64_sec(0.0);              // 1.0 (1/cos)
  print "atan2_deg(1,1):", f64_atan2_deg(1.0, 1.0);   // 45.0
  print "asin_deg(1):", f64_asin_deg(1.0);            // 90.0
  print "sinc(0):", f64_sinc(0.0);                     // 1.0 (sin(x)/x, defined at 0)
  return 0;
}
```

| Builtin | Description |
|---------|-------------|
| `f64_sec(x)` / `f64_csc(x)` / `f64_cot(x)` | `1/cos(x)`, `1/sin(x)`, `1/tan(x)` (radians) |
| `f64_atan_deg(x)` / `f64_atan2_deg(y, x)` | `atan`/`atan2`, result in degrees |
| `f64_asin_deg(x)` / `f64_acos_deg(x)` | `asin`/`acos`, result in degrees |
| `f64_sec_deg(x)` / `f64_csc_deg(x)` / `f64_cot_deg(x)` | reciprocal trig, input in degrees |
| `f64_sinc(x: f64) -> f64` | `sin(x)/x`, defined as `1.0` at `x = 0` (no division-by-zero) |

---

## RGB color packing

Pack three 0-255 color channels into one `i64` (and back), plus a
standard-weights grayscale conversion:

```vani
intent "RGB packing";

fn main() -> i64 {
  let rgb: i64 = i64_pack_rgb(255, 128, 0);   // orange
  print "packed:", rgb;
  print "r:", i64_unpack_rgb_r(rgb);   // 255
  print "g:", i64_unpack_rgb_g(rgb);   // 128
  print "b:", i64_unpack_rgb_b(rgb);   // 0
  print "grayscale(white):", f64_rgb_to_grayscale(255.0, 255.0, 255.0);   // 255
  return 0;
}
```

| Builtin | Description |
|---------|-------------|
| `i64_pack_rgb(r, g, b: i64) -> i64` | pack three 0-255 channels into one `i64` |
| `i64_unpack_rgb_r` / `_g` / `_b` `(rgb: i64) -> i64` | extract one channel back out |
| `f64_rgb_to_grayscale(r, g, b: f64) -> f64` | perceptual grayscale (standard luma weights) |

---

## Hashing values for use as keys

Deterministic (not randomized/keyed) hashing for building composite
`HashMap`/`HashSet` keys out of several fields, plus `siphash_*` for
when you need a keyed hash (e.g. resisting hash-flooding on
untrusted input). All return `u64`:

```vani
intent "hashing values";

fn main() -> i64 {
  let h1: u64 = hash_i64(42);
  let h2: u64 = hash_str("hello");
  let combined: u64 = hash_pair(1, 2);
  let triple: u64 = hash_triple(1, 2, 3);
  print "same input, same hash:", hash_i64(42) == h1;
  print "hash_pair(1,2) == hash_pair(1,2):", hash_pair(1, 2) == combined;
  print "hash_str consistent:", hash_str("hello") == hash_str("hello");
  return 0;
}
```

| Builtin | Description |
|---------|-------------|
| `hash_i64(x: i64) -> u64` / `hash_f64(x: f64) -> u64` / `hash_str(s: Str) -> u64` | hash one scalar value |
| `hash_combine(h1, h2: u64) -> u64` | mix two already-computed hashes into one |
| `hash_combine_3` / `hash_combine_4` | mix three / four hashes |
| `hash_pair(a, b: i64) -> u64` | shorthand for hashing a 2-`i64` composite key |
| `hash_triple(a, b, c: i64) -> u64` | shorthand for a 3-`i64` composite key |
| `f64_hash_pair` / `f64_hash_triple` | same, for `f64` fields |
| `str_hash_pair` / `str_hash_triple` | same, for `Str` fields |
| `siphash_i64(k0, k1: u64, v: i64) -> u64` | keyed SipHash of an `i64`, resists hash-flooding |
| `siphash_str(k0, k1: u64, s: Str) -> u64` | keyed SipHash of a `Str` |

**A composite key pattern**: `hashmap_insert` only takes `i64` keys
in v1, so to key a `HashMap` by (say) an `(x, y)` grid coordinate,
hash the pair down to one `i64` first: `hashmap_insert(mut ref m,
hash_pair(x, y) as i64, value)`.

---

## Safe math: can't crash, even on bad input

The regular math builtins trap on invalid input (division by zero,
`sqrt` of a negative, `log` of zero or negative -- see
[Intermediate 10b](10b_runtime_errors_primer.md) on the L4 runtime
guards). These `_safe_` variants take an explicit fallback value
instead of trapping, for the cases where "bad input" is expected and
you'd rather substitute a default than crash:

```vani
intent "safe math";

fn main() -> i64 {
  let sd:  f64 = f64_safe_div(10.0, 0.0, 0.0 - 1.0);   // -1.0 (fallback; would trap on /)
  let ssq: f64 = f64_safe_sqrt(0.0 - 4.0);              // 0.0 (fallback baked in; would trap on sqrt)
  let isd: i64 = i64_safe_div(10, 0, 0 - 1);            // -1
  let slg: f64 = f64_safe_log(0.0 - 1.0, 0.0 - 9.0);    // -9.0

  print "safe_div(10, 0, fallback=-1):", sd;
  print "safe_sqrt(-4):", ssq;
  print "safe_log(-1, fallback=-9):", slg;
  return 0;
}
```

| Builtin | Description |
|---------|-------------|
| `f64_safe_div(a, b, fallback: f64) -> f64` | `a/b`, or `fallback` if `b == 0.0` |
| `i64_safe_div(a, b, fallback: i64) -> i64` | same, for integers |
| `f64_safe_sqrt(x: f64) -> f64` | `sqrt(x)`, or `0.0` if `x < 0.0` |
| `f64_safe_log(x, fallback: f64) -> f64` | `log(x)`, or `fallback` if `x <= 0.0` |

---

## Bit manipulation

All bit operations work on `i64` values:

```vani
intent "bit manipulation";

fn main() -> i64 {
  let x: i64 = 5;    // binary: 0101

  // Single-bit operations
  let s: i64  = i64_set_bit(x, 1);    // set bit 1  -> 0111 = 7
  let c: i64  = i64_clear_bit(7, 1);  // clear bit 1 -> 0101 = 5
  let t: i64  = i64_toggle_bit(x, 1); // toggle bit 1 -> 0111 = 7
  let b: bool = i64_test_bit(x, 0);   // test bit 0 -> true (bit is set)

  // Bit counting and analysis
  let p: i64 = i64_parity(7);             // 1 (three 1-bits -> odd)
  let l: i64 = i64_leading_zeros(1);      // 63 (only bit 0 set)
  let tr: i64 = i64_trailing_zeros(8);    // 3 (1000 in binary)
  let rv: i64 = i64_reverse_bits(1);      // 1 -> high bit becomes low bit

  // Rotation and byte swap
  let rl: i64 = i64_rotate_left(1, 4);   // 0001 -> 0001_0000 = 16
  let rr: i64 = i64_rotate_right(16, 4); // back to 1
  let bs: i64 = i64_bswap(1);            // byte-reverse (big<->little endian)

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
| `i64_bswap(x: i64) -> i64` | Byte-swap (big<->little endian) |
| `i64_byte_at(x: i64, n: i64) -> i64` | Read byte `n` (0 = least significant) |
| `i64_set_byte(x: i64, n, v: i64) -> i64` | Replace byte `n` with `v`, return the new `i64` |
| `i64_count_leading_ones(x: i64) -> i64` | Count leading 1-bits (complements `i64_leading_zeros`) |
| `i64_count_trailing_ones(x: i64) -> i64` | Count trailing 1-bits |

```vani
intent "byte-level bit ops";

fn main() -> i64 {
  print "byte_at(0x1234, 0):", i64_byte_at(0x1234, 0);           // 52 (0x34)
  print "set_byte(0x1234, 0, 0xFF):", i64_set_byte(0x1234, 0, 255);  // 4863 (0x12FF)
  print "count_leading_ones(-1):", i64_count_leading_ones(0 - 1);    // 64 -- all bits set
  print "count_trailing_ones(7):", i64_count_trailing_ones(7);       // 3 (0b111)
  return 0;
}
```

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
  print "sigmoid(2.0):", sg;          // ~= 0.880

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

**Previous**: [Sec.15 -- Math, random numbers, and clone ->](15_math_rng.md)
**Next**: [Sec.15b -- Vec statistics and combinators ->](15b_vec_stats.md)
