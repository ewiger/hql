# Part 12: Contracts and binding

- **Source**: the review of the extension machinery against the released
  `0.1.0` registry, adopted as the binding model of this proposal
- **Depends on**: [modules](01-modules.md) (a module is a contract),
  [traits](05-traits.md) (an implementation is declared),
  [packages and extensions](09-packages-and-extensions.md) (the registry is
  the second input of binding)
- **Needed by**: external extensions, the last stage of
  [part 9](09-packages-and-extensions.md)

## Scope

This part fixes who says what about a declaration. **An HQL module is a
language contract. Extensions do not extend the HQL type system; they provide
native implementations for declarations already present in that contract.**
A module does not inherently require an extension: it may hold HQL bodies,
native implementations, or both.

```text
                  HQL source
                     │
       ┌─────────────┴─────────────┐
       │                           │
 declarations                  HQL bodies
       │                           │
       ▼                           ▼
  type environment          HQL implementation
       │
       │ declaration and implementation identities
       ▼
  implementation registry
       │
       ├── HQL implementation
       │
       └── native extension implementation
```

## Rules

### An extension implements declarations and creates none

Every language-level fact is declared in HQL: that a function exists and its
signature, that a type exists with its parents, fields and constructor, that a
type implements a trait, and the semantic properties of a function (purity,
effects, capabilities). An extension supplies executable bodies and runtime
representations for declarations it is bound to. Anything an extension offers
that no declaration names is a binding error, not an addition to the language.

### One declaration behind every call form

```hql
"x".pad_left(3, "0")
string.pad_left("x", 3, "0")
"x" | pad_left(3, "0")
```

A method call, a qualified call and a pipeline step resolve to the same
declaration, `string.pad_left`, first. Only after resolution does the
declaration point at an implementation. The checker never asks the extension
registry what a name means for one syntax.

```text
method syntax ───────┐
qualified call ──────┼──> declaration string.pad_left ──> native implementation
pipeline syntax ─────┘
```

### Receivers are explicit

A declaration has a receiver or it does not:

```text
function declaration
    receiver   : the type of self, or none
    parameters : in order
    result     : one type
```

`fn pad_left(self, width: Int, fill: String) : String` has a receiver;
`fn parse(text: String) : Data` has none. `text | parse` lowering to
`parse(text)` is call lowering, a property of the pipeline syntax, and not
part of the module's implementation interface: a first parameter is never a
receiver.

### HQL is the only signature authority

The types of a declaration's receiver, parameters and result, its generic
parameters and their bounds, and which arguments may be omitted come from the
`.hql` contract alone: `fn range(start: Int, end: Int, step: Int = 1)`
declares `step` optional. A default is written only in an external
declaration; a function with an HQL body takes every argument it declares. An
extension's operation carries a handler and, where it needs one, an
implementation interface descriptor (how many evaluated arguments it receives,
whether it takes a lambda); it carries no HQL types.

### A native implementation of a trait is declared in HQL

```hql
impl Ord for Int
impl Ord for Float
impl Ord for String
```

An `impl` header without a body declares that a type implements a trait
natively. The extension binds a body to that declaration:

```text
impl Ord for Int    -> native compare over Int
impl Ord for Float  -> native compare over Float
impl Ord for String -> native compare over String
```

There is one implementation table. Trait resolution for `a.compare(b)`,
`a < b`, `sort(xs)`, `min(xs)` and `max(xs)` finds the implementation of
`Ord` for the type; whether its body is HQL or native does not reach type
checking. Only a module an extension provides may declare a bodyless header;
in a program or a workspace module, an implementation's functions are written
in braces.

### Coherence and ownership

Within one resolved environment there is at most one implementation for a
trait and a type. Two are an error naming both, whether they come from two
modules, a module and the standard library, or one program; import order
never chooses. The one implementation reached through two imports is still
one. An implementation is declared by the unit that declares the trait or the
type, so no third party changes what `Add for Int` means.

### A module claim is total

An extension that provides a module implements every native declaration of
that module and nothing else:

```text
declaration without an implementation  -> missing implementation error
implementation without a declaration   -> undeclared operation error
```

`provides` names a module identity, not a spelling: names are matched once,
during binding, and execution never looks a declaration up by its name again.

### Identities

Loading the module graph allocates identities: package, module, declaration,
type, trait and implementation. They are indexes assigned while loading, not
persistent identifiers. Binding maps declaration and implementation
identities to bodies; everything after binding works on identities.

### A native type has a declared identity and a supplied representation

```hql
type File : {Replayable, Writable} {
    path : String
}

fn file(path: String) : File
```

`File`, its parents and its field `path` are declared in HQL. The extension
supplies the runtime representation of `File`, the body of the constructor
`file`, and the projection that reads `path` from the representation. The
runtime value of an extension-represented type carries the identity of the
HQL-declared type, so the closed set of built-in value representations is
not the interface third-party extensions implement.

### Semantic properties are part of the contract

Whether a function is pure or effectful, and later which capabilities it
needs (filesystem, network, vault), changes what the executor may do with a
call: reorder, cache or fuse a pure one, and preserve an effectful one. That
belongs to the declaration. An extension's own purity marking is checked
against the declaration, never the source of it.

### Validation belongs to a runtime environment

```text
runtime environment =
    module graph
  + extension set, with versions
  + configuration
```

An environment is built, bound, validated and frozen, and any number of
programs then run against it. A process caches it while nothing changes. With
every extension compiled into the binary there is one environment per
process; with external extensions there is one per extension set.

## Current state

| Rule | State |
| --- | --- |
| an extension creates no declaration: every operation of a module an extension provides is declared | built for functions: binding refuses an undeclared or missing operation, by name |
| one declaration behind every call form | built: a call, a receiver call, a qualified call and a pipeline step resolve the same declaration, and the checked execution tree carries its identity beside the operation bound to it ([issue 0036](../../../issues/0036-contracts-receivers-purity-identities.md)). Resolution still matches the written name against the operation table of each visible module's extension, then takes the declaration identity through the overload the call fits; whether the operation has a receiver, is private, and which signatures it takes are read from its declaration identities in the environment the program runs against |
| explicit receivers | built: `value.f()` reaches only a declaration with `self`, typed by the module's pattern or written, `fn join(self: Collection<String>, …)`, `fn abs(self: Int)`; a declaration without `self`, `range(start: Int, …)` or `regex.is_match(text: String, …)`, is reached by a call or a pipeline step and refused as a receiver call. Every form of one name takes a receiver or none does. A receiverless declaration's first parameter fills the operation's input slot, which is call lowering, and binding records that the declaration has no receiver |
| HQL is the only signature authority | built for every operation of the core, `seq`, `math`, `string`, `option`, `result`, `regex` and `format`: such an operation is `Signature::Declared`, its Rust side a name, a summary and a body, and binding derives the checking signature from the declaration. A result that keeps, reorders or folds its input (`reverse`, `distinct`, `take_while`, `drop_while`, `with`, `without`, `intersection`, `difference`, `abs`, `pow`, `sign`), an ordering argument (`min_by`, `max_by`), a stream's result beside a finite input's (`format`) and the narrowing `filter`, whose predicate is a `T -> Is<U>`, are stated by overloaded declarations; `/` and `*` mark the arguments supplied by position only and by name only, and `...Any` a variadic argument of any type in each position. A call takes the first declaration its input, arity and value arguments' types fit, and binding refuses a declaration an earlier one always takes first. An operation of a module without a contract, and one whose result is computed from the call, keep a native signature, and binding compares it with the declaration in full wherever the declaration states one |
| native trait implementations declared in HQL | built: `lib/std/core/mod.hql` declares `impl Eq`, `Ord`, `Add`, `Sub`, `Mul`, `Div`, `Rem` for the built-in types they hold for; binding binds each header's implementation identity to its native body, matching the trait and type names once, and refuses a header without a native body and a native body without a header; the checker's native implementations are read from the bound headers, a bound `T : Eq` accepts `Int` through the header, and dispatch reaches only a body bound to a header |
| coherence and ownership | built: one implementation per trait and type across the standard library, imported modules and the program, the error naming both; an implementation belongs with its trait or its type, which binding checks for a standard header by the modules of the trait and type identities; a program writes no bodyless header |
| a module claim is total | built: a declaration is bound to its provider's operation by name once, at binding, and validation refuses a declaration without an operation and an operation without a declaration; a `private fn` declaration is bound like any other and offered to no importer |
| identities | built for modules, external declarations, types, traits, implementations and constructors: loading `lib/std/` allocates each an identity; binding maps a declaration identity to its operation, an implementation identity to its trait, its type and its native body, and each constructor of the binary's table to the type identity whose constructor declarations state its forms. An operation's signatures, receiver, purity, shape and privacy are read through its declarations, and a constructor's forms through its type identity. Package identities are not allocated. Resolution matches the written name among the visible modules' operation tables, and dispatch of a native trait function matches the trait's name and the receiver's type constructor among the bound headers at each call |
| native type identity | built-in types use the closed `Value` representations; `File` is a `Value::Endpoint` |
| semantic properties in the contract | built for purity: `effectful fn` declares an external declaration effectful and `fn` pure; binding refuses an extension whose marking differs, and checking reads the declaration. `io` and `io.logging` declare their operations, `source`, `tap`, `target`, `sink` and the five levels effectful. The operations of `graph`, `io.net`, `lexical` and `semantic` have no declaration, so their extension's marking, pure, stands. Capabilities are not declared |
| validation per environment | built: a runtime environment is the module graph and the extension set, loaded and bound, then validated once and frozen; a frozen environment is immutable and shared by every program run against it, and a configuration's imports are resolved over it once, as the runtime every program of that configuration starts from. The process caches one environment per module graph and extension set, and one runtime per configuration; a failing validation refuses the environment, and every program under it, naming the defect |

## Work units

In order:

1. **Declaration identity and binding.** Load `.hql` modules first, allocate
   module, declaration, type and trait identities, then bind extension
   implementations to them. Lower all three call forms to one declaration
   identity. Done for modules, external declarations, types, traits,
   implementations and constructors: native headers are bound to native
   bodies and constructors to type identities at binding, and the call forms
   carry the declaration identity. Package identities remain, and so does
   resolution by identity: a call's name is matched among the visible
   modules' operation tables, and a native trait function is dispatched by
   trait name and receiver type.
2. **HQL as the only signature authority.** Done for every operation of the
   core, `string`, `math`, `regex`, `option`, `result`, `seq` and `format`
   contracts ([issue 0033](../../../issues/0033-overloaded-declarations.md)):
   overloaded declarations state the kept, reordered and folded results, the
   ordering arguments, a stream's result and the narrowing `filter`. The
   operations of `graph`, `io`, `net`, `logging`, `lexical` and `semantic`
   keep native signatures.
3. **Explicit receivers.** Done: a receiver call reaches a declaration with
   `self` only; pipeline lowering stays in the checker.
4. **Native trait implementation headers.** Done for the core traits over
   the built-in types.
5. **Coherence validation.** Done for trait implementations; extended to
   duplicate providers of one module and of one native implementation as
   environments gain extension sets.
6. **Native type representation.** A declared type's identity, its supplied
   representation, constructor binding and field projection, with a native
   value carrying the declared type's identity.
7. **Semantic metadata in contracts.** Purity, then capabilities, declared in
   HQL and verified against the extension. Done for purity, `effectful fn`;
   capabilities remain.
8. **Environment validation.** Replace the once-per-process check with a
   built, validated and frozen runtime environment. Done for the
   compiled-in extension set: the environment is built from the module graph
   and the extension set, validated once and frozen, and each configuration
   is resolved over it once.
9. **External extension loading.** Load a descriptor, resolve what it
   provides, bind implementations, validate the environment and freeze it:
   with the units above in place, loading adds no new model.
