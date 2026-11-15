/// Helpers over a book's title.
module shelf for String

/// The title in capitals, with a mark.
fn shout(self) : String = upper(self) + "!"

/// The title in lower case.
fn hush(self) : String = lower(self)

/// Pages a day to finish a book in so many days.
fn per_day(pages: Int, days: Int) : Int = pages / days

/// The title padded to a width, for the module's own use.
private fn pad(title: String, width: Int) : String = title
