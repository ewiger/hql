// Module `shapes.square`: lists `Shape` and declares `area` but no `name`,
// which `Shape` requires and gives no default body.
module shapes.square for Square : Shape

import shapes

type Square { side : Float }

fn area(self) : Float = self.side * self.side
