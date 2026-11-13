// status: invalid
// feature: `internal` is reserved
// implementation: implemented
// environment: pure
// phase: parse
// kind: unexpected-token
// severity: failure

// `internal`, visibility across the modules of one package, is admitted once
// every module records its package. Until then it is refused wherever it is
// written.
internal fn double(n: Int) : Int = n * 2

double(21)
