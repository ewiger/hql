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

`hql render` runs every such block in a document and prints the document with
each answer transcluded as an `hql#result` block:

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

```hql#result
key           value
------------  -----
barn_owls     2
observations  4
saw_robin     true
species       3
```
````

Without `--write`, rendering prints and changes nothing. `--write <file>`
writes the rendered document, replacing each previous `hql#result` block
rather than appending another.
