// Module `mighty.geometry`: plane shapes and the functions over them. A
// workspace module under the root `examples/mighty-types/lib`, named by its
// position: `mighty/` is a namespace directory and this file is `geometry`.

module mighty.geometry
type Circle { radius : Float }
type Rect { width : Float, height : Float }

fn circle_area(c: Circle) : Float = 3.0f * c.radius * c.radius
fn rect_area(r: Rect) : Float = r.width * r.height
fn unit_circle() : Circle = Circle(radius = 1.0f)
fn scale(r: Rect, k: Float) : Rect = Rect(width = r.width * k, height = r.height * k)
