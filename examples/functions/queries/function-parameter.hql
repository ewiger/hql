// status: valid-now
// feature: A function taken as a parameter, and one returned as a result
// implementation: implemented
// environment: pure
// expected-type: Int
// expected: 36

// `adder(3)` returns a function that captured `k`, so it keeps the 3 after
// `adder` has returned. `twice` takes a function and applies it two times.
fn adder(k: Int) : Int -> Int = n => n + k
fn twice(f: Int -> Int, x: Int) : Int = f(f(x))

twice(adder(3), 30)
