// status: valid-now
// feature: in a project that declines the prelude, a function the program imports with from is in scope
// implementation: implemented
// environment: knowledge-v1
// vault: contexts/strict
// expected-type: String
// expected: owl

from string import trim

"  owl " | trim
