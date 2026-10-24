// status: invalid
// feature: rules belong to a contract: a plain structural type has none
// implementation: implemented
// environment: pure
// error: TypeMismatch
// stage: typecheck

type Point = {
    x: Float
    y: Float

    require x >= 0.0f
}

Point(x = 1.0f, y = 2.0f)
