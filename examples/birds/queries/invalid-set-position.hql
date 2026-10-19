// status: invalid
// feature: A set has no positions to index
// implementation: implemented
// environment: pure
// error: TypeMismatch
// stage: typecheck

// A set keeps each species once and in no order, so "the first one" names
// nothing; sort it into a list to give its members positions.
species = Set(["barn-owl", "common-raven"])
species[0]
