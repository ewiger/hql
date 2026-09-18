// status: invalid
// feature: Sorted maps require intrinsically ordered keys
// implementation: implemented
// environment: pure
// error: TypeMismatch
// stage: typecheck

// A list of sightings has positions, but is not intrinsically Orderable.
// OrderedMap accepts these keys; SortedMap rejects them.
SortedMap(
    [ ["barn-owl"], ["european-robin"] ],
    [2, 1]
)
