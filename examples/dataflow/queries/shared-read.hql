// status: valid-now
// feature: a source bound once is read once: both pipelines and both branches see the one read, even after the file is replaced
// implementation: implemented
// environment: pure
// io: sandbox
// expected-type: List<List<String>>
// expected: [["owl"], ["owl", "wren", "kite"], ["owl", "wren"], ["kite"]]

type Sighting {
    species : String
    count   : Int
}

seen = tap(file("../data/sightings.jsonl"), JsonLines) | Sighting
first = seen | take(1) | map(s => s.species)
replaced = [{species: "heron", count: 9}] | target(file("../data/sightings.jsonl"), JsonLines)
{ many, few } = seen | fork {
    many = filter(s => s.count > 1) | map(s => s.species)
    few = filter(s => s.count <= 1) | map(s => s.species)
}
[first, seen | map(s => s.species) | take(9), many | take(9), few | take(9)]
