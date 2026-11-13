// Module `shapes`: the trait every shape module implements.
module shapes

trait Shape {
    fn area(self) : Float
    fn name(self) : String
    fn describe(self) : String = fmt("{} with area {}", self.name(), self.area())
}

fn total<T : Shape>(xs: List<T>) : Float = xs | map(x => x.area()) | sum
