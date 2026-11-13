// status: invalid
// feature: The trailing arguments of a generic variadic parameter fill one type
// implementation: implemented
// environment: pure
// phase: check
// kind: type-mismatch
// severity: error

// `T` is one type for every trailing argument. The first binds it to Int,
// and a String shares no declared ancestor with an Int, so the call is
// refused. A list literal holds both as trees: `[1, "owl"]` is a List<Data>.
fn pack<T>(xs: ...T) : List<T> = xs

pack(1, "owl")
