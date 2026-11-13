// status: valid-now
// feature: Bytes holds the UTF-8 octets of a text and converts to and from hex and base64
// implementation: implemented
// environment: pure
// expected-type: Data
// expected-json: {"size": 5, "hex": "68656c6c6f", "base64": "aGVsbG8=", "text": "hello", "back": true}

import encoding

b = Bytes("hello")
{size: b.size(), hex: b.encode(Hex), base64: b.encode(Base64),
 text: b.to_text() | unwrap_or(""), back: Bytes("68656c6c6f", Hex) == b}
