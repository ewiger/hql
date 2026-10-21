// Module `mighty.text`: a receiver module over String. Each function takes
// `self`, so `"hi".shout()` and `text.shout("hi")` are the same call.
module for String

fn shout(self) : String = fmt("{}!", self.upper())
fn initials(self) : String = self.split(" ").map(w => w.chars().first().unwrap_or("")).join("")
fn is_blank(self) : Bool = self.trim().length() == 0
