// status: valid-now
// feature: String.shout as String.yell binds the receiver under the new name, and as a function
// implementation: implemented
// environment: pure
// modules: imports/lib
// expected-type: List<String>
// expected-json: ["DUNE!", "EMMA!"]

from shelf import String.shout as String.yell

["Dune".yell(), yell("Emma")]
