// status: design-question
// feature: empty presentation
// implementation: pending
// environment: knowledge-v1
// expected-type: Presentation
// expected: no visible output; the updated Card is still computed
// note: Suppressing a view differs from returning Unit. empty presents an existing value as nothing, while a declaration has nothing to present in the first place. empty versus none spelling is unresolved.
// alternatives: hypermarkdown/empty-cell.hmd, design/unit-void.hql

updated = alice | set .fm.status "reviewed"
updated | empty
