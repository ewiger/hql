// status: valid-now
// feature: a JSON number read as a Decimal keeps its digits exactly
// implementation: implemented
// environment: pure
// expected-type: List<Decimal>
// expected: [1234567890.123456789012345, 0.10]

// The decoder keeps the number's text, and reading it as a declared
// `Decimal` field takes every digit and the scale; a Float would round the
// first to 1234567890.1234567.
type Payment {
    item   : String
    amount : Decimal
}

bond = Data("{\"item\": \"bond\", \"amount\": 1234567890.123456789012345}", Json) | Payment
tea = Data("{\"item\": \"tea\", \"amount\": 0.10}", Json) | Payment
[bond.amount, tea.amount]
