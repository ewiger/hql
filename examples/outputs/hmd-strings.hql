// status: design-question
// feature: Hmd construction
// implementation: pending
// environment: knowledge-v1
// expected-type: List[Hmd]
// expected: three Hmd fragments; not yet joined or rendered
// note: Titles Alice/Bob/Carol differ from reference IDs alice/bob/carol; raw title interpolation is not a safe canonical link constructor.
// alternatives: outputs/hmd-typed.hql

cards
| filter(.type == "person")
| map(c => hmd("- [[${c.title}]]"))
