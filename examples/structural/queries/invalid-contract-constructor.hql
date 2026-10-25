// status: invalid
// feature: a contract built with its constructor is held to its rules, as one read with `| T` is
// implementation: implemented
// environment: pure
// phase: execute
// kind: contract-violation
// severity: error
// expected-type: UserData

type UserData : TypeContract = {
    id: String
    age: Int = 18

    require id != ""
}

UserData(id = "")
