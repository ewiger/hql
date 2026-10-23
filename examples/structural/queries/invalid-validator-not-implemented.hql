// status: invalid
// feature: `validate f with V` needs `V` implemented for the field's type
// implementation: implemented
// environment: pure
// error: TypeMismatch
// stage: typecheck

trait Email : Validator { }

impl Email for String {
    fn validate(self) : Bool = self.contains("@") and not self.starts_with("@")
}

type Person : DataContract = {
    age: Int
    validate age with Email
}

d : Data = {age: 3}
d | Person
