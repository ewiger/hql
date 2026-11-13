// status: invalid
// feature: An impl and a module's trait list are one coherence rule: two implementations for one type are refused
// implementation: implemented
// environment: pure
// modules: traits/lib
// phase: check
// kind: duplicate-implementation
// severity: error

// `module shapes.circle for Circle : Shape` already implements `Shape` for
// `Circle`. The refusal names the module.
import shapes
import shapes.circle

impl Shape for Circle {
    fn area(self) : Float = 0.0f
    fn name(self) : String = "disc"
}

Circle(radius = 1.0f).area()
