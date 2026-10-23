// status: valid-now
// feature: a contract's `require` and `validate` rules hold for every value `| T` reads
// implementation: implemented
// environment: pure
// expected-type: List<String>
// expected: ["Alice", "CH"]

// `validate country with is_country_code` is `require is_country_code(country)`.
// `age` is optional: a rule reads it as `age?`, and holds where it is absent.
fn is_country_code(code: String) : Bool = code.length() == 2 and code.upper() == code

type UserData : DataContract = {
    id: String
    name: String
    age?: Int
    country: String

    require id != ""
    require name != ""
    validate country with is_country_code
    require age? >= 0
    require age? <= 150
}

d : Data = {id: "u-42", name: "Alice", country: "CH", signup_source: "campaign-7"}
user = d | UserData
[user.name, user.country]
