// status: valid-now
// feature: String.* as String.bar_* binds each String receiver under a prefixed name, with no wrapper
// implementation: implemented
// environment: pure
// modules: imports/lib
// expected-type: List<String>
// expected-json: ["DUNE!", "emma"]

from shelf import String.* as String.bar_*

["Dune".bar_shout(), bar_hush("Emma")]
