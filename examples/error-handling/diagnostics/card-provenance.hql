// status: invalid
// feature: A failure over a value read from a card names the card's document as its source
// implementation: implemented
// environment: knowledge-v1
// vault: error-handling/diagnostics/vault
// expected-type: List<Int>
// phase: execute
// kind: division-by-zero
// severity: failure

// `fine.md` declares `pages: 0`. The division fails on the element `map`
// was applying its function to, and the diagnostic's `source` is the
// document that card was read from.
cards | map(c => match c.metadata.pages { some(Int n) => 100 / n, _ => 0 })
