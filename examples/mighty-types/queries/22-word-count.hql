// status: proposed
// implementation: pending
// proposal: doc/proposals/HQL-0024/README.md
// environment: pure
// feature: Word count, the MapReduce classic, as one pipeline
// expected-type: String
// expected: the=3 fox=2 brown=1

// Split, group by the word, count each group, order by count with the word
// as the tiebreak, and keep the top three. `sort` is stable, so sorting by
// word first and by count second leaves ties in word order.
text = "the quick brown fox jumps over the lazy dog the fox"

text.split(" ")
| group(by = w => w)
| map(g => Pair(first = g.key, second = g.items.size()))
| sort(by = p => p.first)
| sort(by = (a, b) => compare(b.second, a.second))
| take(3)
| map(p => fmt("{}={}", p.first, p.second))
| join(" ")
