// status: valid-now
// feature: Int.abs selects the overload of abs whose self is an Int, and binds it unqualified
// implementation: implemented
// environment: pure
// expected-type: Int
// expected: 3

from math import Int.abs as int_abs

int_abs(0 - 3)
