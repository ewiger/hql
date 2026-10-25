# Visibility

A declaration inside a module is **public** unless it is marked `private`. A
module's **contract** is its public declarations: what an importer reaches,
and what the module promises to keep.

## Public by default

`club.reading` declares `days` and `days_for_all` unmarked, so both are
public, and `rounded_up` marked `private`:

````hql
module club.reading

private fn rounded_up(n: Int, d: Int) : Int = (n + d - 1) / d

fn days(pages: Int, per_day: Int) : Int = rounded_up(pages, per_day)
````

`days` calls `rounded_up` inside the module. An importer reaches `days`, and
`rounded_up` is not part of the contract:

```hql
import club.reading

reading.rounded_up(412, 40)
```

```text
name error: the module `reading` has no function `rounded_up`
 --> <expression>:3:1
  |
3 | reading.rounded_up(412, 40)
  | ^^^^^^^^^^^^^^^^^^
```

A private declaration is never exported: no importer reaches it qualified,
unqualified, as a pipeline step, or — for a private receiver function — as
`value.f()`. A helper marked `private` is free to change without changing
the module's contract.

## What `private` marks

`private` may mark every declaration a module writes: a function, a receiver
function, a type, a trait and a module-level binding.

- A public declaration may not mention a private type or trait in its
  signature — a parameter, a result, a bound, a field or a supertype. The
  checker rejects such a declaration, since an importer would receive a value
  of a type it cannot name.
- The parser rejects `private` on an `impl`. An implementation of a trait holds in
  every module that sees the type and the trait.

## A program is public

A program is read by no other unit, so every declaration in it is public, and
the parser rejects `private` there:

```hql
private fn double(n: Int) : Int = n * 2

double(21)
```

```text
syntax failure: `private` belongs to a module: every declaration of a program is public, since no other unit reads it
 --> <expression>:1:1
  |
1 | private fn double(n: Int) : Int = n * 2
  | ^^^^^^^
```

## `internal` is reserved

`internal` is visibility across the modules of one package: wider than one
module, narrower than every importer. It is reserved, and the parser rejects it wherever
it is written:

```hql
internal fn double(n: Int) : Int = n * 2

double(21)
```

```text
syntax failure: `internal` is reserved: visibility across one package is admitted once every module records its package; mark the declaration `private`, or leave it public
 --> <expression>:1:1
  |
1 | internal fn double(n: Int) : Int = n * 2
  | ^^^^^^^^
```
