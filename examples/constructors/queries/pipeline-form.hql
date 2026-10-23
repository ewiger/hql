// status: valid-now
// feature: A constructor is called or written as a step, and both forms are one call
// implementation: implemented
// environment: pure
// expected-type: List<Bool>
// expected-json: [true, true, true]

// `fn Set<T>(xs: Collection<T>) : Set<T>` and `fn Map<K, V>(keys: Seq<K>,
// values: Seq<V>) : Map<K, V>` are declared in `std/core/mod.hql`. The
// piped value is the constructor's first argument.
[
    ([1, 2, 2] | Set) == Set([1, 2, 2]),
    (["owl", "bat"] | Map([2, 2])) == Map(["owl", "bat"], [2, 2]),
    ([] | List<Int>) == List<Int>([])
]
