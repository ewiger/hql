# 0028 — Implementing HQL-0024: records, receivers, traits, modules and the standard library

Status: done
Related: [HQL-0024](../proposals/HQL-0024/README.md), [type system](../wiki/hql/type-system.hmd),
[naming](../wiki/hql/naming.hmd), [the Mighty Types corpus](../../examples/mighty-types/README.md)

HQL-0024 specifies eleven parts. This issue is the implementation plan, in the
order the parts depend on each other, with the example corpus that drives it:
every program under `examples/mighty-types/` is written first against the
specification, pinned as `status: proposed`, and promoted to `valid-now` the
day the binary checks and evaluates it.

## Milestones

1. **Program-declared types are real types.** `type Point { x : Int, y : Int }`
   enters the lattice of the program that declares it; `Point(x = 1, y = 2)`
   constructs a record, `p.x` reads it, a union of records is matched with
   `Circle c => …`. Function types reach eight parameters.
2. **Receiver calls** (parts 4 and 8). `value.f(a)` is a call of `f` with
   `value` as its first argument, resolved among trait implementations, visible
   receiver modules and the core operations; `c.title` stays a field read.
   `str(x)` and `fmt("…{}…", x)` join the prelude.
3. **The standard library grows** (part 3): `string`, `int`, `float`, `math`,
   `option`, `result`, `regex`, `format.json`, and the collection operations a
   dataflow language needs: `first`, `last`, `reverse`, `distinct`, `zip`,
   `enumerate`, `any`, `all`, `find`, `range`, `keys`, `values`, `entries`,
   `to_map`, `with`, `join`, `union`, `intersection`, `difference`,
   `take_while`, `drop_while`, `window`, `scan`.
4. **`Result<T, E>`** (part 11): `ok(v)`, `err(e)`, the `ok(p)` and `err(p)`
   patterns, and the combinators beside `Option`'s.
5. **Traits and implementations** (parts 5 and 6). `trait`, `impl … for …`,
   `Self`, default bodies, `T : Trait` bounds checked at the call and usable in
   the body, `Add`, `Sub`, `Mul`, `Div`, `Eq`, `Ord` resolving the operators for
   declared types; a type implementing `Ord` is `Orderable`.
6. **Modules and imports** (parts 1, 2, 9, 10). Dotted paths, `as`, head
   placement, workspace module roots (`hql.toml` `[modules] roots`, `--modules`,
   the corpus header `modules:`), `foo/mod.hql`, `module for T` receiver
   modules, one registry over builtin and workspace providers, and an
   execution context that carries the vault and the module roots.
7. **Contracts in `std/`** (parts 1 and 3): one file per module in the target
   layout, external declarations for what Rust implements, and a test that
   every external declaration has an implementation and every implementation
   a declaration.
8. **Records of the decisions**: the proposal's status, the cards that said
   "there are no traits" and "no receiver call", the changelog, the std
   README, and this issue moved to done.

## Decisions taken while implementing

- Part 7 is adopted from [encode, decode and format](../proposals/HQL-0024/encode-decode-format.md):
  `Format` is a category, every format narrows it and none narrows `String`,
  `Lines` and `JsonLines` are framing formats, and `format(x, F)` gives a
  `String`. HQL-0017 keeps its three questions and its renderers.
- Part 5's split: `Orderable` stays the ancestry form; `impl Ord for T`
  makes `T` `Orderable` for `compare`, `<`, `sort`, `min`, `max`. Structure
  bounds (`Collection`, `Seq`, `Iterable`) stay abstract types.
- Part 4's result-type rule: a receiver function declared in `std/` over
  `Seq<T>` keeps the container rule of the type system card, because the Rust
  implementation it binds to does; a receiver function with an HQL body
  returns what its signature says.
- Types declared by an imported module are visible by their bare name, since
  there is one lattice; functions are reached as `alias.name`.
- A receiver call resolves, in order, a trait implementation for the receiver's
  type, a receiver module whose pattern matches it, and a core operation of that
  name taking it as input. Two candidates of one kind are an ambiguity.

## Outcome

All twenty-three programs of the corpus are `valid-now`; the suite, clippy and
fmt pass. Left for later records: an `impl` of a generic trait written in HQL
(the codec contracts), a workspace module's trait defaults inherited by an
importer's `impl`, `target` folded into `sink`, binary formats, and HQL-0025
for the execution context, reserved.

## Done when

- Every program in `examples/mighty-types/` that the README does not mark as
  waiting is `valid-now`.
- `cargo test`, `cargo clippy --all-targets -- -D warnings` and `cargo fmt
  --check` pass.
- HQL-0024's status says which parts are implemented.
