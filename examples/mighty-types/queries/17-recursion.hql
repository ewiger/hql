// status: valid-now
// implementation: implemented
// environment: pure
// feature: Recursive algorithms: factorial, gcd, quicksort, primes, Collatz
// expected-type: List<Int>
// expected: [3628800, 21, 123589, 10, 111]

// A declared function is bound before its body is checked, so it may call
// itself; `given … otherwise` is what ends the recursion.
fn factorial(n: Int) : Int = 1 given n <= 1 otherwise n * factorial(n - 1)
fn gcd(a: Int, b: Int) : Int = a given b == 0 otherwise gcd(b, a % b)

fn quicksort(xs: List<Int>) : List<Int> = match xs.first() {
    none => [],
    some(pivot) => concat(
        concat(quicksort(xs.drop(1).filter(x => x < pivot)), [pivot]),
        quicksort(xs.drop(1).filter(x => x >= pivot))
    ),
}

fn is_prime(n: Int) : Bool = n >= 2 and range(2, n).all(d => n % d != 0)
fn collatz(n: Int) : Int = 0 given n == 1 otherwise 1 + collatz(n / 2 given n % 2 == 0 otherwise 3 * n + 1)

[
    factorial(10),
    gcd(1071, 462),
    quicksort([5, 3, 8, 1, 9, 2]) | reduce(0, (acc, x) => acc * 10 + x),
    range(1, 30).filter(is_prime).size(),
    collatz(27),
]
