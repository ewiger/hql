# HQL-0055: effects and what they constrain

Companion to [HQL-0055](../README.md). A step describes computation, a stage
describes how the planner groups it, and an effect describes the
computation's interaction with the world outside the evaluation. The three
are related and none is another's alias.

## What an effect is

An effect is an observable interaction with state outside the evaluation.
`lib/std` declares every operation that has one `effectful`:

- reading: `source` and `tap` over an endpoint; `io.fs`'s `entries`, `walk`,
  `glob`, `exists` and `entry`; `io.env`'s `var`, `vars`, `arg` and `args`;
  `time`'s `now`, `today` and `local_zone`;
- writing: `target` and `sink`; the relational mutations `insert`, `update`
  and `delete` (HQL-0051); a transaction's `commit`, `rollback` and `retry`;
- emitting: the `Logger` calls `trace`, `debug`, `info`, `warn` and `error`;
- drawing randomness: `random`'s `sample`, `shuffle`, `choice`, `random_int`,
  `random_float` and `uuid`;
- encrypting: `crypto`'s `encrypt_to` and `encrypt_with_passphrase`.

An effect is not an execution result. A read's result is the data it read;
a write's result today is only that it ran.

## Where HQL already records it

- **Purity** (HQL-0001). An operation is pure or not; a user operation is pure
  only if every call in its body is pure.
- **`effectful`**. A declaration in `lib/` states that calling the function
  has an effect.
- **The checker** records, for every declared function, the effectful
  operations its body uses, directly or through another function, and `hql
  plan` prints each step's purity beside its stage.
- **Transactions** (HQL-0051). A `with transaction` scope groups the writes
  that commit together.

This record adds no mechanism. It states what a planner must read from these.

## Effect, `effectful` and the aute

- `effectful` is the declaration; the effect is what the call does when it
  runs. They are one notion from two sides.
- The aute is a different thing: a completed unit of execution, whether or
  not it carried an effect. `x = 1` is an aute with no effect; a `sink` is an
  aute with one.
- The planner's constraints below are over effectful autes. A pure aute it
  may fuse, split, duplicate or move, since the program cannot tell.

## The constraints

Each rule, then its reason.

1. **Pure steps move freely.** A planner MAY fuse, split, reorder and
   distribute pure steps within their declared shape (HQL-0004). A pure step's
   value depends on its input alone, so where it runs cannot be observed.
2. **Ordered effects stay ordered.** Two effects the program orders, one
   step before another or one statement before another, MUST happen in that
   order. Reordering two writes, or a write and the read that follows it,
   changes what the world observes.
3. **An effect is a barrier to reordering**, whatever the shape of its
   operation (HQL-0005). Pure steps MAY still fuse on either side of it.
4. **A transaction's scope is preserved.** Every write inside one
   `with transaction` scope commits or none does, however the planner splits
   the scope into stages and tasks. A backend that cannot hold the scope in
   one transaction refuses the plan rather than splitting the commit.
5. **A repeat repeats an effect.** A task MUST NOT repeat an effect it has
   completed unless the effect is idempotent, the endpoint's delivery
   semantics allow the repeat (at-least-once with a key the sink deduplicates
   on), or the repeat is a transaction's attempt, which rolls the earlier one
   back.
6. **Partitioning an effect splits its delivery.** A `sink` planned as one
   task per partition writes in parallel; the endpoint's ordering guarantee
   then holds per partition, not across them (HQL-0036).

## What an effect gives

That the statement ran is its value: `Step` (the record's "The types `Aute`
and `Step`"). What the effect did, how many rows and where, is a later and
separate question that issue 0063 keeps open. The two are kept apart so that
the type of a write can gain an outcome without the notion of an effect
changing.
