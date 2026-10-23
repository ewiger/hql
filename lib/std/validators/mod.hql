// Module `validators`: the standard validator types (HQL-0034), imported
// where they are used, `import validators`, and never by the prelude, so a
// program's own `Email` is its own. Unlike the modules the binary's
// extensions implement, this one is written in HQL: each type is a trait
// extending `Validator`, implemented once per type of value it checks.
//
// A field declared with one, `email: Email`, is of the one type that
// implements it; `NonEmpty`, implemented for more than one, is named in a
// rule, `validate tags with NonEmpty`, where the field's type selects the
// implementation. A broken one is the violation `Contract.f` of kind `rule`.
import core
import string
import seq
import option

// An address: one `@` with text before it, and a domain after it holding a
// `.` that neither opens nor closes it.
trait Email : Validator { }

private fn is_domain(domain: String) : Bool = domain.contains(".")
    and not domain.starts_with(".")
    and not domain.ends_with(".")

impl Email for String {
    fn validate(self) : Bool = self.occurrences("@") == 1
        and not self.starts_with("@")
        and is_domain(self.split("@") | last | unwrap_or(""))
}

// Text that is more than whitespace, or a sequence with an element.
trait NonEmpty : Validator { }

impl NonEmpty for String {
    fn validate(self) : Bool = self.trim() != ""
}

impl<T> NonEmpty for List<T> {
    fn validate(self) : Bool = self.size() > 0
}
