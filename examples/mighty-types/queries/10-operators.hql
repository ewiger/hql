// status: proposed
// implementation: pending
// proposal: doc/proposals/HQL-0024/README.md
// environment: pure
// feature: Arithmetic operators resolved through Add and Sub for a declared type
// expected-type: List<Int>
// expected: [4, 6, 2, 2, 1]

// `+` resolves the addition capability of its operand types. For Int and
// Float that is built in; a declared type implements `Add` for its own
// values and changes nothing else.
type Vec2 { x : Int, y : Int }

impl Add for Vec2 {
    fn add(self, other: Vec2) : Vec2 = Vec2(x = self.x + other.x, y = self.y + other.y)
}

impl Sub for Vec2 {
    fn sub(self, other: Vec2) : Vec2 = Vec2(x = self.x - other.x, y = self.y - other.y)
}

a = Vec2(x = 1, y = 2)
b = Vec2(x = 3, y = 4)
c = a + b
d = b - a

[c.x, c.y, d.x, d.y, (c - b).x]
