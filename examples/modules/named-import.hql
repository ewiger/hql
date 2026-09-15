// status: design-question
// feature: named module import
// implementation: pending
// environment: modules-v1
// expected-type: List[Person]
// expected: alice, carol
// note: Imported Person is a type and adults is a value; namespace collision rules remain open.

import people::{Person, adults}
adults
