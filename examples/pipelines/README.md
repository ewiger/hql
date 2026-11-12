# Pipelines: the shop

`shop/` is the vault Part II of the book runs against: a small shop whose
orders pass through files, a SQLite table, PostgreSQL, Kafka, core NATS,
NATS JetStream and a lakehouse. Its `hql.toml` declares every one of them
under an id. The files and the SQLite table are on this machine, so the
programs that read them run here. Every other endpoint names a server by an
environment variable, so a program over it is checked here, `hql check`
reading the configuration and connecting to nothing, and runs in the
[hql-demo repository](https://github.com/ewiger/hql-demo), which starts each
server with Docker Compose.

The orders are the mock events of hql-demo: ada, grace and linus place,
ship and fail to pay for six orders. `data/raw-events.json` holds eight
events as a producer sent them, four of which break the order contract.
`data/silver-replayed.json` holds the six valid events as a replay delivered
them, nine rows: two delivered twice, one corrected, one delivered late.

| Case | What it shows | Result |
| --- | --- | --- |
| [revenue from the ledger](queries/revenue-from-the-ledger.hql) | a `sqlite` table read as `Seq<Event>` and totalled per customer | each customer's total |
| [deduplication by key](queries/deduplication-by-key.hql) | nine replayed rows grouped by `id`, the latest copy of each kept by `max_by` | six events, order 6 corrected |
| [ranked deduplication](queries/ranked-deduplication.hql) | the same rows ranked by `over` within each `id`, the first of each kept | six events, in input order |
| [quarantine from a file](queries/quarantine-from-a-file.hql) | raw events validated against a `DataContract`; the rejected ones with the rules they broke | four rejected ids and their rules |
| [source of a topic](queries/invalid-source-of-a-topic.hql) | a `kafka` endpoint is read by `tap`; `source` is refused at checking | refused |
| [target of a stream](queries/invalid-target-of-a-stream.hql) | a stream has no last value for `target`; `sink` writes it | refused |

From the repository root:

```sh
cargo build --locked
cd examples/pipelines/shop
../../../target/debug/hql run ../queries/revenue-from-the-ledger.hql
```
