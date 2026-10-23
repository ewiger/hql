// status: proposed
// feature: decimal, float, int and to_decimal convert between the numbers
// implementation: pending
// proposal: doc/proposals/HQL-0031/README.md
// environment: pure
// expected-type: List<String>
// expected: ["0.1", "3", "12.50", "1.25", "-2"]

// `decimal` reads a Float by its shortest digits and an Int at scale 0;
// `to_decimal` reads text; `float` gives the nearest Float and `int` the
// integer part.
[
    decimal(0.1f).str(),
    decimal(3).str(),
    "12.50".to_decimal().unwrap_or(0.0).str(),
    float(1.25).str(),
    int(-2.75).str()
]
