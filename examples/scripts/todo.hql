#!/usr/bin/env hql
// status: valid-now
// implementation: implemented
// environment: pure
// feature: A script's first argument picks a status, and a result that is an err exits 1
// expected-type: Result<List<String>, String>
// expected: ok(["The Hobbit", "Leaves of Grass"])

import io.env

type Book { title: String, status: String }

shelf = [
    Book(title = "Frankenstein", status = "done"),
    Book(title = "Dune", status = "reading"),
    Book(title = "The Hobbit", status = "todo"),
    Book(title = "Leaves of Grass", status = "todo"),
]

// The status asked for, `todo` when the script is run without one.
wanted = env.argv()[0] | unwrap_or("todo")
left = shelf | filter(b => b.status == wanted) | map(b => b.title)
ok(left) given left != [] otherwise err("no book is " + wanted)
