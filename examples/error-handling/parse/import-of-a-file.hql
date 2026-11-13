// status: invalid
// feature: An import names a module, never a path
// implementation: implemented
// environment: pure
// phase: parse
// kind: unexpected-token
// severity: failure

// The parser rejects a string after `import`: a module is named by its
// position under a module root, and no file is ever opened here.
import "lib/shapes/geometry.hql"
1
