// status: valid-now
// implementation: implemented
// environment: pure
// io: sandbox
// feature: a read decodes each document as the record its binding declares
// expected-type: Seq<String>
// expected: ["wren", "kingfisher"]

// Each line of the file is one JSON document, a `Data` tree. The binding's
// declared type names an element, so each document is read as a `Sighting`
// by its shape: every field at its type, any other entry ignored. A
// document that does not decode stops the evaluation, naming the document
// and the field.
type Sighting {
    species : String
    site    : String
    seen    : String
}

sightings : Seq<Sighting> = source(file("../sightings.jsonl"))
sightings | filter(s => s.seen == "morning") | map(s => s.species)
