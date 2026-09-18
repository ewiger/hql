// The cards nearest a question, and what sits one link from them.
//
// Run it:  hql --vault tests/fixtures/vault run tests/fixtures/queries/nearest.hql
//
// `semantic` keeps the retrieval — model, metric, index — with every score,
// because a score is evidence rather than relevance. `expand` then records why
// each node is present, so the graph does not claim that the neighbours matched.

import semantic

question = "bearer token authorization"

cards
| semantic(question)
| take(2)
| expand(depth = 1)
| table
