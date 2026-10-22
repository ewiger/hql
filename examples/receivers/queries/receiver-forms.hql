// status: valid-now
// feature: A receiver call reaches a declaration that takes self, written or from the module's pattern
// implementation: implemented
// environment: pure
// expected-type: List<Int>
// expected-json: [42, 2, 3, 3]

// `fn abs(self: Int)` and `fn size<T>(self: Collection<T>)` write their
// receiver; `fn length(self)` takes `String`, the pattern of
// `module for String`. Each is one declaration behind three call forms.
[(0 - 42).abs(), [1, 2].size(), "abc".length(), string.length("abc")]
