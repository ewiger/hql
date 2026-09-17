// status: invalid
// feature: diagnostic
// implementation: pending
// environment: knowledge-v1
// error: PipelineTypeMismatch
// stage: typecheck
// note: Presenters are terminal. A Presentation carries no collection structure, so a later filter stage has nothing to consume; presentation belongs at the end of a pipeline.
// alternatives: presentation/table.hql

cards
| table
| filter(.type == "person")
