// Module `shapes.circle`: a receiver module over its own type, whose `self`
// functions are its implementation of `Shape`.
module shapes.circle for Circle : Shape

import shapes

type Circle { radius : Float }

fn area(self) : Float = 3.0f * self.radius * self.radius
fn name(self) : String = "circle"
fn diameter(self) : Float = 2.0f * self.radius
