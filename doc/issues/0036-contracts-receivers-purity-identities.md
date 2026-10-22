# 0036 — Contracts and binding: explicit receivers, declared purity, identities

Status: done
Related: [HQL-0024 part 12](../proposals/HQL-0024/parts/12-contracts-and-binding.md),
[receiver modules](../proposals/HQL-0024/parts/04-receiver-modules.md),
[0030](0030-hql-only-signature-authority.md),
[0033](0033-overloaded-declarations.md),
[extensions](../wiki/hql/extensions.hmd),
[pure and effectful operations](../models/behavior/pure-vs-effectful.md),
[the receivers corpus](../../examples/receivers/README.md)

Part 12 states who says what about a declaration. Three of its work units
are language-core work: a receiver call reaches only a declaration with
`self`; purity is declared in HQL and the extension's marking checked
against it; and loading allocates identities that binding maps to
implementations.

## Scope

- Explicit receivers: `value.f()` resolves among the visible operations
  whose declarations take `self`; a declaration without `self` is refused as
  a receiver call naming the call and pipeline forms.
- Purity: `effectful fn` before an external declaration; binding compares
  it with the extension's marking; checking reads the declaration. `io` and
  `io.logging` declare their operations.
- Identities, first stage: `src/extensions/binding.rs` loads `std/` once,
  allocating a module identity per module and a declaration identity per
  external declaration, and binds each declaration to its operation.
  Signatures, receivers and purity are read through it, and the execution
  tree's step carries the declaration identity the call resolved to.

## Done when

- `tests/mighty.rs` pins receiver calls over written receivers and the
  refusal of `1.range(3)` and of a `regex` function as a receiver call; the
  receivers corpus holds a valid and an invalid case.
- Binding refuses a purity disagreement, and the io and logging operations
  are effectful by their declarations (`binding` unit tests).
- A call, a receiver call, a qualified call and a pipeline step of
  `pad_left` carry the identity of `string.pad_left` (`execution` unit test).

## Decisions taken while implementing

- **The core collection operations and `math` take `self`.** The corpus and
  part 3 write `[3, 1].sort()`, `p.map(f)` and `42.abs()`, and part 12 makes
  a receiver call reach only a declaration with `self`. Their first parameter
  is now a written receiver, `fn size<T>(self: Collection<T>) : Int`,
  `fn abs(self: Int) : Int`, outside any `module for`. The constructors keep
  their parameters: a constructor is not a receiver function.
- **`seq`'s map and set functions take `self`; `range` does not.**
  `with`, `without`, `entries`, `values`, `intersection` and `difference`
  write their receiver; `range(start, end, step)` has no natural receiver
  and stays a function, `range(1, 3)` and `1 | range(3)`.
- **`regex` declares no receivers.** Its functions take a text and a
  pattern, `regex.is_match(text, pattern)`, so `"ab".is_match("a")` is
  refused; `"a,b".split(",")` is String's `split` whichever modules are
  visible, since the receiver call considers only receiver declarations.
- **The io and logging operations take no receivers.** Endpoints and
  loggers are read, written and logged to with a call or a pipeline step,
  as the corpus does; nothing reached them as a receiver call before.
- **One name takes a receiver in every form or in none.** Whether
  `value.f()` is a call is a property of the name; binding refuses an
  overloaded name whose forms disagree.
- **A receiverless declaration still fills the operation's input slot.**
  The checker's structural signature keeps one input; for a declaration
  without `self` it is the first argument, which is call lowering, and the
  declaration's identity records that it has no receiver.
- **The purity syntax is `effectful fn`.** Part 12 names no syntax. `fn`
  alone is pure, the common case and the default of every extension's
  marking; `effectful` is a word only before `fn`, as `type` and `import`
  are words by position, and belongs to an external declaration: a function
  with an HQL body is effectful exactly when its body uses an effectful
  operation, as before, and writing `effectful` before one is a syntax
  error.
- **The extension's marking stays, as the thing checked.** `Operation`'s
  `purity` is the extension's own marking; `Operation::purity()` returns the
  declaration's, and binding refuses `io.tap` declared pure while `io` marks
  it effectful. An operation of a module without a contract (`graph`,
  `io.net`, `lexical`, `semantic`) has no declaration, and its marking,
  pure, stands.
- **`source`'s declared result is `Iterable<T>`.** Its result is computed
  from the call, a Seq from a Replayable endpoint and a Stream from a live
  one, so the native signature stays and binding compares only what the
  declaration can state; `T` is the type the format decodes to. A format
  argument, `Type<F>`, is native-only in the comparison, as before, so the
  declarations of `source`, `tap` and `Logger` write `format: Type<F>`
  without a default and the comment states when it may be omitted.
- **`Logger` is an operation, not a table constructor.** It is named as its
  type and implemented by the `logging` extension, so a constructor-named
  declaration whose module's extension has an operation of that name is
  bound to the operation; the table constructors are the rest.
- **Identities are indexes into the loaded environment.** `ModuleId` and
  `DeclarationId` are positions in loading order; an operation is found by
  its address in its extension's static table, which is fixed for the
  process. They are not persistent.

## Left open

- **Package identities, and resolution by identity** (work unit 1):
  modules, declarations, types, traits, implementations and constructors
  have identities, and native headers, codecs and constructors are bound by
  them. A call still finds its operation by name in each visible module's
  operation table, the declaration identity attached through the overload
  the call takes; a native trait function is dispatched by trait name and
  receiver type among the bound headers.
- **Native type representation** (work unit 6): `File` is still a
  `Value::Endpoint`.
- **Capabilities** (work unit 7): only purity is declared.
- **External extension loading** (work unit 9): the runtime environment is
  built, validated once and frozen per module graph and extension set, and
  the process holds one, over the compiled-in extensions; no descriptor is
  loaded from outside the binary.
