// status: proposed
// feature: trait as an argument bound
// implementation: pending
// environment: knowledge-v1
// expected-type: String
// expected: Alice
// note: One function serves all three implementors. The trait is a bound on the argument, never the type of a stored value, so label works on DataCard, MdCard and HmdCard without a common supertype and without runtime probing.

fn label(c : impl Card) -> String =
    c.header.title.unwrap_or(c.name())

label(HmdCard(resolve([[alice]])))
