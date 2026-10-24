// status: invalid
// feature: a default is a literal: a call is refused when the program is checked
// implementation: implemented
// environment: pure
// error: TypeMismatch
// stage: typecheck

type Event : DataContract = {
    id: String
    label: String = upper("none")
}

Event(id = "e-1")
