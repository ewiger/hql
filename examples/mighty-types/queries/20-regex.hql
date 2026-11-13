// status: valid-now
// implementation: implemented
// environment: pure
// feature: The regex module: matching, finding, replacing, capturing and splitting
// expected-type: List<String>
// expected-json: ["true","2026-10-21,2026-10-22,2026-10-23","a#b#c#","ERROR disk full","key=value|key|value","3","2"]

// `regex` is a standard module outside the prelude: explicit, since a
// pattern language is a domain of its own. A pattern is a String; a pattern
// that does not parse is an evaluation error at the call.
import regex

log = "2026-10-21 ERROR disk full; 2026-10-22 INFO ok; 2026-10-23 ERROR net down"

[
    str(regex.is_match("abc123", "[0-9]+")),
    regex.find_all(log, "[0-9]{4}-[0-9]{2}-[0-9]{2}").join(","),
    regex.replace_all("a1b22c333", "[0-9]+", "#"),
    regex.find(log, "ERROR [a-z ]+").unwrap_or("none"),
    regex.captures("key=value", "([a-z]+)=([a-z]+)").unwrap_or([]).join("|"),
    str(regex.split("a, b,c", ", *").size()),
    str(regex.find_all(log, "ERROR").size()),
]
