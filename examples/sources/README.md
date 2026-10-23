# Data sources: reading what a run context configures

`station/` is a vault whose `hql.toml` declares six sources under
`[sources.<id>]`: two vaults of notes, `journal` and `archive`, a CSV file
of counts, a JSON array of sightings, and a CSV ledger and a JSON array of
profiles whose fields a boundary maps
([HQL-0035](../../doc/proposals/HQL-0035/README.md)). A program reads one with
`source("id")`, as [HQL-0012](../../doc/proposals/HQL-0012/README.md)
specifies: the id is a literal, the configuration states the kind, and the
kind states the type, so checking reads the configuration and opens nothing.

| Case | What it shows | Result |
| --- | --- | --- |
| [two vaults](queries/two-vaults.hql) | two `vault` sources side by side, each a `Vault` with its own `cards` | each vault's card names |
| [typed counts](queries/typed-counts.hql) | a `csv` source read as `Seq<Tally>` | the sum of the counts |
| [sightings after noon](queries/sightings-after-noon.hql) | a `json` source is the elements of its one array | the species seen after noon |
| [journal counts](queries/journal-counts.hql) | a vault source joined to a CSV source by species, the binary `join` of [HQL-0013](../../doc/proposals/HQL-0013/README.md) | each card's title beside its count |
| [ledger through its boundary](queries/ledger-boundary.hql) | a `csv` source whose headers the boundary maps to `Tally`'s fields | each species and its total |
| [profiles through their boundary](queries/profiles-boundary.hql) | a `json` source whose `name` is read from `profile.display_name` | each id and name |
| [join without a condition](queries/invalid-join-without-condition.hql) | a join with no keys and no `on` is refused, naming `cross_join` | refused |
| [unknown id](queries/invalid-unknown-id.hql) | an unconfigured id is a name error, naming the nearest configured one | refused |
| [computed id](queries/invalid-computed-id.hql) | a computed id is refused: the type would be unknown until evaluation | refused |
| [connection string](queries/invalid-connection-string.hql) | a URL is not an id; connection details are configuration | refused |

From the repository root:

```sh
cargo build --locked
cd examples/sources/station
../../../target/debug/hql run ../queries/two-vaults.hql
```
