# Corpus fixture environments

These are design fixtures, not a running HMD adapter. `cards/` is the proposed
workspace root; a corpus source runs conceptually at its root unless a profile
says otherwise. Companion `.hmd` files are input documents, not counted corpus
cases. Module bodies are future syntax, and `expected/people.hmd` is a golden
output artifact. No fixture runs network code or persists data.

## Profiles

| Metadata environment | Supplied values and assumptions |
| --- | --- |
| `pure` | No host bindings, cards, imports or prelude functions; literals only plus bindings introduced by the case. |
| `prelude-v1` | Proposed pure `length`, accepting String or a collection; no card variables. |
| `knowledge-v1` | Explicit fixture imports of alice/bob/carol, plus cards, people, knowledge, root, and proposed functions described below. |
| `unbound-card-v1` | The same workspace exists but no alice binding/import is supplied. A file named alice.hmd never implicitly exports a symbol. |
| `modules-v1` | knowledge-v1 plus module root `modules/`; people and graph exports have the candidate interfaces in those files. No nonexistent module exists. |
| `alice-cell-v1` | knowledge-v1 with this:Card bound to alice, the owner of a conceptual declaration cell. |
| `transclusion-v1` | Host this is a separate Card; snippet's closure is defined in snippet.hmd and retains its own captured x=10. |

For HMD cases the host also supplies this:Card even with the pure profile;
"pure" excludes extra fixture names and effects, not the defining closure.

All standard functions in the examples are **proposed interfaces**, not an
implemented standard library. `cards` is an ordered collection of all fixture
Cards in root-relative path order. `people` is the validated Person selection
alice, bob, carol. Those three variable names are **explicit fixture imports**,
not automatic name lookup. Cases may shadow them with their own bindings.
`root.projects` denotes the projects namespace, not a property on raw text.

Convenience metadata filters use a candidate lifted predicate: absent type or
status does not match a String comparison, and missing tags is an empty list.
This is an explicit fixture assumption, not a settled rule for optional values
or implicit coercion. General missing-field access is explored separately under
frontmatter/. No pervasive null semantics is assumed.

The proposed schema has Person <: Card with `name:String`, `age:Int`, and
optional `nickname:String`; Project <: Card with `status:String`. Alice/Bob/Carol
are 42/16/35. Only those three satisfy Person; the two Sam documents deliberately
lack the schema. `fm.type` supplies a refinement hint, not proof of conformance.
WorksOn <: Relation[Person,Project] declares typed source/target references and
optional since/until dates and role. Mapping YAML strings to Date/Role/Ref and
projecting validated fields onto Card specializations are proposed adapter work.

## Field conventions and ordinary HMD

For this corpus, title comes from the first heading, body is an Hmd value,
frontmatter is a Frontmatter value, and sections/blocks/links retain identity
and source positions. `fm` is candidate shorthand for `frontmatter`;
`frontmatter`/`FrontMatter` naming is not finalized. Shortcuts such as `.type`,
`.tags`, `.age` are candidates: some require typed schema projection, not
unrestricted access to arbitrary YAML as known types.

YAML reference-looking strings are quoted: `source: "[[alice]]"`. Bare brackets
are YAML collection syntax and must not be mistaken for a typed HQL reference.
These fields are schema-decoded by a future adapter; current HMD parsing alone
does not create a WorksOn relation from them. Ordinary body links stay `[[ref]]`.
The existing HMD `[[target|label]]` form is a display label, not a relation type.
No arbitrary HMD link-property syntax is introduced here.

`[[sam]]` has two candidate cards at people/sam and archive/sam with no explicit
import/spine winner. `does-not-exist` has no target. HQL must delegate actual
resolution to HMD; this fixture does not implement a competing resolver.
Backlink expectations count authored body links only (not YAML endpoint fields,
synthetic facts or transclusions), giving carol, family-record and
research/graph-notes for alice. The resolver's final adapter contract must make
these scopes explicit.

## Knowledge profile and provenance

`knowledge.tsv` is a small explicit semantic fixture, not an exhaustive dump of
all links in cards/. It carries link facts, metadata, structure, Relation Card
claims, an imported HQL declaration, and an explicit derived adult predicate.
All source documents and module interfaces are local. Core relation projection
uses both WorksOn Cards. Identity is the Card path, not the (source,target) pair.

The imported kinship assertion is explicit data in architecture.hql; prose does
not automatically imply it. The Adult derivation is exactly the stated age
predicate; no natural-language inference engine is assumed. Derived facts must
retain rule identity and origin. Fact provenance may point to a Block/Section/
Card or imported assertion; it is distinct from a relation's source endpoint.

The active and ended WorksOn claims are intentionally contradictory. Neither
wins by file order or confidence. Unknown confidence is not 1.0; confidence is
not probability of truth without a model. `knowledge | graph` is a projection,
not the full epistemic model. The graph path example uses directed structural
links alice -> bob -> carol. Neighbor direction remains a design question.
The weighted-edge case uses a candidate projection of explicit numeric claim
confidence to weight: only weights >0.5 survive, including works-active and
family-claim; unweighted facts are excluded. That mapping is not a general rule
that weight and confidence are equivalent.

Evidence is ordinary section content in Relation Cards. A nonempty Evidence
section establishes only that structural predicate, not truth, quality or
sufficiency of evidence. Constraints and Satisfaction are future value concepts.

## Modules, cells and outputs

The module files expose a proposed surface, not production import semantics.
Frontmatter `import` uses the existing singular HMD key; exposing its bindings
as HQL lexical variables is new integration work. The plural `imports` mapping
case is explicitly an alternative, currently just user-owned HMD metadata.
Corpus `.hmd` cases are evaluated conceptually with this workspace root; relative
imports are resolved there, not against the corpus folder that stores the case.

Blocks close over the defining card environment (`this`, explicit imports and
card-level bindings). Locals may shadow without leaking. A copied/transcluded
closure keeps its defining environment; ordering, invalidation and cycles need
future rules. `snippet.hmd` captures 10, even inside a host that binds x=99.

Rendering belongs to the host. Graph, Hmd, HmdTable and other presentation values
are not stdout. HmdGraph is a presentation of Graph. Explicit print/save would
be effects; no corpus case is allowed to persist data today.
