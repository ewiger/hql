// status: valid-now
// feature: a Decimal is written to JSON and YAML as a number with its digits
// implementation: implemented
// environment: pure
// expected-type: List<String>
// expected-json: ["{\n  \"amount\": 12.50,\n  \"item\": \"tea\"\n}", "amount: 12.50\nitem: tea\n"]

// The number keeps its scale: `12.50`, not `12.5`.
type Payment {
    item   : String
    amount : Decimal
}

p = Payment(item = "tea", amount = 12.50)
[format(p, Json), format(p, Yaml)]
