// status: proposed
// feature: A declared type implements Slice and its values take the bracket
// implementation: pending
// proposal: doc/proposals/HQL-0026/README.md
// environment: pure
// expected-type: List<String>
// expected-json: ["second","third"]

// `Slice` is the capability behind `xs[a:b]`. A type implementing it says
// what a part of one of its values is, and its slice is of its own type.
type Playlist { name : String, tracks : List<String> }

impl Slice for Playlist {
    fn slice(self, range: Range) : Playlist =
        Playlist(name = self.name, tracks = self.tracks[range])
}

mix = Playlist(name = "mix", tracks = ["first", "second", "third"])

mix[1:].tracks
