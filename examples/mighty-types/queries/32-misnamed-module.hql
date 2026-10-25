// status: invalid
// feature: A module's declaration names the module its file is
// implementation: implemented
// environment: pure
// modules: mighty-types/lib
// phase: check
// kind: unknown-name
// severity: error

// `lib/mighty/misnamed.hql` is the module `mighty.misnamed`, and it opens with
// `module mighty.renamed`: importing it is refused, naming both.
import mighty.misnamed

misnamed.hello()
