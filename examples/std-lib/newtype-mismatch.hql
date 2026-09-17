// status: invalid
// feature: nominal distinctness
// implementation: pending
// environment: pure
// error: TypeMismatch
// stage: typecheck
// note: Invalid under the newtype reading in std-lib/newtype.hql. A bare String is not a CardName, so the wrapper has to be written; if this were accepted the newtype would be documentation rather than a type.

newtype CardName(String)

fn lookup(n : CardName) -> String = n.value

lookup("alice")
