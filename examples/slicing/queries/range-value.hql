// status: valid-now
// feature: A Range is a value, and one Range slices any sliceable type
// implementation: implemented
// environment: pure
// expected-type: List<String>
// expected-json: ["common-raven european-robin","el"]

// `a:b` inside brackets is a Range. A program that names the same window
// more than once builds the Range itself, and the bracket takes it as it
// takes `a:b`: the index's type decides between a position and a slice.
window = Range(start = 1, end = 3)
sightings = ["barn-owl", "common-raven", "european-robin"]

[
    sightings[window].join(" "),
    "hello"[window],
]
