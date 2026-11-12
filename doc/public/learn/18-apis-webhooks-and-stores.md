# 18. APIs, webhooks and stores

!!! warning "Design only"

    This chapter describes functionality that is not in the current release.
    It is written as if it existed: apart from `http`, `websocket` and
    `socket`, which the [network page](09-io/network.md) of chapter 9 shows
    running, these programs do not run against the current binary, and
    names, keys, types and outputs may change before they ship.

Everything the shop has read so far, it wrote first: its own files, tables,
topics and subjects. A business also reads what others publish. The
payment provider calls the shop when a payment settles. The carrier answers
questions about parcels, a page of answers at a time. The supplier's
catalogue is a GraphQL API. And the auditor wants every order kept for
seven years, somewhere cheaper than a lakehouse.

Each of those is one more transport, and the model of
[chapter 15](15-endpoints.md) does not change for any of them: an endpoint
by id or by constructor, read by `source` if it ends and by `tap` if it does
not, written by `target` or `sink`, its type known from configuration before
anything connects.

## Three transports over HTTP

`http(url)` in the current release reads one resource as bytes, with a
format to say what the bytes are. Most APIs are more than one resource: a
collection split into pages, a query posted to one address, or a server
that calls back. Three transports of `io.net` name those shapes, and each
gives **messages**, one item per document, `Json` unless a format is named:

| Transport | What one read is | Ends | Read by |
| --- | --- | --- | --- |
| `rest(url, items, next)` | the items of a collection, every page in turn | yes | `source` |
| `graphql(url, query, variables, items)` | the nodes one query returns | yes | `source` |
| `webhook(port, path)` | the body of each request the host receives | no | `tap` |

The shapes are the same two as before. A paginated collection and a query
result are finite, read once per evaluation and replayable within it. A
webhook is live: requests arrive when the sender decides, and the stream is
as endless as a topic.

## The carrier's API

The carrier lists parcels at `https://api.carrier.example/parcels`. Each
response holds a page of them under `results`, and the `Link` header names
the next page. The shop configures it once, with the token in the
environment:

```toml
[endpoint.parcels]
kind = "rest"
url = "https://api.carrier.example/parcels"
items = "results"
next = "link"
token_env = "CARRIER_TOKEN"
```

<!-- run: skip -->
```hql
type Parcel {
    order   : Int
    status  : String
    updated : String
}

parcels : Seq<Parcel> = source("parcels")
parcels
| filter(p => p.status == "delivered")
| map(p => p.order)
```

`source` sends a `GET` for each page in turn until none is named, and gives
every item in page order. `items` is a field path into each response body,
`"data.results"` for one nested deeper; `next` is `"link"` for the
`rel="next"` target of the `Link` header, or a field path to a URL in the
body. The token is sent as `Authorization: Bearer`, and the pages are
fetched once per evaluation, so a program that reads `parcels` twice asks
the carrier once.

The parcels are now one more table to the shop. Joining them to the silver
orders of [chapter 17](17-subjects-and-the-lakehouse.md), each an `Event`
of [chapter 16](16-tables-and-topics.md), is a `join` of two `Seq`s, which
chapter 6 already showed:

<!-- run: skip -->
```hql
orders : Seq<Event> = source("orders_silver")

orders
| filter(o => o.kind == "order_placed")
| left_join(parcels, left = o => o.id, right = p => p.order,
            into = (o, p) => {id: o.id, customer: o.customer, status: p.map(x => x.status).unwrap_or("not sent")})
```

A `rest` endpoint is read only. Creating something on the carrier's side is
a request by another method, which is a write to an `http` endpoint.

## The supplier's catalogue

The supplier answers GraphQL. A `graphql` endpoint is one query: one `POST`
of the query and its variables, and the items are the array at `items`
below the response's `data`:

```toml
[endpoint.catalogue]
kind = "graphql"
url = "https://supplier.example/graphql"
query = "{ products(inStock: true) { sku name price } }"
items = "products"
token_env = "SUPPLIER_TOKEN"
```

<!-- run: skip -->
```hql
type Product {
    sku   : String
    name  : String
    price : Decimal
}

products : Seq<Product> = source("catalogue")
products | filter(p => p.price < 50.00) | map(p => p.name)
```

A response whose `errors` is not empty is an evaluation failure naming the
URL and the first error's message: a GraphQL server answers `200` for a
query it could not run, and the endpoint does not mistake that for an
empty catalogue.

## The payment provider's webhook

The payment provider does not wait to be asked. When a payment settles or
fails, it sends a `POST` to an address the shop gave it. A `webhook`
endpoint is that address, and `tap` over it is a `Stream` of request bodies:

```toml
[endpoint.payments]
kind = "webhook"
port = 8080
path = "/payments"
token_env = "PAYMENTS_TOKEN"
```

<!-- run: skip -->
```hql
type Payment {
    order  : Int
    status : String
    amount : Decimal
}

payments : Stream<Payment> = tap("payments")
payments
| filter(p => p.status == "failed")
| map(p => {id: p.order, kind: "payment_failed", amount: p.amount})
| sink("orders_raw")
```

The program listens from the first traversal until it ends, and each
accepted body is one value. The sender is answered as HTTP has it: `202`
once the body is read and decoded, `400` for a body that does not decode,
`401` for a request without the token, `404` for another path. A body that
does not decode is answered and dropped; the stream goes on, because the
sender, not the shop, made the mistake.

A webhook listens on the loopback address unless `host` says otherwise, so
a program on a developer's machine is not reachable from outside by
accident. It is the one transport that accepts connections, and like every
`Network` transport it is refused in a document being rendered.

The failed payments go onto the shop's own raw subject, where the bronze
pipeline of chapter 17 validates them with every other order. The webhook
is an edge; what comes through it joins the flow the shop already has.

## The archive

The auditor wants every raw order kept for seven years, as files, in
object storage. An object store is a transport whose documents are objects
under keys, and its module is in the `io.store` family: `s3`, which also
reaches MinIO and Ceph, `azure_blob`, `adls`, `gcs`, `sftp` and `hdfs`.

```toml
[endpoint.archive]
kind = "s3"
bucket = "shop-archive"
prefix = "orders/raw/"
region = "eu-central-1"
format = "json_lines"
credentials_env = "ARCHIVE_CREDENTIALS"
```

<!-- run: skip -->
```hql
tap("orders_raw") | collect(1000) | sink("archive")
```

`collect(1000)` turns the stream into batches, and `sink` writes each batch
as one object under the prefix, so the archive is files of a thousand
orders each rather than a million files of one. Read back, the prefix is a
finite set of objects, and `source("archive")` is a `Seq` of every order in
it: a replay of last year is a pipeline over the archive instead of over the
subject, as [chapter 19](19-oltp/index.md) shows.

A **format** says what the bytes of one object are. Beside the text formats
of chapter 9, binary formats decode bytes to values with a schema of their
own: `Parquet` for columns of a table, `Avro` and `Protobuf` for records on
a wire. An archive written as Parquet is smaller and is read by every
analytics tool there is:

<!-- run: skip -->
```hql
import io.store.s3

orders : Seq<Event> = source("orders_silver")
orders | target(s3("shop-archive", key = "orders/2026.parquet", credentials_env = "ARCHIVE_CREDENTIALS"), Parquet)
```

A **table format** is one level up: Delta Lake and Apache Iceberg keep a
table as Parquet objects and a transaction log in a store. Where the
`spark` kind of chapter 17 asks a SQL server for a table's rows, a `delta`
or `iceberg` kind reads the log and the objects itself, through the store's
transport, so a table in a bucket is readable with no server running at all.

## Further brokers and databases

The families have room for the systems a shop meets next, each a module
named for its system, each a transport with a constructor and a kind:

| Family | Systems |
| --- | --- |
| `io.msg` | `rabbitmq`, `pulsar`, `mqtt`, `redis_streams`, `sqs`, `azure_service_bus`, `artemis`, `rocketmq`, `nsq`, `zeromq` |
| `io.db` | `duckdb`, `clickhouse`, `mongodb`, `couchdb`, `cassandra`, `opensearch`, `redis`, `neo4j` |
| `io.net` | `grpc`, `feed` |
| `io.store` | `s3`, `azure_blob`, `adls`, `gcs`, `sftp`, `hdfs` |

A queue such as RabbitMQ or SQS is a `tap` and a `sink`, as a topic is; a
document database is a `source` and a `target`, as a table is. Whether a new
system ends is the one fact about it a program depends on, and the kind
states it.

## What is next

[Chapter 19](19-oltp/index.md) puts every system of Part II into one
file: the shop's whole day as a pipeline, checked before it runs, and where
that pipeline may run.
