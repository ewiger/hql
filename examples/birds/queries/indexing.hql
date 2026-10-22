// status: valid-now
// feature: A position in a sequence, from the start or the end, and its absence past the end
// implementation: implemented
// environment: pure
// expected-type: Data
// expected-json: {"first":"barn-owl","last":"european-robin","from_end":"european-robin","missing":null,"nested":2}

// Positions count from 0. A position past the end is absence, as a missing
// key of a map is, so `sightings[5]` is `none` rather than a failure. A
// negative position counts from the end: `sightings[-1]` is the last.
sightings = ["barn-owl", "common-raven", "european-robin"]
counts = {birds: [1, 2, 1]}

{
    first: sightings[0],
    last: sightings[2],
    from_end: sightings[-1],
    missing: sightings[5],
    nested: counts.birds[1],
}
