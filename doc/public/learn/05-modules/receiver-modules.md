# Receiver modules

A module that names a type pattern after `for`, `module club.titles for
String`, is a **receiver module**. Its functions take `self` as their first
parameter, at the module's pattern, and once the module is imported they
resolve as **receiver calls**, `value.f(args)`, on every value the pattern
fits. This is `lib/club/titles.hql`:

````hql
/// Receiver functions over a book's title.
module club.titles for String

/// The first letter of each word, `"The Hobbit".initials()` is `"TH"`.
fn initials(self) : String = self.words().map(w => w.slice(0, 1)).join("")

/// A shelf mark: the initials and the year, `"TH-1937"`.
fn shelf_mark(self, year: Int) : String = fmt("{}-{}", self.initials(), year)
````

`self` is written untyped: its type is the module's pattern, `String`. Inside
the module, `self.initials()` calls the module's own receiver function.

```hql
import club.titles

["The Hobbit".initials(), "Dune".shelf_mark(1965), titles.initials("Mrs Dalloway")]
```

```text
["TH", "D-1965", "MD"]
```

`value.f(args)` is `module.f(value, args)`: the value fills `self`, and
`titles.initials("Mrs Dalloway")` is the same call written with its module. A
function declared without `self` takes no receiver and is called only through
its module.

## Receiver calls throughout the book

The prelude's modules are receiver modules too: `string` is `module string
for String`, so `"Dune".upper()` calls `string.upper`, and `[1, 2].size()`
calls the collection's `size`. A receiver call is resolved by the value's
type among the modules in scope, so two modules may declare one name over
different types: `"abc".contains("b")` is the string's and
`[1, 2].contains(2)` the collection's.

`c.title` without parentheses is a field read, not a call.

## A receiver function's body

A receiver function is a function, and its body takes either form. A block
over `self` names its steps as any block does:

````hql
module counting for Int

fn twice(self) : Int {
    doubled = self * 2
    doubled
}
````

A program declares no receiver functions: `self` belongs to a module's
pattern, and a program has none.

```hql
fn initials(self: String) : String = "TH"
"The Hobbit".initials()
```

```text
syntax failure: a function with a body takes its receiver at the module's pattern: write `self` untyped
 --> <expression>:1:19
  |
1 | fn initials(self: String) : String = "TH"
  |                   ^^^^^^
```
