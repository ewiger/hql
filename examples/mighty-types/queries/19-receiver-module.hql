// status: valid-now
// implementation: implemented
// environment: pure
// modules: mighty-types/lib
// feature: A workspace receiver module written in HQL
// expected-type: List<String>
// expected-json: ["HELLO WORLD!","al","true","HI!"]

// `mighty.text` opens with `module mighty.text for String`, so once imported its
// functions with `self` resolve as `value.f()`; `text.shout("hi")` is the
// qualified spelling of the same declaration. `is_blank` is declared by
// `mighty.text` and by the prelude's `string` too, so `"   ".is_blank()` is
// ambiguous and the call is qualified.
import mighty.text

["hello world".shout(), "ada lovelace".initials(), str(text.is_blank("   ")), text.shout("hi")]
