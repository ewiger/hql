// status: valid-now
// feature: `errs() | flat_map(e => e.violations)` gives one element per violation, not one per rejected value
// implementation: implemented
// environment: pure
// expected-type: List<String>
// expected: ["[] UserData.id_present", "[] UserData.age_positive", "[] UserData.age_positive"]

type UserData : DataContract = {
    id: String
    age?: Int
    require id != "" as id_present
    require age? >= 0 as age_positive
}

xs : List<Data> = [{id: "", age: -1}, {id: "u-2"}, {id: "u-3", age: -5}]
xs
| validate(UserData)
| errs()
| flat_map(e => e.violations)
| map(v => fmt("{} {}", v.path, v.rule))
| List
