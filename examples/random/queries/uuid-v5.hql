// status: valid-now
// feature: a version 5 UUID is a function of its name and namespace
// implementation: implemented
// environment: pure
// expected-type: String
// expected: 2ed6657d-e927-568b-95e1-2665a8aea6a2

import random

uuid("www.example.com", "dns")
