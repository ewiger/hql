# 0065 — Neo4j parity analysis: graph patterns, traversal and graph schema

Status: backlog
Related: [0016](0016-sql-parity-analysis.md),
[0015](0015-jq-parity-analysis.md),
[0017](archive/0017-graph-projection-and-view.md),
[graph-type](../wiki/hql/types/graph-type.hmd),
[link-op](../wiki/hql/operators/link-op.hmd),
[knowledge](../wiki/hql/knowledge.hmd)

Neo4j is the reference graph database, and Cypher is its query language. HQL
already has a graph model: `Graph<N, E>` and `Edge<S, T>` in the graph
extension, cards linked by wikilinks and traversed as uplinks and downlinks,
and the `@[[..]] -> @[[..]] {..}` link operator. What it lacks is much of the
vocabulary Cypher uses on a graph: patterns, variable-length paths and
shortest paths. This issue is to Neo4j what
[0016](0016-sql-parity-analysis.md) is to SQL.

Parity is of **capability, not syntax**. Whatever a Cypher query can compute
over a property graph, an HQL program computes over a `Graph` value, a vault,
or a knowledge graph, with its types checked before it runs.

This issue is an **analysis**, not an implementation. Its outcome is a
proposal and follow-up issues; no code under `src/` changes here.

## Questions the analysis answers

1. **The data model.** How Neo4j's property graph maps onto HQL: a node with
   labels and properties against a typed `N`; a relationship with one type and
   properties against `Edge<S, T>` with a `Data` payload; a label against a
   declared type, where a key in a tree is an author's claim and not a type.
   Whether one node may carry several labels, and what that is in HQL's types.
2. **Pattern matching.** `MATCH (a:Person)-[:KNOWS]->(b)`, and what HQL's form
   for it is: a pipeline over edges, a pattern expression, or both. Optional
   matches (`OPTIONAL MATCH`) against `Option<T>`, and several patterns sharing
   a variable, which is a join ([0041](archive/0041-binary-joins.md)).
3. **Paths and traversal.** Variable-length paths (`-[*1..3]->`), direction,
   `shortestPath` / `allShortestPaths`, and a path as a value with its own type.
   How far uplinks and downlinks reach, and where a recursive traversal ends on
   a graph with cycles.
4. **Aggregation and projection.** `WITH`, `RETURN`, `collect`, `count`,
   `DISTINCT`, `ORDER BY`, `SKIP` / `LIMIT`, and `UNWIND`. Most of these
   overlap with the sequence operations already shipped; the analysis names
   only what a graph changes.
5. **Writes.** `CREATE`, `MERGE`, `SET`, `DELETE` and `DETACH DELETE`. In a
   functional language a write returns a new graph, or is a `target` effect on
   a store. The analysis decides which, and whether writing a vault's links
   belongs here at all.
6. **Schema and constraints.** Uniqueness and existence constraints, and
   indexes, against contracts and validator types
   ([0043](archive/0043-contract-validation.md)).
7. **Graph algorithms.** What the Graph Data Science library offers
   (centrality, community detection, similarity), and which belong in an
   extension rather than the language.
8. **A Neo4j source.** Whether a Neo4j database is a configured data source
   (`source("id")`, [0040](archive/0040-configured-data-sources.md)), and
   whether a query pushes down to Cypher.
9. **What HQL adds.** Typed nodes and edges, documents as nodes,
   retrieval beside traversal, so the analysis argues for parity *and more*
   and not for a Cypher clone.

## Method

- Survey the Cypher manual and sort every feature into *supported now*,
  *supportable* (with the missing type or operation named) or *out of scope*
  (with a reason).
- For each supportable feature, write a Cypher query and the HQL program it
  would become as an example under `examples/` with `status: proposed`. The
  examples are the design corpus.
- Rank the roadblocks by how many examples each one unblocks.

## Done when

- A feature matrix (Cypher feature, HQL status, missing piece) exists in a new
  proposal under `doc/proposals/`.
- The examples above are committed with `status: proposed`.
- The main roadblocks are listed in priority order, and each becomes a backlog
  issue or a step of an existing one.
