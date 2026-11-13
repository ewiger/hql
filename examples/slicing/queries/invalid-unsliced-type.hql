// status: invalid
// feature: A declared type without Slice takes no range bracket
// implementation: implemented
// environment: pure
// phase: check
// kind: type-mismatch
// severity: error

// A declared type says what a part of its value is by implementing Slice;
// one that does not is not sliced.
type Playlist { name : String, tracks : List<String> }

mix = Playlist(name = "mix", tracks = ["first", "second"])

mix[1:]
