# Channels

An endpoint of bytes needs a format to make its bytes documents. A
**channel** holds values instead. `Channel<T>(name)` is an endpoint of values
of type `T`, held in memory for one evaluation. It keeps the values' type, so
it takes no format. `target` writes one value to it, `sink` writes each value
of a sequence, and `source` gives what it holds now:

```hql
c = Channel<Int>("nums")
1 | target(c)
[2, 3] | sink(c)
source(c)
```

```text
[1, 2, 3]
```

Values are read in the order they were written. Reading does not consume:
two reads give the same values.

```hql
c = Channel<Int>("n")
[1] | sink(c)
a = source(c)
b = source(c)
[size(a), size(b)]
```

```text
[1, 1]
```

`tap(c)` gives only what is written after it, as a stream:

```hql
c = Channel<Int>("n")
[1] | sink(c)
t = tap(c)
[2] | sink(c)
t | take(5)
```

```text
[2]
```

## The element type is checked

A channel's `T` is checked as any other type is, and a format beside a
channel is a type error, since a channel holds values rather than bytes:

```hql
c = Channel<Int>("n")
"x" | target(c)
```

```text
check error[type-mismatch]: Channel<Int> holds Int, not String
 --> <expression>:2:7
  |
2 | "x" | target(c)
  |       ^^^^^^^^^
```

```hql
c = Channel<Int>("n")
source(c, Json)
```

```text
check error[type-mismatch]: a channel holds values, not bytes, so it takes no format
 --> <expression>:2:8
  |
2 | source(c, Json)
  |        ^
```

## One name, one channel

A channel lives for its execution scope: a program run, or one cell of a
document. Within a scope one name is one channel, and constructing the same
name with another element type is an evaluation failure naming both:

```hql
a = Channel<Int>("n")
b = Channel<String>("n")
1
```

```text
execute failure[invalid-argument]: the channel "n" holds Int, so it is not also a Channel<String>
 --> <expression>:2:5
  |
2 | b = Channel<String>("n")
  |     ^^^^^^^^^^^^^^^^^^^^
```

A cell of a document being rendered may write a channel, because a channel
stays inside the evaluation ([effects and what a host allows](hosts.md)). A
channel of `LogEvent` values is how a program keeps its own
[logs](logging.md) as values.
