// status: valid-now
// feature: a contract field written `name: T = literal` takes that value where the data has none
// implementation: implemented
// environment: pure
// expected-type: List<Int>
// expected: [18, 34]

// Defaults are applied before the rules are checked, so `age` always holds a
// value the rule can read.
type UserData : DataContract = {
    id: String
    age: Int = 18
    country: String = "CH"

    require age >= 0
}

xs : List<Data> = [{id: "u-1"}, {id: "u-2", age: 34}]
xs | UserData | map(u => u.age) | List
