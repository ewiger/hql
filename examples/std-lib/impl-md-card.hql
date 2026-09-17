// status: proposed
// feature: trait implementation
// implementation: pending
// environment: knowledge-v1
// expected-type: Seq<Ref<Doc>>
// expected: the empty sequence: plain Markdown has no wikilink dialect
// note: MdCard implements the same trait over a CommonMark document from fixtures/cards/notes/plain.md. Asking a Markdown card for its uplinks is a well-formed question with an empty answer, which is what keeps card-ness independent of file format.

type MdCard = { source : Doc }

impl Card for MdCard {
    header = self.source.header
    body   = self.source.body

    fn name() -> String = self.header.name.as<String>()
    fn uplinks() -> Seq<Ref<Doc>> = []
}

note : MdCard = MdCard(resolve([[notes/plain]]))
note.uplinks()
