// status: invalid
// feature: A file that is not there is an evaluation failure, not a type error
// implementation: implemented
// environment: pure
// expected-type: Int
// phase: execute
// kind: io-failure
// severity: failure

// The program is well typed: `source` of a `JsonLines` file is a stream of
// documents, and `size` counts it. Whether the file exists is known only
// when the run opens it.
source(file("no-such-file.jsonl"), JsonLines) | size
