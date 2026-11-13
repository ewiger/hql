// status: valid-now
// feature: an unset variable reads as none, as a withheld one does
// implementation: implemented
// environment: pure
// expected-type: String
// expected: anonymous

import io.env

env.var("HQL_EXAMPLE_VARIABLE_NEVER_SET") | unwrap_or("anonymous")
