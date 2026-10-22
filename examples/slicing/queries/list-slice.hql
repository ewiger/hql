// status: proposed
// feature: A slice of a List is a List, and a negative position counts from the end
// implementation: pending
// proposal: doc/proposals/HQL-0026/README.md
// environment: pure
// expected-type: Data
// expected-json: {"first_two":["barn-owl","common-raven"],"last_two":["european-robin","house-sparrow"],"last":"house-sparrow","middle":["common-raven","european-robin"],"none":[]}

// A slice keeps the type it is taken from: a List gives a List. The same
// rule for negative positions holds for one position and for a slice, so
// `sightings[-1]` is the last element and `sightings[-2:]` the last two.
sightings = ["barn-owl", "common-raven", "european-robin", "house-sparrow"]

{
    first_two: sightings[:2],
    last_two: sightings[-2:],
    last: sightings[-1],
    middle: sightings[1:-1],
    none: sightings[3:1],
}
