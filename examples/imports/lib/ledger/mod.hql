/// A reading ledger's arithmetic.
module ledger

/// Pages a day over so many days, rounded up.
fn per_day(pages: Int, days: Int) : Int = (pages + days - 1) / days
