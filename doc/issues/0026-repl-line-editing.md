# 0026 — The REPL edits lines and reads multi-line input

Status: done
Related: the [CLI model](../models/behavior/cli.md), section "The REPL"

`hql repl` reads stdin one raw line at a time, so a terminal session has none
of what a REPL is expected to have:

- **No line editing or history.** The arrow keys print escape sequences instead
  of moving the cursor or recalling an earlier input.
- **No multi-line input.** Every line is a complete program, so a literal laid
  out over several lines fails on its first line:

  ```
  hql> shelf = [
  syntax failure: expected an expression, found end of input
  ```

  The tutorial writes its data this way, so it cannot be pasted into a session.

## Scope

- An interactive session edits a line in place and recalls earlier inputs with
  the arrow keys, through a line editor. History persists across sessions in a
  file under the user's home directory.
- An input whose brackets — `(`, `[`, `{` — are not yet closed continues on the
  next line under a continuation prompt, and is evaluated as one input once
  they close. A string literal cannot span lines, so brackets alone decide.
  Recalling a multi-line input from history recalls all of it.
- Piped input follows the same rule, so a script with a multi-line literal runs
  the same as the session typed by hand.
- Ctrl-C abandons the input being typed and keeps the session; Ctrl-D on an
  empty line leaves.
- A line opening with `|` continues the input before it, so the book's
  pipelines, written one step per line, paste into a session.

## Done when

- The CLI model's section "The REPL" states the continuation rule and history.
- A test in `tests/cli.rs` pipes a multi-line binding and uses it afterwards.
