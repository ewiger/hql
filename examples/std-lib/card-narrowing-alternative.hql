// status: design-question
// feature: narrowing alternative to a trait
// implementation: pending
// environment: pure
// expected-type: Unit
// expected: a contributed contract with no runtime value
// note: The preserved reading from the wiki cards, where Card is a document narrowed by conditions rather than a trait with implementors. It gives one Card type and no dispatch, so a JSON document and an HMD document must either be the same type or stop being cards.
// alternatives: std-lib/trait-card.hql

type Card <: Doc {
    // the document represents a piece of knowledge
    // the document has a place in a vault
}
