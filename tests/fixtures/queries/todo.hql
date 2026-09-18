// Everything still open in the vault, by title.
//
// Run it:  hql --vault tests/fixtures/vault run tests/fixtures/queries/todo.hql

open = cards | filter(c => c.metadata.status == "todo")

open
| sort(by = c => c.title)
| map(c => c.title)
| table
