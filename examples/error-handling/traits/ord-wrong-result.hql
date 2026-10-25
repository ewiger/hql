// status: invalid
// feature: compare of Ord returns an Ordering, not a number
// implementation: implemented
// environment: pure
// error: TypeMismatch
// stage: typecheck

// `compare` decides `<`, `sort`, `min` and `max`, and returns `Ordering`.
// An implementation writing an `Int` is rejected at its result.
type Version { major : Int }

impl Ord for Version {
    fn compare(self, other: Version) : Int = self.major - other.major
}

[Version(major = 2), Version(major = 1)] | sort
