// status: valid-now
// feature: `warn P` is a rule whose violation leaves the value valid, and `as name` names a rule
// implementation: implemented
// environment: pure
// expected-type: List<Int>
// expected: [34, 130]

type Person : DataContract = {
    name: String
    age?: Int

    require age? >= 0 as non_negative_age
    warn age? < 120 as plausible_age
}

// 130 breaks only the warning, so the value reads.
xs : List<Data> = [{name: "Ana", age: 34}, {name: "Old", age: 130}]
xs | Person | map(p => p.age) | somes() | List
