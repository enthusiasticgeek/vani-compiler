# GoF Design Patterns in vāṇī

All 22 Gang-of-Four design patterns from
[refactoring.guru/design-patterns](https://refactoring.guru/design-patterns),
implemented in vāṇī's English-keyword surface. Each file is a
runnable single-file program with the pattern's intent in a
header comment and a small driver in `main()`.

These patterns ship in the dual-backend parity sweep at
`tests/run_end_to_end.rs::llvm_backend_run_produces_same_output_as_c`,
so each pattern produces byte-identical output across the C and
LLVM backends.

## Creational (5)

How objects are created. Patterns that decouple a system from the
specific classes it instantiates.

| Pattern | File | Intent |
|---|---|---|
| Factory Method | [`creational/factory_method.vani`](creational/factory_method.vani) | Defer instantiation to a sub-type (concrete creators). |
| Abstract Factory | [`creational/abstract_factory.vani`](creational/abstract_factory.vani) | Build families of related products. |
| Builder | [`creational/builder.vani`](creational/builder.vani) | Staged construction of a complex object. |
| Prototype | [`creational/prototype.vani`](creational/prototype.vani) | Clone-and-modify from a seed. |
| Singleton | [`creational/singleton.vani`](creational/singleton.vani) | One process-wide instance (`Atomic<i64>` counter). |

## Structural (7)

How classes / objects compose into larger structures. Patterns
that ease composition without sacrificing flexibility.

| Pattern | File | Intent |
|---|---|---|
| Adapter | [`structural/adapter.vani`](structural/adapter.vani) | Translate one interface into another the client expects. |
| Bridge | [`structural/bridge.vani`](structural/bridge.vani) | Decouple abstraction from implementation. |
| Composite | [`structural/composite.vani`](structural/composite.vani) | Treat individual + composed objects uniformly. |
| Decorator | [`structural/decorator.vani`](structural/decorator.vani) | Add responsibilities to an object dynamically. |
| Facade | [`structural/facade.vani`](structural/facade.vani) | Unified interface over a subsystem. |
| Flyweight | [`structural/flyweight.vani`](structural/flyweight.vani) | Share fine-grained intrinsic state across many instances. |
| Proxy | [`structural/proxy.vani`](structural/proxy.vani) | Surrogate that controls access to another object (caching here). |

## Behavioral (10)

How objects interact and distribute responsibility.

| Pattern | File | Intent |
|---|---|---|
| Chain of Responsibility | [`behavioral/chain_of_responsibility.vani`](behavioral/chain_of_responsibility.vani) | Pass a request along a chain until handled. |
| Command | [`behavioral/command.vani`](behavioral/command.vani) | Encapsulate a request as a queue-able object. |
| Iterator | [`behavioral/iterator.vani`](behavioral/iterator.vani) | Sequential access without exposing storage. |
| Mediator | [`behavioral/mediator.vani`](behavioral/mediator.vani) | Centralize peer-to-peer interaction in one object. |
| Memento | [`behavioral/memento.vani`](behavioral/memento.vani) | Capture + restore internal state. |
| Observer | [`behavioral/observer.vani`](behavioral/observer.vani) | One-to-many notification on state change. |
| State | [`behavioral/state.vani`](behavioral/state.vani) | Behavior varies with internal state (transitions). |
| Strategy | [`behavioral/strategy.vani`](behavioral/strategy.vani) | Pluggable, interchangeable algorithms. |
| Template Method | [`behavioral/template_method.vani`](behavioral/template_method.vani) | Skeleton algorithm with a customizable hook step. |
| Visitor | [`behavioral/visitor.vani`](behavioral/visitor.vani) | Operations on an object structure without changing the elements. |

## Running

Each example is single-file and self-contained:

```bash
# pick any pattern
vanic run examples/language/english/design_patterns/behavioral/observer.vani

# or run via the C backend
vanic run examples/language/english/design_patterns/structural/proxy.vani --backend=c
```

The dual-backend parity sweep covers all 22:

```bash
cargo test --test run_end_to_end llvm_backend_run_produces_same_output_as_c
```

## vāṇी-specific deviations from textbook GoF

A few patterns adapt to vāṇी's affine ownership + composition-over-
inheritance model. The full catalog of v1 deviations (not just
design-pattern ones — also includes parser shortcuts, codegen
quirks, by-design choices) lives in
[`docs/v1_limitations.md`](../../../../docs/v1_limitations.md);
each deviation below cross-references its L-number there.

- **Composite** uses a tagged struct (`kind: i64` discriminator)
  instead of an enum with a `Vec` payload — vāṇी's v1 doesn't
  destructure-bind non-Copy enum payloads. See
  [v1_limitations.md L1](../../../../docs/v1_limitations.md).
- **Bridge** uses an integer discriminator for the renderer family
  instead of `Box<dyn Renderer>` — vāṇी's v1 has no `Box`-like
  owning-interface-object yet. See
  [v1_limitations.md L2](../../../../docs/v1_limitations.md).
- **Decorator** composes via a flag-bag struct (`email`/`sms`/
  `slack` bools) instead of nested decorators wrapping each other —
  same composable additive semantics, different representation.
- **Observer** keeps observers in a `Vec<dyn Observer>` parameter
  rather than a struct field — v1 C-codegen has a known issue with
  `Vec<dyn Iface>` as a struct field. See
  [v1_limitations.md L8](../../../../docs/v1_limitations.md).
- **Mediator** + **Observer** extract for-loops over `self.field`
  into free functions; the parser can't take `ref self.field` at
  a for-loop head. See
  [v1_limitations.md L7](../../../../docs/v1_limitations.md).
- **Proxy** declares state via `let p: CachingProxy = ...;` with no
  `let mut`; mutation goes through `mut ref self` on methods. See
  [v1_limitations.md L5](../../../../docs/v1_limitations.md).
- **Singleton** lives in `main()`'s scope and is borrowed by
  callers — vāṇी has no globally-mutable storage by default
  (affine ownership). The `Atomic<i64>` counter pattern shown is
  the closest idiomatic equivalent.

Each example header comment calls out its specific deviation
inline.

## Translations to other languages

Translate any English design-pattern example to Sanskrit / Hindi /
Marathi via the cross-language tool:

```bash
python3 tools/vani_translate.py \
  --to sanskrit \
  examples/language/english/design_patterns/behavioral/observer.vani \
  -o /tmp/observer_sa.vani --add-sri-header
vanic run /tmp/observer_sa.vani
```

Pure-Devanagari design-pattern examples may ship in a future
session if there's demand.
