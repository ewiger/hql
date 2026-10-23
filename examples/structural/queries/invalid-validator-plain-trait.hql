// status: invalid
// feature: only a trait extending `Validator` is a validator type
// implementation: implemented
// environment: pure
// error: TypeMismatch
// stage: typecheck

trait Loud {
    fn validate(self) : Bool
}

impl Loud for String {
    fn validate(self) : Bool = self == upper(self)
}

type Shout : DataContract = {
    text: String
    validate text with Loud
}

d : Data = {text: "HI"}
d | Shout
