// status: invalid
// feature: a validator type implemented for more than one type is named in a rule, not as a field's type, since the field would have no one type
// implementation: implemented
// environment: pure
// error: TypeMismatch
// stage: typecheck

import validators

type Post : DataContract = {
    tags: NonEmpty
}

d : Data = {tags: ["a"]}
d | Post
