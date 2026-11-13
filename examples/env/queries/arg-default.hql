// status: valid-now
// feature: an argument the invocation does not pass is none, and a program supplies its default
// implementation: implemented
// environment: pure
// expected-type: String
// expected: owl

import io.env

env.arg("species") | unwrap_or("owl")
