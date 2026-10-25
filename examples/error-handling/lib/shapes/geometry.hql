// Module `shapes.geometry`: one public function and one private helper, for
// the name errors an importer meets.
module shapes.geometry

private fn square(n: Int) : Int = n * n

fn area(side: Int) : Int = square(side)
