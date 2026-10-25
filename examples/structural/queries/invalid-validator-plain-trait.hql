// status: invalid
// feature: only a trait extending `Validator` is a validator type
// implementation: implemented
// environment: pure
// phase: check
// kind: type-mismatch
// severity: error

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
