// status: valid-now
// feature: a contract built with its constructor fills its defaults and holds its rules
// implementation: implemented
// environment: pure
// expected-type: List<Int>
// expected: [18, 34]

type UserData : TypeContract = {
    id: String
    age: Int = 18

    require id != ""
    require age >= 0
}

[UserData(id = "u-1"), UserData(id = "u-2", age = 34)] | map(u => u.age) | List
