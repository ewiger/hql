# Changelog

Notable changes to HQL, newest first. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and the versions follow
[SemVer 2](https://semver.org/spec/v2.0.0.html).

The language continues to be in active development.

## [Unreleased]

### Added

- `Decimal`, an exact decimal number ([HQL-0031](doc/proposals/HQL-0031/README.md);
  [issue 0042](doc/issues/0042-decimal.md), shown in the
  [decimal examples](examples/decimal/README.md)): a number written with a
  decimal point, `12.50` or `1.5e-3`, is an arbitrary-precision `Decimal`
  whose scale is kept for display and whose equality and order are by value;
  a `Float` is written with the suffix `f`, `0.5f`, `6.02e23f` or `2f`.
  `+`, `-` and `*` are exact, `/` is exact where the quotient terminates and
  otherwise rounded half to even to 100 significant digits, and
  `round(x, places)` rounds half to even. `Decimal` is `Scalar` and
  `Orderable`, and `min`, `max`, `sum`, `product`, `average`, `abs` and
  `sign` take it. An `Int` beside a `Decimal` widens to one; a `Float` beside
  one is a type error naming `decimal(x)` and `float(d)`. `decimal`, `float`,
  `int` and `s.to_decimal()` convert. A JSON or YAML number or a CSV cell
  read as a `Decimal` keeps its digits, and a `Decimal` is written to JSON
  and YAML as a number with its scale, and to CSV and text as its digits.
  A program that passes a literal with a decimal point to a `Float`
  parameter, or mixes one with a `Float`, now writes the suffix `f`.
- Contract rules and defaults ([HQL-0032](doc/proposals/HQL-0032/README.md);
  [issue 0043](doc/issues/0043-contract-validation.md), shown in the
  [structural examples](examples/structural/README.md)): a `TypeContract` or
  `DataContract` body takes `require P` and `validate f with V` beside its
  fields, and `name: T = literal` defaults. In a rule `f?` reads an optional
  field, applying the rule only where it is present, or tests its presence
  as a condition, and `=>` is implication. `x | T` and a decoded stream
  check every rule, and a value that breaks some is an evaluation error
  naming each as written. A contract's constructor, `T(…)`, fills its
  defaults and checks its rules too. `warn P` is a rule whose violation
  leaves the value valid, and `as name` names a rule, reported as
  `Contract.name`. A default may be a list, set or `Data` literal of
  literals, never a call.

- Errors as values ([HQL-0033](doc/proposals/HQL-0033/README.md), shown in
  the [structural examples](examples/structural/README.md)):
  `x | validate(T)` reads a `Data`, a collection or a stream as `T` and
  gives a `Result<T, Invalid>` for each value instead of stopping. An
  `Invalid` holds the value and every `Violation` of it, each with its
  contract, path, kind (`missing`, `undeclared`, `type` or `rule`), rule
  identity, the rule as written and a message; a value that breaks only
  warnings is `ok`. `oks()` and `errs()` keep the successes and the
  failures of a collection of results. A `missing` or `type` violation
  fills `expected` with the type the field was read as, and a `type` or
  `undeclared` one fills `actual` with the tree the field holds;
  `Violation.severity` defaults to `"error"`, so `| Invalid` reads a stored
  `Invalid` back. `x | T` and `x | validate(T)` derive their violations from
  one reading, so the fail-fast diagnostic says each error's message.
  `oks()` and `errs()` take a `Stream` of results to a `Stream`, one value
  at a time, as they take a `List`, `Seq` or `Set` to its own kind.

- `fork`, broadcast fan-out ([HQL-0037](doc/proposals/HQL-0037/README.md),
  shown in the [dataflow examples](examples/dataflow/README.md)):
  `xs | fork { a = …, b = … }` gives every branch the same input, evaluated
  once, and its value is the record of the branches' results, typed
  `{a: A, b: B}`. `{ a, b } = xs | fork { … }` binds the branches, and
  over a `Stream` each branch is a stream in the upstream's order.
  A producer bound once is evaluated once: a `tap` of a file is read once
  and replayed for every later traversal, as standard input and sockets
  were, and a pure branch of a bound `fork` that nothing reads is not run.

- Validator types ([HQL-0034](doc/proposals/HQL-0034/README.md), shown in
  the [structural examples](examples/structural/README.md)): a trait may
  extend others, `trait Email : Validator { }`, and has their functions as
  its own. A trait extending the core trait `Validator` is a validator type.
  `validate f with V` holds where `V`'s `validate` of the field is true, and
  is a type error where the field's type does not implement `V`; a field
  declared `f: V` is of the type implementing `V` and is held to it,
  reported as the violation `Contract.f`.

- `Table<R>` ([issue 0044](doc/issues/0044-typed-tables.md), shown in the
  [constructors examples](examples/constructors/README.md)): a table is
  typed by its row, `[Book(…)] | Table` a `Table<Book>` and a table of
  `Data` maps a `Table<Data>`. `Table` written without its row is
  `Table<Data>`, and a `Table<Book>` is not a `Table<Film>`.

### Fixed

- A record declared under another record, `type Sub : Base = {…}`, has its
  parent's fields, the parent's first: `Sub(n = 1, m = 2)` builds and
  `(d | Sub).n` reads. A contract's rules read inherited fields, and a value
  read as `Sub` holds `Base`'s rules before its own.
- `Table` ([HQL-0017](doc/proposals/HQL-0017/README.md);
  [issue 0039](doc/issues/0039-table-shape-and-presentation-review.md), shown
  in the [constructors examples](examples/constructors/README.md)):
  `Table(xs, columns = [...])` builds rows from records or `Data` maps, the
  columns in the order named, or the first row's fields. Each format writes a
  `Table` with its own encoder, with no renderer named: `Text` aligned
  columns, `Markdown` and `Hmd` a pipe table, `Csv` rows header first, `Json`
  and `Yaml` an array of objects, keys in column order. A column naming no
  field of a declared record is a type error. The
  [presentation](doc/wiki/hql/presentation.hmd) card states the
  presentation model.
- Declaration visibility ([HQL-0028](doc/proposals/HQL-0028/README.md);
  [issue 0038](doc/issues/0038-import-units-and-visibility.md), shown in the
  [Mighty Types programs](examples/mighty-types/README.md)): `private` before
  a function, a type, a trait or a binding of a module makes it visible
  within that module only. No importer reaches it, by name, as a receiver
  call or as a step; a private type is named by no importer; a public
  signature that mentions a private type or trait is refused. `private` is
  refused on an `impl` and in a program, and `internal` is reserved. A
  module's public bindings are read as `m.name`.
- Typed decoding ([HQL-0021](doc/proposals/HQL-0021/README.md), accepted,
  implemented in part; [issue 0037](doc/issues/0037-io-typed-decoding.md),
  shown in the [I/O examples](examples/io/README.md)): the declared type of a
  binding or a function result decodes each document of a read as a declared
  record, `s : Seq<Sighting> = source(file("birds.jsonl"))`, a live read one
  document at a time. A CSV cell is parsed as the scalar its field declares.
  A document that does not decode is an evaluation error naming the
  endpoint, the document and the field.

- Purity is declared in HQL ([HQL-0024 part 12](doc/proposals/HQL-0024/parts/12-contracts-and-binding.md),
  [issue 0036](doc/issues/0036-contracts-receivers-purity-identities.md)):
  `effectful fn source(…)` declares an external operation effectful and a
  plain `fn` pure, and binding refuses an extension whose own marking
  differs. `std/io/mod.hql` and `std/io/logging.hql` declare `file`,
  `source`, `tap`, `target`, `sink`, `Logger` and the five log levels.
  `effectful` before a function with a body is a syntax error: such a
  function is effectful exactly when its body uses an effectful operation.

- Slices ([HQL-0026](doc/proposals/HQL-0026/README.md), shown in the
  [slicing corpus](examples/slicing/README.md) and stated in
  [the bracket operator](doc/wiki/hql/operators/bracket-op.hmd)): `xs[a:b]`,
  either bound left out, takes the part between two positions, of the
  receiver's own type, its bounds clamped so a slice never fails. `a:b` is a
  `Range`, a core record a program also builds and passes to the bracket. A
  String is indexed and sliced by character, `"hello"[1:3]`. A Stream's
  bounded slice is a List and an open one a Stream, produced lazily. The core
  trait `Slice` gives a declared type the bracket, and `Seq`, `String` and
  `Data` implement it natively.

- A type's constructor is an operation declared in HQL
  ([HQL-0018](doc/proposals/HQL-0018/README.md), shown in the
  [constructors corpus](examples/constructors/README.md)): `List`, `Set`,
  `Map`, `OrderedMap`, `SortedMap`, `Graph`, `Channel` and `Data` are
  declared as `fn` beside their types in `std/` and bound to the binary's
  constructors. A program declares its own, `fn Temp(celsius: Float) : Temp`
  beside `type Temp`, several told apart by their first parameter, called,
  piped or passed to `map`; a constructor value whose expected parameter
  fits no form or several is refused naming them. Text is parsed by
  construction: `Data(s, Json)`, `s | Data(Csv)`.

- A `Data` tree is read as a record the program declares, by its shape
  ([HQL-0016](doc/proposals/HQL-0016/README.md), shown in
  [reading records](examples/mighty-types/queries/26-reading-records.hql)):
  `d | is(Book)`, `of_type(Book)` and a `Book b` arm of `match` take a map
  with an entry for every required field at its type, ignore the entries
  `Book` does not name, and bind or keep the `Book` its constructor builds.
  A built-in record, a card kind among them, is not read from a tree.

- Generic traits ([HQL-0027](doc/proposals/HQL-0027/README.md), shown in the
  [generic traits corpus](examples/generic-traits/README.md)): a trait takes
  type parameters, `trait Index<T>`, an implementation its own,
  `impl<T> Index<T> for Ring<T>`, and a bound names a generic trait,
  `E : Index<T>`, a call filling `T` from the implementation that applies.
  One type may implement a trait at several arguments, chosen by a call's
  argument types; overlapping implementations are refused, generic or
  concrete. `Index<T>` is a core trait: `xs[i]` over a declared type is its
  `xs.at(i)`, and a sequence and Data implement it natively. The codec
  contracts are the traits `Decoder<T>` and `Encoder<T>` over the format,
  their native headers bound to the decoder and encoder tables.

- Overloaded declarations in the standard library: one name declared more
  than once in a `std/` module is one operation with several signatures, and
  a call takes the first its input and arguments fit. A bound written with
  arguments, `S : Seq<T>`, holds the input at its own type, and a constructor
  parameter, `F<_> : Iterable<_>`, rebuilds it around another element as
  `F<U>`; `Type<U>` takes a type name. The core's collection operations,
  `std/collections.hql`, are declared this way rather than described in
  comments: `take`, `sort`, `collect`, `emit`, `flat_map`, `reduce`, `count`,
  `sum`, `product` and `average` by overload, `map`, `of_type` and `somes`
  over `F<_>`, `filter` and `drop` over `S`, and `map` and `filter` over one
  `Option` and `map` over a `Result` as overloads of their own. So are `reverse`, `distinct`,
  `take_while`, `drop_while`, `min_by`, `max_by`, `with`, `without`,
  `intersection` and `difference` of `seq`, and `abs`, `pow` and `sign` of
  `math`, over an `Int` or a `Float`. `hql builtins` prints each overload.
  Every core, `seq`, `math`, `string` and `format` operation now takes its
  types from its declaration ([issue 0033](doc/issues/0033-overloaded-declarations.md),
  shown in the [overloads corpus](examples/overloads/README.md)):
  - A `/` in an external declaration ends the parameters a call supplies by
    position only, and a `*` opens those it supplies by name only:
    `count(value = 1)`, `contains(value = 1)`, `get(key = …)` and
    `pow(exponent = 2)` are refused, and `format`'s `limit` and `renderer`
    are named.
  - An overload is chosen by its value arguments' types where the input and
    the arity leave several; a call that fits no form, `2 | pow(2.0)`, is
    refused naming every form of that arity.
  - Binding refuses an overload declared after one that takes every call it
    fits.
  - `format` and its shorthands are declared in `std/format.hql`, a stream
    giving a `Stream<String>` and a collection or any other value a `String`.
  - `fmt` is declared `fmt(self, args: ...Any)`: `...Any` takes a value of
    any type in each position, where `...T` binds one type for them all.
  - A constructor parameter takes as many arguments as its bound,
    `M<_, _> : Map<_, _>`.
  - The narrowing `filter(x => x | is(U))` is a declaration whose predicate is
    a `T -> Is<U>`, binding `U` to the type tested.

- String functions in the prelude
  ([issue 0031](doc/issues/0031-missing-string-functions.md)):
  `last_index_of`, `occurrences`, `strip_prefix` and `strip_suffix` (an
  `Option`), `trim_prefix` and `trim_suffix` (the text unchanged when the part
  is missing), `is_blank`, `equals_ignore_case`, `to_bool`, `ord`, `chr`,
  `bytes` (UTF-8), `truncate` and `center`. `split` takes an optional
  `limit` that keeps at most that many pieces, the last holding the rest.

- A complete type system and the language model around it, from
  [HQL-0024](doc/proposals/HQL-0024/README.md), shown program by program in
  the [Mighty Types corpus](examples/mighty-types/README.md):
  - Records: `type Point { x : Int, y : Int }` is a type of the program,
    constructed as `Point(x = 1, y = 2)` or `Point(1, 2)`, read with `p.x`,
    generic over its fields, `Stack(items = [1])` being a `Stack<Int>`, and
    matched by type inside a union, which a `match` must cover.
  - Receiver calls: `value.f(args)` calls `f` with the value first, resolved
    among the visible modules by the receiver's type; `module.f(x)` qualifies
    one. Two modules may declare one name, `contains` of a String and of a
    collection, told apart at the call.
  - The `string`, `seq`, `math`, `option` and `result` modules of the
    prelude, which is the import list of `std/prelude.hql`: `str`, `fmt`,
    `trim`, `split`, `join`, `first`, `last`, `zip`, `enumerate`, `range`,
    `any`, `all`, `find`, `take_while`, `window`, `distinct`, `to_map`,
    `with`, `entries`, `join` of two collections, `sqrt`, `pow`, `int`,
    `float`, `unwrap_or`, `and_then` and the rest; `Pair<A, B>`; a step as a
    function value, `map(str)`.
  - `Result<T, E>`: `ok(v)` and `err(e)` as values and patterns, with
    `is_ok`, `map_err`, `and_then` and the ways out beside `Option`'s.
  - Traits: `trait Shape { fn area(self) : Float }` with default bodies,
    `impl Shape for Circle { … }` once per trait and type, `T : Shape` bounds
    checked at every call, dispatch on the value's own type; the operators
    resolve through `Add`, `Sub`, `Mul`, `Div`, `Rem` and `Eq`, and a type
    implementing `Ord` is `Orderable` for `<`, `sort`, `min` and `max`.
  - Strict traits: `std/core.hql` declares the built-in types' native
    implementations as bodyless headers, `impl Ord for Int`, so `T : Eq`
    takes `Int` and `T : Add` refuses `String`; binding refuses a header
    without a native body and a native body without a header; one
    implementation per trait and type across the standard library, imported
    modules and the program, the error naming both; an implementation is
    declared with its trait or its type ([HQL-0024 part 12](doc/proposals/HQL-0024/parts/12-contracts-and-binding.md)).
  - A standard module's `.hql` declaration is its functions' signature: an
    operation implementing one writes no type in Rust, binding derives what
    the checker uses, and an operation that keeps a native signature is
    compared with its declaration in full, so the two cannot drift. A
    declaration may write its receiver's type, `fn join(self: Collection<String>, …)`,
    and a default for an argument that may be omitted, `step: Int = 1`.
  - Modules: `import a.b as c` binds one module under a local name, builtin
    (`io.net`) or a workspace module found by its position under a module
    root, `hql.toml` `[modules] roots` or `--modules`; a module file declares
    functions, types, traits and implementations, may open with `module for
    T` to make its `self` functions receiver functions, and imports for
    itself. Imports sit at the head of a unit.
  - The `regex` module, imported where it is used: `is_match`, `find`,
    `find_all`, `captures`, `replace` and `split`.
  - Function types reach eight parameters; an argument list takes a trailing
    comma; an integer literal takes a call, `7.max(3)`.

### Changed

- The table renderers `aligned`, `pipe`, `rows` and `records` write only a
  `Table`. Named for another value, in `format(..., renderer = ...)` or in a
  vault's `[format.<Type>]`, they are refused before the program runs;
  `xs | format(Markdown, renderer = "pipe")` is written
  `xs | Table(columns = [...]) | markdown`.
- A module is a closure ([HQL-0028](doc/proposals/HQL-0028/README.md)): it
  sees its own declarations and imports and no host binding, so `vault`,
  `cards`, `graph`, `stdin` and the vault's configured imports belong to the
  program module alone. `cards = vault.cards` and `graph = vault.graph` moved
  out of `std/vault.hql` into the program module. A module that read one of
  them takes it as a parameter.
- Two imports declaring one receiver function are an ambiguity, the
  prelude's modules included: with `mighty.text` imported,
  `"   ".is_blank()` names `text.is_blank` and `string.is_blank` and is
  qualified. A unit's own declaration wins over every import's; a use above
  that declaration is refused, and a receiver call of the name of an own
  function without `self` is refused rather than reaching an import's.
- A receiver call, `value.f()`, reaches only a declaration that takes `self`
  ([issue 0036](doc/issues/0036-contracts-receivers-purity-identities.md),
  shown in the [receivers corpus](examples/receivers/README.md)). The core's
  collection operations, `math`, and `seq`'s map and set functions write
  their receiver, `fn size<T>(self: Collection<T>)`, `fn abs(self: Int)`, so
  `xs.size()` and `42.abs()` still call them; `1.range(3)` and
  `"ab".is_match("a")` are refused naming the call and pipeline forms, which
  still reach them.

- A negative position counts from the end: `xs[-1]` is the last element, not
  absence, and `s.slice(start, end)` counts a negative bound from the end
  ([HQL-0026](doc/proposals/HQL-0026/README.md)). An index that is neither an
  `Int` nor a `Range` is refused as "an index is an Int position or a Range".

- A generic variadic parameter binds its type from the trailing arguments:
  `fn pack<T>(xs: ...T)` makes `pack(1, 2)` a `List<Int>`, and refuses
  `pack(1, "owl")`, whose arguments share no declared ancestor.

- A step over a value applied to an optional input is lifted over it
  ([issue 0033](doc/issues/0033-overloaded-declarations.md)): `none | abs`,
  `none | sqrt` and `none | upper` check and evaluate to absence, and
  `some(-2) | abs` is an `Option<Int>`. A step over a collection still takes
  absence as the empty collection.
- `collect(for = …)` is refused as an argument `collect` does not declare.

- `string.replace` and `regex.replace` are now `replace_all`, and each has a
  `replace_first` beside it, so a replacement names whether it applies to
  the first occurrence or to every occurrence.

- A format is a `Format`, a category of its own, not a `String` subtype
  ([HQL-0024 part 7](doc/proposals/HQL-0024/parts/07-formats-and-codecs.md),
  from [encode, decode and format](doc/proposals/HQL-0024/encode-decode-format.md)).
  `format(x, Json)` and its shorthands give a `String` that keeps its format
  for presentation, so `xs | json` is a `String` and encoding it again in
  another format encodes that text. `Lines` and `JsonLines` are framing
  formats: `source(file(p), Lines)` is a `Seq<String>` and `source(stdin,
  JsonLines)` a `Stream<Data>`; `.jsonl` and `.ndjson` imply `JsonLines`.
  `std/format.hql` declares the codec contracts `Decoder<F, T>` and
  `Encoder<T, F>`. `Text` is one whole document; a `.txt` or `.log` path
  implies `Lines`; and `lines` and `jsonlines` encode one record per line.
  `sink` writes a stream, one document per value, and `target` one whole
  document, as before. The Mighty Types corpus
  shows `source(file, Lines)`, `target(file, Lines)` and
  `sink(file(p).end, Lines)`.
- `import` names a module, not an extension; `import nonesuch` is refused as
  "no module named".
- The binary runs on a thread with a large stack, so a program may recurse
  thousands of calls deep.

- Endpoints, from [HQL-0021](doc/proposals/HQL-0021/README.md): `stdin`,
  `stdout` and `stderr`, `file(path)`, and a file's `.end`. `source(e, F)`
  reads an endpoint's documents in a format, as a `Seq` from a file and as a
  `Stream` from standard input; `tap` reads them one at a time, and at a
  file's `.end` follows what is appended. `target` writes one completed value
  and `sink` one document per value; a file is replaced, atomically for
  `target`, and its `.end` appended to. A literal path's extension implies the
  format. `hql render` reads files beside the document and refuses every
  write.
- Channels: `Channel<T>(name)` is an endpoint of values held in memory for one
  evaluation. `target` and `sink` append to it, `source` gives what it holds
  now without removing it, and `tap` what is written from then on. A document
  may write its own channels.
- Logging, with `import logging`: `Logger(endpoint, Format, name = …, level =
  …)` writes `LogEvent`s as `sink` would, one document per event, or the
  events themselves to a `Channel<LogEvent>`. `trace`, `debug`, `info`, `warn`
  and `error` write one event when its level reaches the threshold, and
  `--log-level` overrides every threshold. A level is a type, tested with
  `e.level | is(Error)`. `hql render` binds `cell.logs`, a cell's own channel,
  and never saves it into the document.
- Network endpoints, with `import net`: `http(url, method = …, headers =
  …)`, `websocket(url)` and `socket(host, port)`. `source` over `http` sends
  one `GET` per evaluation and reads the body as a `Seq`; a write sends one
  request per document and fails on a status that is not a success, naming
  the URL. A WebSocket message is one document, and a socket's documents are
  separated as their format separates them. `hql render` refuses every
  network endpoint at check time, and `file("https://…")` is refused, naming
  `http`.
- A function whose body uses an effectful operation, or calls an effectful
  function, is effectful, and `hql check` reports it with what it uses:
  ``load is Effectful: it uses `source` ``.
- `hql run -` refuses `source(stdin, …)` and `tap(stdin, …)` at check time:
  standard input holds the program.
- A channel holds at most a million values in one scope; one more ends the
  evaluation, naming the channel.
- Indexing: `xs[i]` reads position `i` of a sequence, counting from `0`, as
  an `Option`; past either end it is `none`. A `Data` tree that is a sequence
  is indexed too, and a `Set`, a `Map` or an unbounded stream is refused.
- `hql '<program>'` is `hql eval '<program>'`.
- `--compact` writes each JSON document on one line, as `jq -c` does.
- In HQL source a document reference is written `@[[name]]`, from
  [HQL-0023](doc/proposals/HQL-0023/README.md): `@` directly before a
  wikilink. `[[` is two brackets, so `[[1, 2], [3]]` is a list of lists. A
  bare `@` is refused, and `[[alice]]` with `alice` unbound is refused,
  naming `@[[alice]]`. Document bodies keep writing `[[name]]`, and a link
  inside a fenced code block or an inline code span authors no edge.
- Without `--vault`, `hql` finds the vault itself: the nearest directory at or
  above the current one that holds `.hmd/` or `hql.toml`. `--vault` still
  overrides it, and outside every vault a program runs with no vault.

### Removed

- The core `debug` step, which captured a value into the run's report queue.
  `debug` is now the logging operation; a program reports on itself through a
  `Logger`.

### Changed

- A stream written with `--to json` pretty-prints each document, as `jq .`
  does; concatenated, they read back as the same stream.
- A result of type `Unit` prints nothing, so a program that ends in a write
  is silent.
- A first argument that names no command is a program, not a usage error.
- Every streamed CSV piece ends its lines with `\n`, the header's included.

- A `Data` tree that is one string is written as its bare text in `Text`, as
  a `String` is, so `map(e => e.message) | sink(stdout, Text)` writes lines.

- `hql run` writes an unformatted result as `Hql` too, so every command that
  evaluates writes the same text for the same value. `--to text` gives a bare
  string.

## [0.4.2] — 2026-10-19

HQL output that reads back as HQL, and a build floor that holds.

### Added

- **An `Hql` presentation format**, with the shorthand `hql` and `--to hql`: a
  value written as the literal that writes it, so the text reads back as the
  same value. A string is quoted even when it stands alone.

### Changed

- `hql eval` and the REPL write an unformatted result as `Hql`, so
  `hql eval '"Dune"'` prints `"Dune"`. `hql run` still writes plain text, and
  `--to text` gives a bare string anywhere.
- **Rust 1.95 or newer is required to build HQL.** The line editor,
  rustyline 19, needs it, so `rust-version` is now 1.95; 0.4.1 declared 1.88
  and did not build on it.

## [0.4.1] — 2026-10-19

A REPL that works on a terminal, and the HQL book.

### Fixed

- **The REPL.** A terminal session edits the line and recalls earlier inputs
  with the arrow keys, from a history kept in `~/.hql_history`. An input goes
  on over several lines while a bracket is open, or while the next line opens
  with `|`, so a multi-line literal or a pipeline written one step per line
  runs as one input, typed or piped. Ctrl-C abandons the input being typed;
  Ctrl-D leaves. A session keeps every declaration — a binding with a lambda in
  it, an annotated binding, `fn`, an alias — where it used to drop some.
- A record literal opening a line inside a list, `[\n {title: …}]`, was read
  as a set.
- `format(…, limit = n)` ignored its limit over a sequence the vault produced,
  such as `cards | map(…)`.
- A collection printed as text wrote its strings without quotes, so
  `["a, b"]` read as two elements, and wrote a set in square brackets like a
  list. A value now prints as the literal that writes it: `["a, b"]`,
  `{1, 3, 2}`. Only a string standing alone prints bare.

### Added

- **The HQL book.** `doc/public/learn/` teaches the whole language in twelve
  chapters, from a first value through records and lists, SQL-style
  questions, a vault of documents, graphs, retrieval, streams and formats, to
  documents built from their cells. Every program in it runs against the new
  tutorial vault, `examples/tutorial/`, and `tests/tutorial.rs` checks that
  each writes what the book shows.

## [0.4.0] — 2026-10-19

Generic functions and aliases, the reduction vocabulary, graph
construction, a naming lint, and the open ends of the proposals.

### Changed — breaking

- A `fn` or alias named in `UpperCamelCase` is refused: that casing names a
  type's constructor, which only the module declaring the type provides.
- `graph` is bound to the vault's graph, `vault.graph`, beside `cards`.
- The grammar notation in the expressions model is `::=`, since `:=` is now
  HQL syntax.

### Added

- **Generic functions.** `fn first<T>(xs: List<T>) : Option<T>`; inside the
  body `T` is a type of its own, and `T : Orderable` lets the body order. Each
  call binds `T` from its arguments and checks the bound, naming the parameter.
- **Aliases.** `name(p, ..) := body` defines a function whose types the
  checker writes: a fragment body takes the most general input it accepts, a
  parameter the type of the step argument it fills. `hql check` prints each
  signature.
- **Combiners and aggregations.** `min(a, b)`, `max(a, b)`, `merge`, `union`
  and `concat`; `reduce(0, +)`, `reduce(max)` and `reduce({}, merge)`, a fold
  whose combiner ignores order accepting any collection; `product`, `average`,
  `merge_all`, `union_all` and `concat_all`.
- **Graph construction.** `Graph(edges)` and `Graph(nodes, edges)`; a vault's
  `name`, `root` and `graph`.
- **Constructors as values.** `groups | map(Set)`.
- **Check-time warnings and the naming lint.** A name off its kind's casing is
  a warning at the name; a type test that always holds is a warning.
- **Type interpretation.** `is(List)` over a bare constructor; `filter` over an
  `Option`, narrowing it; a refused annotation names `of_type`, the constructor
  or `sort`.
- **Diagnostics.** A whole-input step over a stream names `take(n)` and
  `collect(n)`; `collect(for = …)` says time windows are not implemented; an
  unknown renderer is refused before the program runs; a bare `vault`, `cards`
  or `graph` without a vault names `--vault`.

### Fixed

- `[] | sum` is the `Int` `0`.
- A type parameter first bound narrower, as by an empty list in
  `reduce([], concat)`, widens to what it meets later.
- A comparison with `Never`, the element of nothing, checks against any
  orderable side, so `none | filter(x => x > 0)` checks.

## [0.3.0] — 2026-10-19

Cells: the HQL blocks of a document share values by id and run in dependency
order, and each saved output records the code it came from.

### Added

- **Cells.** A cell's info string takes `id=<name>`, `replace`, `after` and
  `hidden`. A cell with an id exports its value to every other cell of the
  document and keeps its own bindings. The document is checked as a whole and
  evaluated in dependency order, document order breaking ties; a cycle and a
  duplicate id are errors naming the cells, and a cell waiting on a failed one
  is skipped. An output under `after` records the cell's id, a hash of its code
  and of the cells it reads, and its status. `hql render --cell <id>` runs one
  cell and what it reads; `--envelope json` prints the document and a record
  per cell.

### Fixed

- A cell's reports point at its place in the document, not at a line of the
  cell read as though it were the document.

## [0.2.0] — 2026-10-19

Functions as values, streams and generated sources, aggregation, and
presentation formats. `cards` becomes a lazy sequence, and presentation moves
from terminal presenters to formats a program can keep computing with.

### Changed — breaking

- `cards` is the standard-library binding `cards = vault.cards`, a lazy
  `Seq<Card>` in the order a sorted walk of the vault reaches each card: the
  entries of each directory in name order, depth first. It was a `Set<Card>`.
  A pipeline over `cards` is a sequence: `filter`, `sort` and `map` keep a
  `Seq`. The host seeds `vault`, of type `Vault`; without a vault, neither
  `vault` nor `cards` is bound.
- The `present` extension, the `Presentation` type and the `table` step are
  gone. `text`, `markdown` and `json` are shorthands of `format`, and their
  results are text a later step reads. `table` is an unknown step whose
  diagnostic names `Table`, which arrives with record values.
- The `graph` step is the `Graph` constructor: `Graph(xs)` or `xs | Graph`.
  `cards | graph` is refused naming `Graph`. `expand(depth = 0)` is refused.
- `--format text|json` is `--envelope text|json`, the CLI's output format.
- A hit's retrieved value is `h.value`; reading `h.card` names `value`.
- `map` over a `Set` returns its image, a `Set`, so equal results collapse.
- `typed` is `of_type`.
- Plain text writes a value as its own rendering, `[a, b]`, not an aligned
  table. `--to markdown` or `format(Markdown, renderer = "pipe")` gives a table.
- A graph's fields are `nodes : Set<Card>` and `edges : Set<Edge>`.
- `std/` states the fields the binary reads: `Hit` and `Ranking` hold
  `score : Float`, `query : String` and `retrieval : Data`; `Doc` declares
  `title` and `body : String`; `Card` declares `kind`. `Score`, `Query`,
  `Retrieval`, `Content` and `HmdContent` are gone.

### Added

- **Function values.** Function types written with arrows, `Int -> Int`;
  `fn` declarations, recursive and variadic (`xs: ...Int`); lambdas as values,
  typed by annotation or by their binding; functions passed and returned.
  Parameters are contravariant and results covariant.
- **Partial application with a hole.** `add(10, _)` is `Int -> Int`: each hole
  takes the type of the parameter it fills.
- **Branching and absence.** `match` with value, type and `some`/`none` arms,
  checked for exhaustiveness; `value given condition otherwise other`; `some`,
  `none` and `somes`.
- **Asking what a value is.** `is(T)` and `of_type(T)`; a `filter` by `is`
  narrows the element type; a `Data` leaf reads as `Int`, `String` and the
  rest, so a metadata field can be summed.
- **Aggregation.** `group`, `reduce`, `sum`, `min` and `max` over finite
  inputs. One signature per name, printed by `hql builtins` one line per
  overload.
- **`tee`**, which binds a side branch of a pipeline and passes its input on.
- **Streams.** `Stream<T>` is a type, and `Seq<T>` a stream that ends.
  `map`, `filter`, `take` and the new `drop` run lazily over a stream; a step
  that needs its whole input refuses one, naming `take(n)`. `collect(n)`,
  `emit` and `flat_map` batch and unbatch; `List(xs)` refuses a stream and a
  set.
- **Generated sources.** A `fn` with a block body yields its values, directly
  or in a `for` loop, and returns `Seq<T>` or `Stream<T>`. Calling it runs
  nothing; `take` ends a stream that would not end by itself.
- **Constructors as pipeline steps.** `xs | Set` is `Set(xs)`.
- **Presentation formats.** `Text`, `Markdown`, `Hmd`, `Json`, `Yaml`, `Csv`,
  `D2` and `Mermaid` are subtypes of `String`. `format(x, F)` with `limit` and
  `renderer`, and the shorthands `text`, `markdown`, `hmd`, `json`, `yaml`,
  `csv`, `d2` and `mermaid`. Graph renderers write adjacency text, node-link
  JSON, D2 and Mermaid, each node saying why it is present. A vault chooses
  renderers under `[format.Type]` in `hql.toml`. Over a stream, `format` gives
  a stream of pieces.
- **The presentation context.** `--to <format>` writes a result the program did
  not format; `--to none` evaluates and writes nothing. `hql render` writes
  each cell as `hmd`. A stream result is written piece by piece as it arrives,
  and ends when its reader stops reading.
- `hql doctor` checks the semantic-search chain — vault, `hql.toml`, index,
  model, query vectors, card coverage — and names the first missing part.
- `hql render` builds a Markdown document from its cells. `--emit replace`, the
  default, writes each cell's output in its place; `--emit after` keeps the
  cell and writes the output after it between `<!-- hql:output -->` markers.
  `--write` takes the output path, and `--in-place` rewrites the source under
  `--emit after`.
- Scalar operators: `<`, `<=`, `>`, `>=` over values of one orderable type;
  `and`, `or`, `not` over `Bool`, short-circuiting; `-`, `*`, `/`, `%` over
  `Int` or `Float`, with truncating `Int` division. Division by zero and `Int`
  overflow are runtime errors. A comparison does not chain.
- The `debug` step: an identity step that records its input, a label and its
  trace in the report log at severity `debug` or `info`. `--report-level`, the
  vault's `[report] level` and `$HQL_REPORT_LEVEL` set which reports the text
  output shows; JSON output carries all of them.
- `std/` is embedded in the binary, and a built-in record's fields are read
  from its declaration.
- `Ranking<T>` is a declared subtype of `Seq<Hit<T>>` over any element type.
- Corpora: `examples/functions/`, `examples/streams/` and
  `examples/mapreduce/`, every case checked by `tests/corpus.rs`.

### Fixed

- `cards = [1, 2]` followed by `cards` evaluated to the vault's cards and
  ignored the binding without a report.
- `1 - 2` was a syntax error: a minus after an operand is subtraction, and only
  a minus before a digit with no operand ahead of it is a literal's sign.
- A step written inside a pipeline reports its own span, not the whole
  pipeline's.
- A wikilink's display text is not part of its target.
- Equality looks through subtyping, so `Json` text compares with a string.
- CSV lays a list of records out in one column per key.

## [0.1.0] — 2026-09-27

The first release: a typed expression core, a vault of Markdown and
HyperMarkDown documents, pipelines over its cards, two named retrievals, and
graph traversal that records why each node is present.

### Language

- A typed expression core — scalars, collection literals, lambdas, field access
  and pipelines — checked before it is evaluated.
- `type` declarations parsed and checked, so `std/` is read rather than assumed:
  `core`, `collections`, `doc`, `graph`, `knowledge` and `retrieval`.
- `Scalar` as a type, `Data` as a union, and a collection vocabulary settled on
  order: a sequence has one, a set does not, and `take` needs elements that
  carry their own.
- `:` as the subtype operator, replacing `<:`.
- A collection literal evaluates at its checked element type, and an unrelated
  tree is held as `Data`.
- `Hit` and `Ranking` carry the retrieval that produced them — query, index,
  model, model revision, metric, and whether the search was approximate.

### Retrieval and graph

- `lexical`, which matches shared spellings offline with nothing to build.
- `semantic`, which scores against vectors computed ahead of time;
  [`contrib/semantics/`](contrib/semantics/README.md) produces the index and the
  binary only reads it.
- Which model fills that index is the vault's choice. `[semantics.embedder]` in
  `hql.toml` declares one of two providers — `local`, a checkpoint in the
  producer's own process, or `http`, an OpenAI-shaped `/v1/embeddings` endpoint
  covering a hosted provider and a model server on `localhost` alike — and every
  `build` flag overrides it field by field. A key is named as an environment
  variable and never written into the committed file, and `http` needs an
  explicit `revision`, because an endpoint offers nothing to derive the pin a
  score claims. The producer gains no dependency: `http` uses the standard
  library, and indexing through one needs no `torch`.
- Scoring checks the precondition it rests on. A dot product is cosine only for
  unit-length vectors, so the producer normalises whatever an embedder returned,
  the index refuses to be written holding a vector that is not unit-length, and
  the reader refuses an index declaring `normalized = 0` or a metric it does not
  compute. Truncated embeddings are why this cannot be implicit: a prefix of a
  unit vector is not a unit vector.
- `expand`, which traverses outwards and records the reason each node is
  present, so a graph of matches and their neighbours does not claim that all of
  them matched.
- Links traverse as uplinks and downlinks.

### The command line

- `eval`, `check`, `run`, `repl`, `render`, `builtins` and `config`. `-` reads
  standard input wherever a file is taken.
- `--format json` emits the type, the value and the report queue for another
  program.
- `hql render --write` runs the `hql#eval` blocks in a document and writes each
  answer beneath it as an `hql#result` block, replacing the previous one.
- Reporting yields a value *and* a queue: `--report strict` stops at the first
  error and is the default, `--report collect` reports every error findable. The
  mode comes from the query, then the vault's `hql.toml`, then `$HQL_REPORT`,
  then the default.
- Exit `0` on success, `1` on a language or I/O failure, `2` on a usage error.
  Diagnostics carry zero-based UTF-8 byte ranges and render with a line, a
  column and a caret.

### Extensions and execution

- An extension mechanism with declared contracts, and a checked execution tree
  with structural step contracts.
- MapReduce pipeline stages with pluggable execution backends.

### Corpora

- `tests/fixtures/birds/`: fifty-six encyclopedia articles with an embedding
  index committed beside them.
- [`examples/birds/`](examples/birds/README.md): linked HyperMarkDown cards with
  runnable collection queries, including intentional errors for duplicate keys
  and non-orderable sorted-map keys.

[Unreleased]: https://github.com/ewiger/hql/compare/v0.4.2...HEAD
[0.4.2]: https://github.com/ewiger/hql/compare/v0.4.1...v0.4.2
[0.4.1]: https://github.com/ewiger/hql/compare/v0.4.0...v0.4.1
[0.4.0]: https://github.com/ewiger/hql/compare/v0.3.0...v0.4.0
[0.3.0]: https://github.com/ewiger/hql/compare/v0.2.0...v0.3.0
[0.2.0]: https://github.com/ewiger/hql/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/ewiger/hql/releases/tag/v0.1.0
