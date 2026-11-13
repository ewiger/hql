// status: valid-now
// feature: decoded JSON read as an open structural type, one document at a time
// implementation: implemented
// environment: pure
// io: sandbox
// expected-type: List<String>
// expected: ["barn owl", "wren"]

// The format decodes bytes into Data at the source; `| Sighting` reads each
// document as the type. `site` and `seen` are not declared, and Sighting is
// open, so they are tolerated.
type Sighting : Data = {
    species: String
}

source(file("../sightings.jsonl"), Json)
| Sighting
| map(s => s.species)
| take(2)
| List
