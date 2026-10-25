/// Receiver functions over a book's title.
module club.titles for String

/// The first letter of each word, `"The Hobbit".initials()` is `"TH"`.
fn initials(self) : String = self.words().map(w => w.slice(0, 1)).join("")

/// A shelf mark: the initials and the year, `"TH-1937"`.
fn shelf_mark(self, year: Int) : String = fmt("{}-{}", self.initials(), year)
