// status: invalid
// feature: a value that breaks a contract rule does not read as the contract, and every broken rule is named
// implementation: implemented
// environment: pure
// error: RuntimeError
// stage: evaluate
// expected-type: UserData

// Both rules are broken: the error names each as written.
type UserData : DataContract = {
    id: String
    age?: Int

    require id != ""
    require age? >= 0
}

d : Data = {id: "", age: -3}
d | UserData
