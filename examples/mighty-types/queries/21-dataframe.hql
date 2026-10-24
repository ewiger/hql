// status: valid-now
// implementation: implemented
// environment: pure
// feature: Group, aggregate, sort and join over records, as a dataframe would
// expected-type: List<String>
// expected-json: ["north: 13 units, 55.0 revenue, managed by ada","south: 7 units, 17.5 revenue, managed by bob"]

// Records are the rows, `group` partitions them, a map over the groups
// aggregates each partition into a record, and `join` pairs the rows of
// two collections that a predicate relates. The east region has no manager,
// so the inner join drops it.
type Sale { region : String, product : String, units : Int, price : Float }
type Region { name : String, manager : String }
type Summary { region : String, units : Int, revenue : Float }

sales = [
    Sale(region = "north", product = "widget", units = 10, price = 2.5f),
    Sale(region = "north", product = "gadget", units = 3, price = 10.0f),
    Sale(region = "south", product = "widget", units = 7, price = 2.5f),
    Sale(region = "east", product = "gizmo", units = 1, price = 99.0f),
]

regions = [Region(name = "north", manager = "ada"), Region(name = "south", manager = "bob")]

fn revenue(s: Sale) : Float = float(s.units) * s.price

by_region = sales
| group(by = s => s.region)
| map(g => Summary(
    region = g.key,
    units = g.items | map(s => s.units) | sum,
    revenue = g.items | map(revenue) | sum,
  ))
| sort(by = r => r.region)

join(by_region, regions, (s, r) => s.region == r.name)
| map(p => fmt("{}: {} units, {} revenue, managed by {}", p.first.region, p.first.units, p.first.revenue, p.second.manager))
