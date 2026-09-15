// status: invalid
// feature: diagnostic
// implementation: pending
// environment: knowledge-v1
// error: DuplicateDeclaration
// stage: declare
// note: Historical HQL assertion syntax; invalid under reject-duplicates policy only. Idempotence remains open. Duplicate endpoints do not make two different Relation Cards duplicates.

declare relatedTo([[alice]], [[bob]]) { kind = daughter }
declare relatedTo([[alice]], [[bob]]) { kind = daughter }
