// status: valid-now
// feature: encode writes a text's UTF-8 octets in hex, base64, base64url and percent-encoding
// implementation: implemented
// environment: pure
// expected-type: List<String>
// expected-json: ["68c3a9", "Pz8+", "Pz8-", "a%20b%2F%C3%A9~"]

import encoding

["hé".encode(Hex), "??>".encode(Base64), "??>".encode(Base64Url), "a b/é~".encode(Percent)]
