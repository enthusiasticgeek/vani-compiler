# SOLID design principles in vāṇी -- primer

> **Read this after the [vāṇी design idioms primer](11a_vani_idioms_primer.md)
> and before the [GoF tutorial](11_design_patterns.md).** SOLID explains
> *why* a design is good; GoF explains *named solutions* to recurring
> problems. The two are complementary, not alternatives.

SOLID is a set of five principles for writing maintainable software.
They were articulated for object-oriented languages but none of them
actually require inheritance -- they describe properties of modules,
interfaces, and functions that hold in any typed language. vāṇी maps
them cleanly, and in one case (LSP) can enforce them with the SMT
verifier.

---

## S -- Single Responsibility Principle

> *A module should have one, and only one, reason to change.*

Each struct and function should do one job. When you find yourself
passing a "mode" flag to change behaviour inside a function, that
function is doing two jobs.

**Violation**:

```
fn process(data: ref Vec<i64>, mode: i64) -> i64 {
  if mode == 0 {
    // validate
  } else {
    // format for output
  }
  return 0;
}
```

**Fixed** -- two functions, each with one reason to change:

<img class="manas" src="../images/mascot/manas_mascot_success.png" title="this is the correct, working version"/>

```
fn validate(data: ref Vec<i64>) -> i64 { /* ... */ return 0; }
fn format(data: ref Vec<i64>)   -> i64 { /* ... */ return 0; }
```

In vāṇी, affine ownership reinforces SRP: if a function takes
ownership of a value, it is responsible for that value's full
lifecycle. Splitting responsibilities becomes structurally visible.

---

## O -- Open / Closed Principle

> *Software entities should be open for extension but closed for
> modification.*

In class-based OOP "extension" means subclassing. In vāṇी it means
adding a new `implement` block for an existing `interface` without
touching the interface definition or any existing implementation.

```
interface Renderer {
  fn render(self: Self) -> Str;
}

struct HtmlRenderer {}
struct JsonRenderer {}

implement Renderer for HtmlRenderer {
  fn render(self: HtmlRenderer) -> Str { return "<p>hello</p>"; }
}

implement Renderer for JsonRenderer {
  fn render(self: JsonRenderer) -> Str { return "{\"msg\":\"hello\"}"; }
}

// Adding a third renderer requires ZERO changes to Renderer or
// the two existing impls -- the interface is closed, the set of
// implementors is open.
struct MarkdownRenderer {}
implement Renderer for MarkdownRenderer {
  fn render(self: MarkdownRenderer) -> Str { return "**hello**"; }
}

fn emit(r: dyn Renderer) -> Str {
  return r.render();
}

fn main() -> i64 {
  let h: dyn Renderer = HtmlRenderer {};
  let j: dyn Renderer = JsonRenderer {};
  let m: dyn Renderer = MarkdownRenderer {};
  let _ = emit(h);
  let _ = emit(j);
  let _ = emit(m);
  return 0;
}
```

---

## L -- Liskov Substitution Principle

> *Subtypes must be substitutable for their base types without
> altering the correctness of the program.*

Classical LSP talks about subclasses. In vāṇी the equivalent is:
every type that implements an interface must satisfy not just the
*signature* but also the *behavioral contract* of that interface.

vāṇी can enforce this mechanically with `requires` / `ensures` --
but note the clause has to live on each `implement` block's
function, not on the `interface` declaration itself (v1's parser
doesn't accept `ensures` directly on an interface method signature):

<img class="manas" src="../images/mascot/manas_mascot_success.png" title="this is the correct, working version"/>

```
interface Counter {
  // Contract: after increment, value > value_before.
  fn increment(self: Self) -> i64;
  fn value(self: Self) -> i64;
}

struct UpCounter   { n: i64 }
struct BrokenCounter { n: i64 }   // violates the contract

implement Counter for UpCounter {
  fn increment(self: UpCounter) -> i64
    ensures _return > 0;
  {
    self.n = self.n + 1;
    return self.n;          // always > 0 after first call -- OK
  }
  fn value(self: UpCounter) -> i64 { return self.n; }
}
```

That compiles cleanly -- `UpCounter` satisfies the contract it
declares. Now give `BrokenCounter` the *same* `ensures` clause on
an implementation that can't satisfy it:

<img class="manas" src="../images/mascot/manas_mascot_error.png" title="this code does not compile!"/>

```
implement Counter for BrokenCounter {
  fn increment(self: BrokenCounter) -> i64
    ensures _return > 0;
  {
    return 0 - 1;           // violates its own ensures clause
  }
  fn value(self: BrokenCounter) -> i64 { return self.n; }
}
```

The SMT verifier catches the LSP violation at compile time --
something no runtime test suite can guarantee exhaustively. `vanic`
rejects it with `ensures clause does not hold at this return (SMT
counterexample)`, pointing at the exact `return` that breaks the
promise.

---

## I -- Interface Segregation Principle

> *Clients should not be forced to depend on interfaces they do not use.*

Prefer many small, focused interfaces over one large "fat" interface.
A type that only needs to be printed should not have to implement
serialization methods.

**Fat interface (violation)**:

```
interface Document {
  fn render(self: Self)    -> Str;
  fn serialize(self: Self) -> Str;
  fn validate(self: Self)  -> i64;
  fn compress(self: Self)  -> Vec<i64>;
}
```

**Segregated (correct)**:

```
interface Renderable  { fn render(self: Self)    -> Str;     }
interface Serializable{ fn serialize(self: Self) -> Str;     }
interface Validatable { fn validate(self: Self)  -> i64;     }
```

A struct implements only the interfaces it genuinely supports:

```
struct Invoice {}
implement Renderable   for Invoice { fn render(self: Invoice)    -> Str  { return "invoice"; } }
implement Serializable for Invoice { fn serialize(self: Invoice) -> Str  { return "{}"; }      }
// Invoice does NOT implement Validatable -- that's intentional.
```

In vāṇी, generic bounds let callers express exactly which capability
they need: `fn print<T: Renderable>(x: T)` -- not
`fn print<T: Document>(x: T)`.

---

## D -- Dependency Inversion Principle

> *High-level modules should not depend on low-level modules. Both
> should depend on abstractions.*

Concretely: functions should accept an interface type (or a generic
bound) rather than a specific struct. This decouples the caller from
the implementation and makes both independently testable.

**Coupled (violation)**:

```
struct MySqlDb { connected: i64 }

fn save_user(db: MySqlDb, id: i64) -> i64 {
  // hard-wired to MySqlDb -- cannot swap for a test stub
  return 0;
}
```

**Inverted (correct)** -- depend on the abstraction:

```
interface Database {
  fn save(self: Self, id: i64) -> i64;
}

struct MySqlDb  { connected: i64 }
struct InMemoryDb { store: Vec<i64> }

implement Database for MySqlDb {
  fn save(self: MySqlDb, id: i64) -> i64 { return 0; }
}
implement Database for InMemoryDb {
  fn save(self: InMemoryDb, id: i64) -> i64 {
    vec_push(ref self.store, id); return 0;
  }
}

// Monomorphized at zero cost -- no vtable needed here:
fn save_user<T: Database>(db: T, id: i64) -> i64 {
  return db.save(id);
}

// Or with runtime dispatch when the concrete type isn't known:
fn save_user_dyn(db: dyn Database, id: i64) -> i64 {
  return db.save(id);
}
```

The `<T: Database>` form (inline bound, v0.5.3+) is preferred when
the type is known at the call site; `dyn Database` when it varies
at runtime (e.g. a plugin loaded from config).

---

## Quick reference

| Principle | vāṇी mechanism | Enforced by |
|---|---|---|
| **S** -- Single Responsibility | One function / struct per concern | Code review; affine ownership makes split visible |
| **O** -- Open / Closed | `interface` + new `implement` blocks | Type system: existing code untouched |
| **L** -- Liskov Substitution | `requires` / `ensures` on interface methods | SMT verifier at compile time |
| **I** -- Interface Segregation | Many small `interface` declarations | Type system: implement only what you need |
| **D** -- Dependency Inversion | `<T: Iface>` bounds or `dyn Iface` params | Compiler: concrete struct not visible to caller |

LSP is the standout: vāṇी is one of the few languages where a
Liskov violation in an `implement` block is a *compile error*, not
a bug found at runtime.

---

**Previous**: [vāṇी design idioms -- intuition primer](11a_vani_idioms_primer.md)  
**Next**: [Architectural patterns: Hexagonal and Pipeline ->](11c_architecture_primer.md)
