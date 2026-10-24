// status: valid-now
// implementation: implemented
// environment: pure
// feature: Generic records and functions, with the parameters taken from each use
// expected-type: List<String>
// expected-json: ["4","[1, 2, 3]","1","one","a"]

// `Stack<T>` takes `T` from its field at construction, so
// `Stack(items = [1, 2])` is a `Stack<Int>`. `Pair<A, B>` is the prelude's
// pair, the element `zip` and `enumerate` produce.
type Stack<T> { items : List<T> }

fn push<T>(s: Stack<T>, x: T) : Stack<T> = Stack(items = concat(s.items, [x]))
fn peek<T>(s: Stack<T>) : Option<T> = s.items.last()
fn pop<T>(s: Stack<T>) : Stack<T> = Stack(items = s.items.take(s.items.size() - 1))
fn swap<A, B>(p: Pair<A, B>) : Pair<B, A> = Pair(first = p.second, second = p.first)

s = Stack(items = [1, 2]) | push(3) | push(4)
p = swap(Pair(first = "one", second = 1))

[
    str(peek(s).unwrap_or(0)),
    str(pop(s).items),
    str(p.first),
    p.second,
    str(peek(Stack(items = ["a"])).unwrap_or("")),
]
