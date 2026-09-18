// status: valid-now
// feature: Key positions follow intrinsic String order
// implementation: implemented
// environment: pure
// expected-type: Seq<String>
// expected: [barn-owl, common-raven, european-robin]

counts = SortedMap(
    ["european-robin", "barn-owl", "common-raven"],
    [1, 2, 1]
)
alphabet = counts.keys
alphabet
