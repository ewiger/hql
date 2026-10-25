// status: invalid
// feature: a value read as a subtype holds its parent's rules too
// implementation: implemented
// environment: pure
// phase: execute
// kind: contract-violation
// severity: error
// expected-type: Window

type Span : DataContract = {
    low: Int
    high: Int

    require low <= high
}

type Window : Span = {
    width: Int
}

// `Window` declares no rule, and `Span`'s still holds of it.
d : Data = {low: 7, high: 3, width: 4}
d | Window
