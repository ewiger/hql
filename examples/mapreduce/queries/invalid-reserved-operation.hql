// status: invalid
// feature: The operation types are reserved, and a program declares none of them
// implementation: implemented
// environment: pure
// phase: check
// kind: type-mismatch
// severity: error

// `Operation`, `Elementwise`, `Partition`, `Reduction` and `Barrier` are the
// standard library's types for what a pipeline step invokes, one per shape
// and their common supertype. A program's own record of the same name would
// make a step's shape depend on which declaration a reader found first.
type Operation {
    name : String
}

Operation(name = "map")
