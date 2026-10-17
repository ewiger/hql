# Queries in documents

A card can carry a query. Rendering the card runs the query against the vault
and writes the answer beneath it, so a list that would otherwise be maintained
by hand is computed instead.

## An `hql#eval` block

Write the query in a fenced block whose info string is `hql#eval`:

````markdown
```hql#eval
cards
| filter(c => c.metadata.status == "todo")
| sort(by = c => c.title)
| map(c => c.title)
| table
```
````

## Rendering

`hql render` runs every cell in a document and builds the document it yields.
By default each cell is replaced by its output, written as plain Markdown:

```sh
hql --vault examples/birds/wiki render examples/birds/wiki/field-notes.hmd
```

The field notes card in the birds example computes a summary of its own
observations:

````markdown
```hql#eval
sightings = ["barn-owl", "european-robin", "barn-owl", "common-raven"]
occurrences : Collection<String> = sightings
species = Set(sightings)

{
    observations: size(occurrences),
    barn_owls: count(occurrences, "barn-owl"),
    species: size(species),
    saw_robin: contains(species, "european-robin"),
}
```
````

In the built document the cell is gone and a Markdown table is in its place:

```markdown
| key          | value |
| ------------ | ----- |
| barn_owls    | 2     |
| observations | 4     |
| saw_robin    | true  |
| species      | 3     |
```

A record becomes a table, a sequence a bulleted list, and `| table` a table.

## Writing the result

Without `--write`, rendering prints and changes nothing. `--write <out>` writes
the built document to another file, leaving the source as it is:

```sh
hql --vault wiki render wiki/board.hmd --write site/board.md
```

## Keeping the cells

`--emit after` keeps each cell and writes its output immediately after it,
between two HTML comments that Markdown previews do not display:

````markdown
```hql#eval
cards
| filter(c => c.metadata.status == "todo")
| map(c => c.title)
```
<!-- hql:output -->
- Bearer tokens
- Session cookies
<!-- /hql:output -->
````

With `--in-place` this rewrites the source itself, the way a notebook saves its
cell outputs. The next render replaces what is between the markers rather than
adding to it. `--in-place` without `--emit after` is refused, because it would
replace the cells in the source with their output.

A single cell can choose its own mode in its info string, such as
`` ```hql#eval after `` in a document otherwise built with `replace`.
