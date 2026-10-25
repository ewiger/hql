// status: invalid
// feature: An import of a module no root provides is a name error
// implementation: implemented
// environment: pure
// error: NameError
// stage: typecheck

// `import` is resolved by the checker against the module roots in force.
// With none configured, the diagnostic says how to configure one.
import nosuch
1
