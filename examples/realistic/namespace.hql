// status: proposed
// feature: namespace selection
// implementation: pending
// environment: knowledge-v1
// expected-type: List[Card]
// expected: projects/active-project
// note: Namespace tree querying and _.fm projection candidate; compare .field shorthand. No filesystem I/O inside the core.

root.projects
| cards
| filter(_.fm.status == "active")
