# How names resolve

Every module is an **import unit**: one scope in which its imports take effect
and its names resolve. A program is an import unit of its own, the **program
module**. An import makes a module visible in the importing unit and nowhere
else.

## Imports are per unit

A module's imports are its own. A module that imports `math` sees `math`, and
a program that imports the module does not see `math` through it. The
prelude, too, is the implicit import list of each unit separately.

## A unit's own names win

A declaration the unit writes itself wins over an imported declaration of
the same name, in every spelling: an unqualified call, a pipeline step and a
receiver call. The imported one stays reachable through its module. This
program declares its own `lower`:

```hql
fn lower(s: String) : String = "mine"
[lower("ABC"), "ABC" | lower, string.lower("ABC")]
```

```text
["mine", "mine", "abc"]
```

The program's `lower` takes no `self`, so the receiver spelling reaches
nothing, and the prelude's `lower` is not reached in its place:

```hql
fn lower(s: String) : String = "mine"
"ABC".lower()
```

```text
check error[unknown-name]: `lower` is this unit's own function, which takes no `self`, so `value.lower(…)` does not reach it and no import's `lower` over String is reached in its place; write `lower(value, …)`, or qualify the import's
 --> <expression>:2:1
  |
2 | "ABC".lower()
  | ^^^^^^^^^^^^^
```

A unit owns its own names, so a module that later gains a declaration of the
same name cannot take one away from it. Which declaration wins does not
depend on the order the declarations are written in.

## Two imports, one name

Two imported declarations of one name, neither the unit's own, are an
**ambiguity**, and the call naming them is a name error. Qualifying the call
resolves it, `titles.initials(s)` or `string.split(s, ",")`; the order of the
imports never decides.

## A module is a closure

A free name in a module resolves through the module's own declarations and
its own imports, and through nothing else. A module never sees the program
module, and no module imports it. The bindings the host seeds into a program
— `vault`, `cards`, `stdin`, `stdout` and the rest — are not bound in a
module. What a module needs from a program reaches it as a parameter, a
function argument or a trait bound:

````hql
module club.shelf

fn books(cards: Seq<Card>) : Seq<Card> = cards | filter(c => c.metadata.category == "book")
````

A program calls it with its own cards, `shelf.books(cards)`. A module
written this way checks once, and means the same in every program that
imports it.

## Imports form no cycle

The import graph is acyclic: an import that closes a cycle is a name error,
and the diagnostic names the cycle. The module name tree is not a dependency graph —
`club` and `club.reading` see nothing of each other unless one imports the
other. A child may import its parent; a parent importing its child is
warned at the import. Two modules that need each other are resolved by moving
the shared part into a module both import.
