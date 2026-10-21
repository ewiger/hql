// status: valid-now
// implementation: implemented
// environment: pure
// modules: mighty-types/lib
// feature: A workspace receiver module written in HQL
// expected-type: List<String>
// expected-json: ["HELLO WORLD!","al","true","HI!"]

// `mighty.text` opens with `module for String`, so once imported its
// functions with `self` resolve as `value.f()`; `text.shout("hi")` is the
// qualified spelling of the same declaration.
import mighty.text

["hello world".shout(), "ada lovelace".initials(), str("   ".is_blank()), text.shout("hi")]
