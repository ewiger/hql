# 20. OLAP

[Chapter 19](../19-oltp/index.md) was the shop's transactions as they
happen. This chapter is what they leave behind. The owner does not ask the
order topic a question; the owner asks how much each customer spent this
month, which day of the week sells most, and whether refunds are rising.
Those are questions over **history**, over every order rather than the next
one, and the tables they are asked of are the shop's **OLAP** side: online
analytical processing, the warehouse, the lakehouse, the place a report
reads.

The two sides differ in almost every way that matters to a pipeline, and a
platform that serves both, as Databricks does, keeps them apart on purpose:

| | OLTP | OLAP |
| --- | --- | --- |
| What a row is | one fact as it happened: an order, a payment | one fact at a chosen **grain**: an order, a customer-day, a customer |
| What the table holds | the current state | the history |
| How it is written | one row at a time, as the fact occurs | in batches, by a load that runs on a schedule or as a stream lands |
| How it is read | one row, by key | many rows, by scan, aggregated |
| The shape of its pipeline | `tap`, decide each value, `sink` | `source` whole, `group` and `sum`, `target` |
| What goes wrong | a row that does not decode | a row that arrives twice, late, or corrected |

The last line is the subject of most of this chapter. A transactional
pipeline delivers each message at least once, and a message delivered twice
into a transactional table is a duplicate row to be found later. In an
analytical table a duplicate is a wrong total in a report, and the pipeline
that loads the table is where it is removed. Every load of an analytical
table therefore answers three questions: what a row stands for, which rows
are the same row, and which of the same rows is the one to keep.

In the terms of [chapter 14](../14-etl-pipelines.md), an analytical load is
usually a batch: a `source` that ends, whole-input steps over it, a `target`
that replaces or appends. It need not be. The silver table of
[chapter 17](../17-subjects-and-the-lakehouse.md) is an analytical table
filled by a stream, and the pages here show the same deduplication over a
batch and over micro-batches. The side a table is on says what its rows mean;
whether it is loaded by a batch or by a stream is the type of the input.

## The pages

| Page | What it covers |
| --- | --- |
| [Bronze, silver and gold](bronze-silver-and-gold.md) | the three layers of a lakehouse, what each promises, and the load rule of each |
| [Grain and keys](grain-and-keys.md) | what one row stands for, the key that names it, and a duplicate at each grain |
| [Deduplication](deduplication.md) | exact duplicates with `distinct`, duplicates by key with `group` and `max_by`, and a replayed stream as their source |
| [Merge](merge.md) | a table kept up to date by key: insert or update, today as a pipeline and as design as a write mode |
| [Incremental loads](incremental-loads.md) | loading only what is new: a watermark the table holds, and a position the server keeps |

The pages run against the shop's vault, `--vault examples/pipelines/shop`.
A program that writes to the lakehouse is shown checked, with the type
`hql check` prints; a pipeline that ends in `target` or `sink` is `Unit`,
the type of a program whose result is what it wrote.
The file `silver_replayed` is the shop's six valid events as a replay
delivered them: two delivered twice, one corrected after the fact, and one
delivered late.

## What is next

Part III reads a different kind of data. [Chapter 21](../21-a-vault.md)
opens a folder of documents and asks it the questions chapter 6 asked a
table.
