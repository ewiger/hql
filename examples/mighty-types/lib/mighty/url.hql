// Module `mighty.url`: a receiver module over String whose contract is one
// function and one binding. `words` is `private`: the module's own `slug`
// calls it, and no importer reaches it, by name or as `value.words()`.
module for String

separator = "-"

private fn words(self) : List<String> = self.lower().split(" ").filter(w => w.length() > 0)

fn slug(self) : String = self.words().join(separator)
