/// Reading plans: how long a book takes at a number of pages a day.
module club.reading

private fn rounded_up(n: Int, d: Int) : Int = (n + d - 1) / d

/// Days to read `pages` at `per_day` pages a day; a part day counts as one.
fn days(pages: Int, per_day: Int) : Int = rounded_up(pages, per_day)

/// Days to read every book of `shelf`, one after another.
fn days_for_all(shelf: List<Int>, per_day: Int) : Int {
    each = shelf | map(pages => days(pages, per_day))
    each | sum
}
