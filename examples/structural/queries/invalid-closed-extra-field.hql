// status: invalid
// feature: a closed type refuses a field it does not declare, when the Data is read
// implementation: implemented
// environment: pure
// phase: execute
// kind: contract-violation
// severity: error
// expected-type: Point

type Point = {
    x: Float
    y: Float
}

d : Data = {x: 1.0, y: 2.0, z: 3.0}
d | Point
