# Queries: traversal, sequences, generators, and bound sources

Status: **proposed design; not implemented.** Nothing here is yet part of the grammar recorded in [expressions.md](expressions.md). The design depends on function types ([issue 0009](../../issues/0009-function-type-and-constructor.md)) and replaces the special `cards` keyword ([issue 0008](../../issues/0008-cards-as-a-vault-binding.md)) with an ordinary bound value.

A query does not have one privileged source type. A pipeline receives a value, and each step states the weakest property it needs from that value.

For example:

```hql
cards
| filter(.metadata.type == "concept")
| take(5)
```

`filter` only needs traversal. `take` additionally needs a defined order. `size` instead needs finiteness.

These properties should be represented directly in the type hierarchy.

## Collection hierarchy

There are three independent questions about a group of values:

Does it support traversal?

Is that traversal ordered?

Is the number of elements finite?

The hierarchy follows directly from those questions:

```hql
abstract type Iterable<T>

abstract type Collection<T> : Iterable<T>

abstract type Stream<T> : Iterable<T>

abstract type Seq<T> : {Collection<T>, Stream<T>}

type List<T> : Seq<T>

type Set<T> : Collection<T>
```

Conceptually:

```text
                    Iterable<T>
                    /         \
                   /           \
          Collection<T>      Stream<T>
             /     \           /
            /       \         /
       Set<T>       Seq<T>
                      |
                   List<T>
```

`Iterable<T>` means only that the values can be traversed. It promises neither finiteness nor a meaningful order.

`Collection<T>` adds finiteness. Its elements can therefore be counted, membership can be decided by complete traversal, and multiplicity has a finite sum.

`Stream<T>` adds a stable traversal order. It does not promise that traversal terminates.

`Seq<T>` has both properties: it is a finite collection with a defined order.

`List<T>` is a materialized sequence.

`Set<T>` is a finite collection without positional semantics.

This makes `Seq<T>` the intersection that was missing from the previous hierarchy:

```hql
Seq<T> : {Collection<T>, Stream<T>}
```

It is not necessary to choose whether `Seq` is fundamentally a collection or fundamentally a stream. It is both.

## The contracts are semantic

These types describe observable guarantees, not storage strategies.

A `List<T>` is materialized.

A `Seq<T>` may be materialized or generated lazily.

A `Stream<T>` may be finite or unbounded.

Consequently, **laziness is not a type relationship**.

A lazy sequence remains:

```hql
Seq<T>
```

because it still has a finite size and a defined order.

An unbounded generator is:

```hql
Stream<T>
```

because it has an order but cannot satisfy the finite `Collection<T>` contract.

This is the reason `Stream<T>` exists. It is not "the lazy version of `Seq<T>`". It is the ordered traversal type without the promise of finiteness.

## Collection laws

`Collection<T>` keeps the deliberately weak contract already established for collections:

```hql
size     : Collection<T> -> Int
contains : (Collection<T>, T) -> Bool
count    : (Collection<T>, T) -> Int
```

with:

```text
size(c) is finite

contains(c, x) <=> count(c, x) > 0

size(c) = Σ count(c, x)
```

A collection contains occurrences, not necessarily unique values. That allows both `Seq<T>` and `Set<T>` to refine it.

`Set<T>` adds the constraint:

```text
count(s, x) <= 1
```

`Seq<T>` instead adds stable order and positional semantics.

Neither property belongs on `Collection<T>` itself.

## Order belongs to `Stream<T>`

A `Stream<T>` establishes a stable succession of elements:

```text
x₀, x₁, x₂, ...
```

It does not establish a finite `size`.

That is exactly what operations such as `first`, `take`, and `drop` require. They do not require a collection; they require an ordered traversal.

For example:

```hql
fn first<T>(xs: Stream<T>) : Option<T>

fn take<T>(xs: Stream<T>, n: Int) : Seq<T>

fn drop<T>(xs: Stream<T>, n: Int) : Stream<T>
```

`take` returns a `Seq<T>` because a bounded prefix is finite even when the input is not.

`drop` cannot make the same promise. Dropping five elements from an unbounded stream still produces an unbounded stream.

When the stronger input type is known, operations may preserve it:

```hql
fn drop<T>(xs: Seq<T>, n: Int) : Seq<T>
```

The same rule applies to transformations:

```hql
fn map<T, U>(xs: Stream<T>, f: T -> U) : Stream<U>
fn map<T, U>(xs: Seq<T>,    f: T -> U) : Seq<U>

fn filter<T>(xs: Stream<T>, f: T -> Bool) : Stream<T>
fn filter<T>(xs: Seq<T>,    f: T -> Bool) : Seq<T>
```

A transformation should preserve every guarantee it can prove.

## Finiteness belongs to `Collection<T>`

Operations that require complete traversal accept `Collection<T>`, not `Stream<T>`:

```hql
fn size<T>(xs: Collection<T>) : Int
fn count<T>(xs: Collection<T>, x: T) : Int
```

Likewise, global ordering requires a finite input:

```hql
fn sort<T: Orderable>(xs: Collection<T>) : Seq<T>
```

This gives an unordered finite collection a deterministic order:

```hql
names : Set<String>

ordered = names | sort
```

By contrast:

```hql
stream | sort
```

is a type error, because sorting an arbitrary stream would require seeing an end that may not exist.

Materialization has the same boundary:

```hql
fn list<T>(xs: Seq<T>) : List<T>
```

An arbitrary `Stream<T>` cannot be materialized safely without first introducing a finite bound:

```hql
stream
| take(100)
| list
```

## `Iterable<T>` is the common floor

`Iterable<T>` exists so that unordered and potentially unbounded traversal are not artificially forced into either `Collection` or `Stream`.

It is intentionally weak.

It does not provide:

```text
size
position
first
take
drop
indexing
sorting
```

Those operations require stronger types.

An `Iterable<T>` may still participate in operations whose semantics do not depend on order or finiteness, but a function cannot expose an arbitrary traversal order as a positional result without first establishing an order.

This is particularly important for `Set<T>`. Its runtime representation obviously has to enumerate elements somehow, but that enumeration order is not part of the HQL value.

A positional operation therefore cannot consume a bare set:

```hql
xs : Set<Card>

xs | take(5)       // error: Set<Card> is not Stream<Card>
```

An order must first be introduced:

```hql
xs
| sort(by = .name)
| take(5)
```

Now the prefix is reproducible.

## `Map<K,V>` is separate

`Map<K,V>` should not be forced into the one-parameter hierarchy merely to make the diagram complete.

Doing so immediately raises the question of what its element type is:

```text
K?
V?
(K, V)?
Entry<K,V>?
```

Until HQL has an explicit entry or pair abstraction, `Map<K,V>` is better treated as a keyed collection with its own operations.

This avoids inventing a false subtype relationship just for structural symmetry.

## `cards` is a `Seq<Card>`

The vault is finite, and card enumeration must be reproducible. Therefore the standard `cards` value should be:

```hql
cards : Seq<Card>
```

not:

```hql
Set<Card>
```

and not:

```hql
Stream<Card>
```

The sequence order must be part of the source contract. For the vault, ordering by canonical card name is sufficient and already aligns with the `Orderable` property used elsewhere.

This makes:

```hql
cards | take(5)
```

deterministic across machines.

Filesystem discovery order must never become observable HQL semantics.

## `yield` defines an iterating function

A function whose body contains `yield` is an **iterating function**.

For example:

```hql
fn get_all_cards(v: Vault) : Seq<Card> {
    for c in v.documents {
        yield c
    }
}
```

Each `yield` contributes one element to the traversal in the order it is reached.

Calling the function does not imply materialization. Evaluation advances only as far as its consumer demands:

```hql
get_all_cards(vault)
| take(5)
```

need only produce five cards.

The important point is that `yield` does **not** decide whether the result is a `Seq<T>` or a `Stream<T>`.

The declared return type states the guarantee:

```hql
fn finite_source(...) : Seq<T> {
    ...
    yield x
}

fn unbounded_source(...) : Stream<T> {
    ...
    yield x
}
```

A `Seq<T>` declaration promises that traversal is finite.

A `Stream<T>` declaration does not.

The checker verifies that yielded values have type `T`; the surrounding source or implementation must justify the stronger finiteness contract when `Seq<T>` is declared.

This keeps generator syntax separate from collection semantics.

## `List<T>` and generated `Seq<T>` are deliberately different

These are not equivalent:

```hql
fn a(...) : List<T>
fn b(...) : Seq<T>
```

Returning a `List<T>` promises a materialized value.

Returning a `Seq<T>` promises only a finite ordered sequence. Its implementation may be lazy.

That gives HQL a useful abstraction boundary:

```text
List<T>   finite + ordered + materialized
Seq<T>    finite + ordered
Stream<T> ordered
```

Materialization becomes an implementation property only where the public type explicitly promises it.

## Runtime representation is not `Stream<T>`

The runtime carrier should not itself be called `Stream`.

A lazy `Seq<T>` and a `Stream<T>` need exactly the same underlying mechanism. Naming the carrier `Stream` conflates the implementation strategy with the HQL semantic type.

Use a neutral representation instead, conceptually:

```text
Traversal(Rc<TraversalSource>, TypeRef)
```

where `TraversalSource` describes how to start a fresh traversal and `TypeRef` says whether the HQL value is:

```text
Seq<T>
Stream<T>
...
```

The carrier is a **factory, not a cursor**.

Cloning a value therefore clones the traversal factory, not its current position.

This keeps:

```hql
first_two = cards | take(2)
also_two  = cards | take(2)
```

referentially transparent. Both traversals begin from the same source state and produce the same values.

A live shared cursor would make the second expression depend on whether the first expression had already consumed the value, which does not fit HQL's pure value model.

## Traversal is pushed

The existing evaluator is tree-walking and cannot naturally suspend its Rust call stack at every `yield`.

The simplest lazy implementation therefore uses push traversal.

Conceptually:

```text
TraversalSource -> Sink<T> -> Continue | Stop
```

`yield` sends a value to the supplied sink.

The sink can stop traversal early, which is enough for:

```hql
stream | take(5)
```

even when `stream` is unbounded.

Lazy pipeline steps compose by wrapping sinks.

`map` transforms a value before forwarding it.

`filter` decides whether to forward it.

`take` forwards at most `n` values and then returns `Stop`.

Materialization is simply a sink that collects values into a `Vec<Value>`.

This replaces the current implicit forcing through `elements`.

## Pull is a separate feature

A pushed traversal does not need `next`.

If HQL eventually adds pure pull traversal, the natural operation is:

```hql
fn next<T>(xs: Stream<T>) : Option<(T, Stream<T>)>
```

The returned tail is a new stream value; nothing mutates.

That is fundamentally different from Java or Rust iterators, where `next` advances mutable cursor state.

HQL therefore does not need separate public `Iterable<T>` and `Iterator<T>` objects in the Java sense. `Iterable<T>` in the hierarchy describes the capability to traverse; `Stream<T>` describes an ordered traversal value. Neither is a mutable cursor.

The remaining obstacle is that HQL currently has no tuple type suitable for:

```hql
(T, Stream<T>)
```

and true pull over an unbounded generator would require evaluator suspension.

For those reasons, `next` should remain outside the initial generator design.

Push traversal is sufficient for query pipelines.

## `vault` is an ordinary bound value

The runtime exposes the current vault as:

```hql
vault : Vault
```

It is not query syntax and not a special table.

A program running without a vault simply has no `vault` binding.

A source function can consume it like any other value:

```hql
fn get_all_cards(v: Vault) : Seq<Card> {
    ...
}
```

and the standard environment can define:

```hql
cards = get_all_cards(vault)
```

Because `Seq<T>` may be lazy, this assignment does not require eager enumeration.

`cards` is therefore just an ordinary value with type:

```hql
Seq<Card>
```

## Partial application is not part of the source model

The previous design used:

```hql
cards = bind(get_all_cards, vault)
```

to explain deferred execution.

That is unnecessary.

If:

```hql
get_all_cards(vault)
```

returns a lazy `Seq<Card>`, then:

```hql
cards = get_all_cards(vault)
```

already has the required behaviour.

Partial application is still useful independently:

```hql
cards
| sort(by = partial(distance_to, here))
```

or whatever syntax the function design eventually chooses.

But that is a function-language question, not a query-source question.

It should therefore be resolved together with function types rather than making `bind` part of the sequence model. This also avoids the existing collision with "binding" generic type parameters.

## Resulting model

The final distinction is small:

```text
Iterable<T>
    can be traversed

Collection<T>
    Iterable<T>
    finite
    size and multiplicity are defined

Stream<T>
    Iterable<T>
    stable order
    may be unbounded

Seq<T>
    Collection<T> + Stream<T>
    finite and ordered

List<T>
    Seq<T>
    materialized

Set<T>
    Collection<T>
    unique values
    no positional order
```

The runtime adds one implementation concept:

```text
TraversalSource
    restartable lazy producer
    usable by either Seq<T> or Stream<T>
```

and `yield` adds one function form:

```text
iterating function
    produces values through a TraversalSource
    return type declares whether the result is Seq<T> or Stream<T>
```

This separates the concerns cleanly:

```text
Iterable    traversal
Collection  finiteness
Stream      order
Seq         finiteness + order
List        materialization
Set         uniqueness
yield       production
Traversal   runtime laziness
```

No one type has to stand for two of those ideas.

## Consequences

`cards` becomes an ordinary `Seq<Card>` bound to the current `vault`.

`take`, `drop`, and `first` operate on `Stream<T>` rather than requiring `Seq<T>`.

`size` and `count` operate on `Collection<T>` rather than `Stream<T>`.

`sort` converts a finite collection into a sequence.

`take` converts an arbitrary ordered stream into a finite sequence.

A generated finite sequence and a materialized list share the same sequence semantics without pretending to have the same representation.

An unbounded generator fits naturally without weakening the laws of `Collection<T>`.

The runtime gets one lazy traversal carrier rather than separate machinery for lazy sequences and streams.

Most importantly, **query pipelines no longer start from one privileged type**. Each operation declares the property it actually needs, and the type hierarchy expresses exactly those properties.
