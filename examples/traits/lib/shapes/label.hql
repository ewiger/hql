// Module `shapes.label`: lists two traits that both require `title`. One
// `self` function is one implementation, so the module line is refused.
module shapes.label for Label : Titled, Headed

trait Titled {
    fn title(self) : String
}

trait Headed {
    fn title(self) : String
}

type Label { text : String }

fn title(self) : String = self.text
