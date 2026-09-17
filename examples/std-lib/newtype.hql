// status: design-question
// feature: nominal distinctness
// implementation: pending
// environment: pure
// expected-type: String
// expected: alice
// note: An alias renames a type and shares its values; a newtype makes a distinct one, so a card name cannot be passed where a title is wanted. Whether HQL needs both, and whether the wrapped value is reached by .value or by pattern, are open; std-lib/newtype-mismatch.hql shows what the distinction buys.

type Title = String
newtype CardName(String)

fn lookup(n : CardName) -> String = n.value

lookup(CardName("alice"))
