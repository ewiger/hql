// status: proposed
// implementation: pending
// proposal: doc/proposals/HQL-0024/README.md
// environment: pure
// modules: mighty-types/lib
// feature: Importing a workspace module under an alias
// expected-type: List<Float>
// expected: [3.0, 12.0, 4.0]

// `import mighty.geometry as geo` makes one module visible as `geo`; its
// functions are reached as `geo.f(…)` and the types it declares by their
// names, since there is one lattice. The module root is configuration, not
// syntax: here the corpus header names `lib`.
import mighty.geometry as geo

r = geo.scale(Rect(width = 2.0, height = 1.5), 2.0)

[geo.circle_area(geo.unit_circle()), geo.rect_area(r), r.width]
