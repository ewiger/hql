// status: invalid
// feature: results are not split by an operation of their own; both sides are a `fork` over `oks()` and `errs()`
// implementation: implemented
// environment: pure
// error: TypeMismatch
// stage: typecheck

type UserData : DataContract = {
    id: String
}

xs : List<Data> = [{id: "u-1"}]
xs | validate(UserData) | split(",")
