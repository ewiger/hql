// status: valid-now
// feature: the empty keyed literal {:} is the declared map type, and Data otherwise
// implementation: implemented
// environment: pure
// expected-type: Bool
// expected: true

counts : Map<String, Int> = {:}
counts == Map<String, Int>([], [])
