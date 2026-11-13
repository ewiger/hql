// status: valid-now
// feature: decode reads each encoding back, base64url with or without padding, and percent-decoding leaves + as it is
// implementation: implemented
// environment: pure
// expected-type: List<Option<String>>
// expected-json: ["hé", "hi?", "hi?", "a", "a", "a+b é"]

import encoding

["68C3A9".decode(Hex), "aGk/".decode(Base64), "aGk_".decode(Base64Url),
 "YQ".decode(Base64Url), "YQ==".decode(Base64Url), "a+b%20%C3%A9".decode(Percent)]
