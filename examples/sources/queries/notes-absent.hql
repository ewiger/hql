// status: valid-now
// feature: an empty CSV cell is an absent optional field by default, and a boundary may read it as the empty text instead
// implementation: implemented
// environment: knowledge-v1
// vault: sources/station
// expected-type: List<String>
// expected: ["wren [] 2", "owl [dusk] none"]

// `count?` takes the format's default: the empty cell in the second row is
// absent. `note?` is configured with `absent = ["missing"]`, so the empty
// cell in the first row is the text `""`, present.
type Note {
    species : String
    note?   : String
    count?  : Int
}

notes : Seq<Note> = source("notes")
notes | map(n => fmt("{} [{}] {}", n.species, n.note | unwrap_or("?"), n.count)) | List
