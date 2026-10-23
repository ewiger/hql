// status: invalid
// feature: rules belong to a contract: a plain structural type has none
// implementation: implemented
// environment: pure
// error: TypeMismatch
// stage: typecheck

type Point = {
    x: Float
    y: Float

    require x >= 0.0
}

Point(x = 1.0, y = 2.0)
