// status: valid-now
// feature: an argument is text, converted by the program as any text is
// implementation: implemented
// environment: pure
// expected-type: Int
// expected: 10

import io.env

env.arg("limit") | and_then(s => s.to_int()) | unwrap_or(10)
