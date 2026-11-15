// status: invalid
// feature: two from statements binding one name from two modules are ambiguous where the name is used
// implementation: implemented
// environment: pure
// modules: imports/lib
// phase: check
// kind: unknown-name
// severity: error

from shelf import per_day
from ledger import per_day

per_day(412, 4)
