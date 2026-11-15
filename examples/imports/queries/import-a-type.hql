// status: proposed
// feature: from encoding imports a type and one receiver, without the module name
// implementation: pending
// proposal: doc/proposals/HQL-0060/index.hmd
// environment: pure
// expected-type: Option<String>
// expected: hé

from encoding import Hex, String.decode

"68C3A9".decode(Hex)
