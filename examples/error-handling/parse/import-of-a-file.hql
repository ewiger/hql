// status: invalid
// feature: An import names a module, never a path
// implementation: implemented
// environment: pure
// error: SyntaxError
// stage: parse

// The parser rejects a string after `import`: a module is named by its
// position under a module root, and no file is ever opened here.
import "lib/shapes/geometry.hql"
1
