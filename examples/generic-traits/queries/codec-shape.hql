// status: valid-now
// feature: A trait over a representation, its output a trait parameter inferred from the implementation
// implementation: implemented
// environment: pure
// expected-type: List<List<String>>
// expected-json: [["raven", "robin"], ["wren"]]

// The shape of the codec contracts: the representation is Self and the value
// it reads is the parameter, so `Decoder<String>` for Lines reads Strings. A
// bounded function leaves T to the one implementation its format has.
trait Parse<T> {
    fn parse(self, text: String) : Option<T>
}

type Separated { by : String }

impl Parse<List<String>> for Separated {
    fn parse(self, text: String) : Option<List<String>> = some(text.split(self.by))
}

fn read_all<T, F : Parse<T>>(format: F, texts: List<String>) : List<T> =
    texts | map(text => format.parse(text)) | somes

read_all(Separated(by = ","), ["raven,robin", "wren"])
