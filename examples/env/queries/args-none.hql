// status: valid-now
// feature: args holds every --arg pair, here none
// implementation: implemented
// environment: pure
// expected-type: Int
// expected: 0

import io.env

env.args() | entries | size
