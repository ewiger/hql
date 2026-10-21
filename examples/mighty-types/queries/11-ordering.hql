// status: valid-now
// implementation: implemented
// environment: pure
// feature: A declared type becomes Orderable by implementing Ord
// expected-type: List<String>
// expected-json: ["0.9.9 < 1.2.3 < 1.10.0","1.10.0","true"]

// `impl Ord for Version` supplies `compare`, and with it `<`, `sort`,
// `min` and `max` accept the type: `Orderable` is the ancestry form of the
// same contract.
type Version { major : Int, minor : Int, patch : Int }

fn key(v: Version) : Int = v.major * 10000 + v.minor * 100 + v.patch

impl Ord for Version {
    fn compare(self, other: Version) : Ordering = compare(key(self), key(other))
}

fn show(v: Version) : String = fmt("{}.{}.{}", v.major, v.minor, v.patch)

versions = [
    Version(major = 1, minor = 2, patch = 3),
    Version(major = 1, minor = 10, patch = 0),
    Version(major = 0, minor = 9, patch = 9),
]

[
    versions | sort | map(show) | join(" < "),
    versions | max | map(show) | unwrap_or("-"),
    str(Version(major = 1, minor = 2, patch = 3) < Version(major = 1, minor = 10, patch = 0)),
]
