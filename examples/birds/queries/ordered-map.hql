// status: valid-now
// feature: Construction order belongs to key positions
// implementation: implemented
// environment: pure
// expected-type: Seq<String>
// expected: [european-robin, barn-owl, common-raven]

route = OrderedMap(
    ["european-robin", "barn-owl", "common-raven"],
    [1, 2, 1]
)
stops = route.keys
stops
