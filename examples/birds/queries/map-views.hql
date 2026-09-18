// status: valid-now
// feature: A sorted map through its semantic ancestors
// implementation: implemented
// environment: pure
// expected-type: Data

sorted : SortedMap<String, Int> = SortedMap(
    ["european-robin", "barn-owl", "common-raven"],
    [1, 2, 1]
)
ordered : OrderedMap<String, Int> = sorted
mapping : Map<String, Int> = ordered
ordered_keys : Seq<String> = ordered.keys
unordered_keys : Set<String> = mapping.keys

{
    positions: ordered_keys,
    distinct_keys: size(unordered_keys),
    same_lookup: get(sorted, "barn-owl") == get(mapping, "barn-owl"),
}
