# 5. Modules

A **module** is a named unit of declarations — functions, types, traits and
bindings — kept in a file of its own. A program reaches a module's
declarations by importing it. This page writes a module and imports it; the
pages after it cover receiver modules, visibility, and how a name resolves.

## A module is a file under a module root

A workspace module is an HQL file under a **module root**, named by its
position under the root: `lib/club/reading.hql` is the module `club.reading`,
and `lib/club/mod.hql` would be `club`. The roots come from `hql.toml`:

```toml
[modules]
roots = ["lib"]
```

or from `--modules lib` on the command line. The tutorial vault configures
`lib/` as its root, and keeps the book's modules there.

Every module file opens with its name, which is checked against the file's
place under its root. This is `lib/club/reading.hql`:

````hql
/// Reading plans: how long a book takes at a number of pages a day.
module club.reading

private fn rounded_up(n: Int, d: Int) : Int = (n + d - 1) / d

/// Days to read `pages` at `per_day` pages a day; a part day counts as one.
fn days(pages: Int, per_day: Int) : Int = rounded_up(pages, per_day)

/// Days to read every book of `shelf`, one after another.
fn days_for_all(shelf: List<Int>, per_day: Int) : Int {
    each = shelf | map(pages => days(pages, per_day))
    each | sum
}
````

A `///` line is a doc string: it documents the declaration after it, and
`hql help club.reading` prints it. `private` keeps `rounded_up` inside the
module, which [visibility](visibility.md) covers.

## Importing a module

`import club.reading` makes the module visible under its last segment,
`reading`, and its functions are called through that name:

```hql
import club.reading

[reading.days(412, 40), reading.days(280, 40), reading.days_for_all([412, 280, 310], 40)]
```

```text
[11, 7, 26]
```

`import a.b as c` makes it visible under `c` instead:

```hql
import club.reading as plan

plan.days(412, 40)
```

```text
11
```

A module's function is a value like any other, and takes holes:

```hql
import club.reading

[412, 280] | map(reading.days(_, 40))
```

```text
[11, 7]
```

Imports sit at the head of a source unit, before its declarations and
expressions. Two imports may not share a local name:

```hql
import club.reading
import club.titles as reading
1
```

```text
check error[unknown-name]: `reading` already names the module `club.reading` here; import `club.titles` under another name with `as`
 --> <expression>:2:1
  |
2 | import club.titles as reading
  | ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
```

A module that is not imported is not visible, however it is spelled:

```hql
reading.days(412, 40)
```

```text
check error[unknown-name]: `reading` is not bound
 --> <expression>:1:1
  |
1 | reading.days(412, 40)
  | ^^^^^^^
```

## The standard modules

The standard library is modules of the same kind. The **prelude** is the
list of modules every program imports without writing it: `string`, `seq`,
`math`, `option` and `result`, among others. `"a,b".split(",")` calls the
prelude's `string.split`. Other standard modules are imported by name:

```hql
import regex

regex.split("a1b22c", "[0-9]+")
```

```text
["a", "b", "c"]
```

The [reference](../../reference/index.md) has a page per standard module.

## The pages of this chapter

| Page | What it covers |
| --- | --- |
| [Receiver modules](receiver-modules.md) | `module m for T`, functions taking `self`, and `value.f()` calls |
| [Visibility](visibility.md) | public and `private` declarations, and a module's contract |
| [How names resolve](names.md) | import units, a unit's own names, ambiguity, and modules as closures |

## What is next

With functions and modules in hand, [chapter 6](../06-sql-like-queries.md)
asks the questions SQL asks: where, order by, group by, and the aggregates.
