// status: proposed
// feature: explicit Card type
// implementation: pending
// environment: knowledge-v1
// expected-type: Card
// expected: alice
// note: Preferred typed binding; compare cards/reference and cards/resolve-ascription. No implicit card symbol export.

alice : Card = resolve([[alice]])
alice
