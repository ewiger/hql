// status: valid-now
// feature: import shelf binds the module name of a workspace module, the same as for a standard one
// implementation: implemented
// environment: pure
// modules: imports/lib
// expected-type: List<String>
// expected-json: ["DUNE!", "EMMA!", "103"]

import shelf

["Dune".shout(), shelf.shout("Emma"), fmt("{}", 412 | shelf.per_day(4))]
