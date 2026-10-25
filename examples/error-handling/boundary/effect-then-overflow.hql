// status: invalid
// feature: An evaluation failure in the second statement comes after the first has run
// implementation: implemented
// environment: pure
// expected-type: Int
// error: IntegerOverflow
// stage: evaluate

// The whole program checks, so execution starts: `before` is written, then
// the addition overflows and the run fails. `hql check` prints `Int`, the
// type known before the run; `--envelope json` carries `"type": "Int"` and
// `"value": null`.
["before"] | target(stdout, Lines)

x = 9223372036854775807 + 1
x
