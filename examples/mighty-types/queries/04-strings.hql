// status: valid-now
// implementation: implemented
// environment: pure
// feature: The String receiver module of the prelude
// expected-type: List<String>
// expected-json: ["Hello, World","HELLO, WORLD","hello, world","Hello, HQL","a + b + c","olleh","5","true","true","ababab","00x","A-B"]

// `value.f(args)` calls `f` with the value as its first argument, resolved
// among the receiver modules in scope. `string` is in the prelude, so no
// import is needed; `string.trim(s)` is the same call written qualified.
s = "  Hello, World  "

[
    s.trim(),
    s.trim().upper(),
    s.trim().lower(),
    s.trim().replace_all("World", "HQL"),
    "a,b,c".split(",").join(" + "),
    "hello".chars().reverse().join(""),
    str("hello".length()),
    str("Hello".starts_with("He")),
    str("Hello".contains("ell")),
    "ab".repeat(3),
    "x".pad_left(3, "0"),
    fmt("{}-{}", "a".upper(), "b".upper()),
]
