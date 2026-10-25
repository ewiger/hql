// status: invalid
// feature: An Eq implementation compares two values of its own type
// implementation: implemented
// environment: pure
// error: TypeMismatch
// stage: typecheck

// `impl Eq for Version` makes `==` compare two `Version`s. `Version == Int`
// is rejected at the comparison, since no implementation takes an `Int`.
type Version { major : Int }

impl Eq for Version {
    fn eq(self, other: Version) : Bool = self.major == other.major
}

Version(major = 1) == 1
