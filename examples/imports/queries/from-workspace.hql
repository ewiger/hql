// status: valid-now
// feature: from binds a workspace module's functions unqualified, the same as a standard module's
// implementation: implemented
// environment: pure
// modules: imports/lib
// expected-type: List<String>
// expected-json: ["103", "DUNE!", "EMMA!"]

from shelf import per_day, shout

[fmt("{}", per_day(412, 4)), shout("Dune"), "Emma".shout()]
