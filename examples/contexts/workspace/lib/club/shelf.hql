/// The reading club's shelf: a module of the workspace, found under the
/// module root its `hql.toml` names.
module club.shelf

/// The books on `books` that are not on `read`, in shelf order.
fn unread(books: List<String>, read: List<String>) : List<String> = books | filter(b => not contains(read, b))
