# Function bodies

A function's body takes one of two forms: an expression after `=`, or a
**block** in braces, written without `=`. The two forms are equally general:
any function may take either, at any result type, and both declare the same
function.

```hql
fn doubled_sum(xs: List<Int>) : Int = xs | map(x => x * 2) | sum

fn doubled_sum_block(xs: List<Int>) : Int {
    xs
    | map(x => x * 2)
    | sum
}

[doubled_sum([1, 2, 3]), doubled_sum_block([1, 2, 3])]
```

```text
[12, 12]
```

A pipeline is an expression, so `= xs | f | g` binds the whole pipeline. In a
block, a pipeline continues over several lines, each opening with `|`. A body
longer than one line is written as a block.

## The value block

A **value block** is zero or more bindings, `name = expr`, one per line,
followed by one final expression. The final expression is the function's value
and has the declared result type:

```hql
fn label(pages: Int) : String {
    days = (pages + 39) / 40
    kind = "short" given pages < 200 otherwise "long"
    fmt("{} ({} days)", kind, days)
}

[145, 412] | map(label)
```

```text
["short (4 days)", "long (11 days)"]
```

Each binding names one step of the computation. The steps may call other
functions, and the final line combines them:

```hql
fn area(width: Float, height: Float) : Float {
    w = width
    h = height
    w * h
}

fn triangle(base: Float, height: Float) : Float {
    whole = area(base, height)
    whole / 2.0f
}

[area(2.0f, 3.0f), triangle(5.0f, 5.0f)]
```

```text
[6.0, 12.5]
```

A value block takes any result type. A block returning `Seq<Int>` without
`yield` is a value block, and its last line is the sequence it returns:

```hql
fn even_squares(xs: List<Int>) : Seq<Int> {
    evens = xs | filter(x => x % 2 == 0)
    evens | map(x => x * x)
}

List(even_squares([1, 2, 3, 4]))
```

```text
[4, 16]
```

## A block's scope

A binding is in scope from the line after it to the end of its block, and
nowhere else. A binding inside a block shadows a name outside it without
changing that name. A lambda on the final line captures the block's bindings,
and takes its parameter type from the declared result:

```hql
k = 100

fn adder(n: Int) : Int -> Int {
    k = n * 10
    x => x + k
}

add = adder(2)
[add(1), add(5), k]
```

```text
[21, 25, 100]
```

A block's bindings are not visible after the function:

```hql
fn area(w: Float, h: Float) : Float {
    a = w * h
    a
}
a
```

```text
check error[unknown-name]: `a` is not bound
 --> <expression>:5:1
  |
5 | a
  | ^
  = help: did you mean `abs`?
```

## The iterating block

A block containing `yield` is an **iterating block**. It holds bindings,
`yield expr` and `for name in collection { … }`, one per line, and produces its
values in the order each `yield` is reached. Its result type is `Seq<T>`,
which ends, or `Stream<T>`, which need not, and each yielded value is a `T`:

```hql
fn countdown(from: Int) : Seq<Int> {
    for n in range(0, from) {
        yield from - n
    }
}

List(countdown(3))
```

```text
[3, 2, 1]
```

[Chapter 10](../10-streams.md) is about iterating functions and the streams
they produce.

## What a block may not be

A block ends either in a final expression or in yields, never in both and
never in neither. The parser rejects each of these as a syntax failure.

A block of bindings alone has no value:

```hql
fn one() : Int {
    x = 1
}

one()
```

```text
parse failure[unexpected-token]: a block ends in a value or yields; this one holds only bindings
 --> <expression>:1:16
  |
1 | fn one() : Int {
  |                ^^^^^^^^^^^^^
```

A block that yields holds no expression on a line of its own:

```hql
fn numbers() : Seq<Int> {
    yield 1
    2
}

List(numbers())
```

```text
parse failure[unexpected-token]: a block that yields holds no expression on a line of its own
 --> <expression>:3:5
  |
3 |     2
  |     ^
```

`for` belongs to an iterating block:

```hql
fn total(xs: List<Int>) : Int {
    for x in xs {
        y = x
    }
    0
}

total([1, 2])
```

```text
parse failure[unexpected-token]: `for` belongs to an iterating block; a value block is bindings and a final expression
 --> <expression>:2:5
  |
2 |     for x in xs {
  |     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
```

`= {` begins a record or set literal, never a block, so a binding inside it
does not parse:

```hql
fn one() : Int = {
    x = 1
    x
}

one()
```

```text
parse failure[unexpected-token]: expected `}` after collection elements, found `=`
 --> <expression>:2:7
  |
2 |     x = 1
  |       ^
```

## Wherever a function is declared

A function has one shape wherever it is declared, and its body takes either
form in each place: at the top level of a program, in a module
([chapter 5](../05-modules/index.md)), as a receiver function taking `self`
([receiver modules](../05-modules/receiver-modules.md)), and as a trait's
default body or an implementation's function
([traits](../17-types-and-traits/traits.md)).

A lambda's body stays one expression; a function that needs a block is
declared with `fn`.
