// status: valid-now
// feature: decode gives none for text not valid in the encoding and for octets that are not UTF-8
// implementation: implemented
// environment: pure
// expected-type: List<Option<String>>
// expected-json: [null, null, null, null]

import encoding

["6g".decode(Hex), "aGk".decode(Base64), "/w==".decode(Base64), "%ff".decode(Percent)]
