# Intermediate 11 -- The 22 GoF design patterns

> **Learning goal**: orient yourself in the
> [`design_patterns/`](https://github.com/enthusiasticgeek/vani-compiler/tree/main/examples/language/english/design_patterns)
> directory, understand which v1 idiom each pattern uses, and
> know where to look when you reach for one.

> **Prerequisites**: read the [vāṇी design idioms primer](11a_vani_idioms_primer.md)
> and the [SOLID primer](11b_solid_primer.md) first. The idioms page
> shows the vāṇी-native shapes (enum-as-hierarchy, closure-as-strategy,
> arena-as-graph); SOLID explains *why* a design is good before GoF
> names the recurring solutions.

Design patterns are named solutions to recurring problems --
think of them as blueprints, not finished buildings. A
"Factory" pattern is not a specific piece of code; it's the
idea "have one place that decides which concrete type to
create, so callers don't need to know." The GoF ("Gang of Four")
catalogue named 23 such patterns in 1994 and every major
language community has since translated them into idiomatic
examples. This chapter is a guided tour of the vāṇी versions --
it assumes you've read through Intermediate 1-13 (structs,
enums, ownership, generics, closures, collections) so the
idioms feel familiar.

## The directory layout

```
examples/language/english/design_patterns/
+-- creational/
|   +-- factory_method.vani
|   +-- abstract_factory.vani
|   +-- builder.vani
|   +-- prototype.vani
|   +-- singleton.vani
+-- structural/
|   +-- adapter.vani
|   +-- bridge.vani
|   +-- composite.vani
|   +-- decorator.vani
|   +-- facade.vani
|   +-- flyweight.vani
|   +-- proxy.vani
+-- behavioral/
    +-- chain_of_responsibility.vani
    +-- command.vani
    +-- iterator.vani
    +-- mediator.vani
    +-- memento.vani
    +-- observer.vani
    +-- state.vani
    +-- strategy.vani
    +-- template_method.vani
    +-- visitor.vani
```

Each file:

1. Cites the [refactoring.guru](https://refactoring.guru/design-patterns) URL.
2. States the textbook intent in one paragraph.
3. Lays out the **v1-specific deviation** (often a workaround
   for a documented [v1 limitation](https://github.com/enthusiasticgeek/vani-compiler/blob/main/docs/v1_limitations.md)).
4. Provides a worked example that compiles + runs on both
   backends.

## The idioms you'll see (and why)

vāṇी v1's constraints push these patterns toward specific
shapes. The most important to internalize:

| Pattern | v1 idiom | Why |
|---|---|---|
| **Composite** | Tagged-struct (Leaf vs Branch flag + child indices into an arena `Vec<Composite>`) | No `Box<T>` (L2) → no recursive enums; use index-based arena instead |
| **Bridge** | Integer discriminator + per-impl free fn | Cleanest decoupling without a shared vtable field |
| **Decorator** | Flag-bag struct + a single `apply()` function that conditionally enables features | Avoids chained dyn-wrapper objects |
| **Observer** | `Vec<fn(i64) -> i64>` + index-based dispatch | Zero-overhead when all subscribers share one signature; use `Vec<dyn Iface>` for heterogeneous subscribers |
| **Singleton** | `Atomic<i64>` (or `Atomic<f64>` for float state) passed by `ref` from `main()` | No `static mut`; affine borrow prevents aliasing bugs |
| **Visitor** | Match on a tagged-struct discriminator + per-arm free fn | Closest fit without dyn double-dispatch |
| **Iterator** | Both forms shown -- manual `while idx < len` AND `for x in xs` (the latter for `Vec<T>` only in v1) | Rust-style `Iterator` trait is not in v1 stdlib |
| **Strategy** | `fn` pointer for stateless; `<T: Iface>` inline bound for stateful (v0.5.3+) | `<T: Sorter>` replaces verbose `where T is Sorter` and monomorphizes at zero cost |
| **Template Method** | Default methods on an `interface` | Overrideable steps without inheritance |

The remaining patterns (`Adapter`, `Facade`, `Proxy`, `State`,
`Builder`, `Factory Method`, `Abstract Factory`, `Prototype`,
`Memento`, `Chain of Responsibility`, `Command`, `Mediator`)
are more direct ports that follow standard shapes with minor
surface differences (`then` instead of `=>` in `match`, no `Box`).

## Sample workflow

When you reach for a pattern:

1. Open the matching file under `design_patterns/<category>/`.
2. Read the textbook intent comment at the top.
3. Read the v1-deviation section to understand the idiom.
4. Adapt the worked example to your problem.

Each pattern's worked example is in the dual-backend parity
sweep, so it's guaranteed to compile + run on every release.

## When *not* to reach for a GoF pattern

vāṇी's affine ownership + interface system + match-on-enum
flatten a lot of object-oriented design space. Before reaching
for, say, **Strategy** + a `dyn StrategyIface`, ask whether a
plain enum with one variant per strategy is cleaner. Often it
is. The pattern files are a *reference* -- not a checklist.

## Challenge

Pick one design pattern you've used in another language. Open
its `.vani` example. Reimplement the same pattern with one
change: alter the worked example to fit a small concrete
problem you actually have (a small CLI, a parser, a game
loop). Run it through `vanic run`. Note where the v1 idiom
felt natural and where it pushed back.

---

**Previous**: [SOLID design principles](11b_solid_primer.md)  
**Next**: [SMT -- `requires` / `ensures` intuition primer ->](12a_smt_primer.md)
