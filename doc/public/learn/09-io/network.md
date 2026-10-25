# The network

`import net` adds endpoints that are transports. They are read and written by
`source`, `tap`, `target` and `sink` like any other endpoint, and
constructing one sends nothing. Naming the module in the program's first
lines is how a program states that it may reach the network.

`http(url)` is `Replayable`: reading it sends one `GET` per evaluation and
decodes the response body, so `source` gives a `Seq`:

<!-- run: check -->
```hql
import net
source(http("https://api.example.com/birds"), Json)
```

```text
Seq<Data>
```

`websocket(url)` is live: each message is one document, read as it arrives,
so reading it gives a `Stream`:

<!-- run: check -->
```hql
import net
tap(websocket("wss://events.example.com"), Json)
```

```text
Stream<Data>
```

Writing an `http` endpoint sends one request per document, with the document
as its body, by the `method` given, `"GET"` unless written. The response is
read only for its status, and a status that is not a success is an evaluation
failure naming the URL:

<!-- run: check -->
```hql
import net
source(file("examples/io/sightings.jsonl"))
| target(http("https://api.example.com/report", method = "POST"), Json)
```

```text
Unit
```

`socket(host, port)` is a TCP connection: its documents are separated as
their format separates them, one JSON value after another or one line at a
time.

Without the import the endpoint constructors are not in scope:

```hql
source(http("https://api.example.com/birds"), Json)
```

```text
check error[unknown-name]: `http` is provided by the `net` extension: write `import net`
 --> <expression>:1:8
  |
1 | source(http("https://api.example.com/birds"), Json)
  |        ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
```

A header is ordinary data in the program's text, so a request that needs
credentials belongs in configuration, as a source by id, and not in a program
([sources and targets by id](configured.md)). A document being rendered reaches
no network ([effects and what a host allows](hosts.md)).
