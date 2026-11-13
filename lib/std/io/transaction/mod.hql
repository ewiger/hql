// Provided by the `std` extension.
//
// See doc/proposals/HQL-0051/index.hmd.

/// The control calls of a transaction scope's handle, `tx`, bound by
/// `with transaction tx on participants { … }` (module `io`): each ends the
/// attempt and gives no value, so it stands where any type is expected and
/// nothing after it runs. A block that reaches its end commits with its
/// final value, so a scope needs none of them. In the prelude.
module io.transaction for Transaction

import core
import io

/// Ends the attempt by committing it, with `value` as the scope's `ok`.
/// Written only to commit before the end of the block:
/// `tx.commit(0) given nothing_to_do`.
effectful fn commit<T>(self, value: T) : Never

/// Ends the attempt by rolling it back, with `reason` in the scope's `err`,
/// its `kind` `"rolled_back"`. Nothing the attempt wrote to a participant
/// remains: `tx.rollback("insufficient balance") given sender.balance < 100`.
effectful fn rollback(self, reason: String) : Never

/// Ends the attempt by rolling it back and runs the block again on a fresh
/// transaction, as the next attempt, while the scope's `attempts` remain;
/// with none remaining the scope's `err` has the `kind` `"exhausted"`.
effectful fn retry(self) : Never
