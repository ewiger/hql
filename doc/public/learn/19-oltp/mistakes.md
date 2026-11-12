# When a mistake is found

A binding can break, and it breaks at the earliest phase that can see it.
Checking sees the configuration; evaluation sees the systems:

| What is wrong | Found | What is said |
| --- | --- | --- |
| `tap("order_event")` | checking | check error[unknown-name]: no endpoint `order_event` is configured; did you mean `order_events`? |
| `source("order_events")` | checking | check error[type-mismatch]: `order_events` is a `kafka` endpoint, read by `tap` as a `Stream`, not by `source` as a `Seq` |
| a stream written by `target("orders")` | checking | check error[type-mismatch]: `target("orders")` writes a Table or a sequence of a declared record, not Stream&lt;Event&gt; |
| `sink("ledger")` | checking | check error[type-mismatch]: endpoint `ledger` is a `sqlite` endpoint, which `sink` does not write; write it whole with `target` |
| a password in `hql.toml` | loading the vault | `url` holds credentials; name the environment variable that holds the address, `url_env = "VARIABLE"` |
| `SHOP_DSN` not set | evaluation, before the first row | execute failure[io-failure]: endpoint `orders`: the environment variable `SHOP_DSN`, which `dsn_env` names, is not set |
| a raw order read as `Event` | evaluation, at that order | execute failure[io-failure]: endpoint `raw_events`, field `amount`: "free" is not a decimal number |
| a write to a wildcard subject | evaluation, at the write | execute failure[io-failure]: subject `orders.>` holds a wildcard, which reads many subjects and writes none; a message is published to one subject |

A secret is named by the variable that holds it and never printed. A
failure at evaluation names the endpoint by its id and the value by its
field; what it never does is continue past a value it could not read and
count it later as missing. A pipeline that must continue says so in its
shape, as `day.hql` does with `validate` and `fork`: the order that would
have stopped it is a row in the dead-letter subject instead.
