// status: valid-now
// feature: Explicit comparison without intrinsic element order
// implementation: implemented
// environment: pure
// expected-type: List<List<String>>
// expected: [[european-robin], [common-raven], [barn-owl, barn-owl]]

groups : List<List<String>> = [
    ["barn-owl", "barn-owl"],
    ["european-robin"],
    ["common-raven"],
]

// Lists have positions, but no intrinsic ordering relation of their own.
// A comparison for this operation orders groups by number of observations.
// Equally sized groups retain their previous positions.
sort(groups, by = (left, right) => compare(size(left), size(right)))
