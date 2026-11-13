// status: invalid
// feature: An elementwise operation reads its one element and nothing else
// implementation: implemented
// environment: pure
// phase: check
// kind: type-mismatch
// severity: error

// `Elementwise<I, O>` grants a backend the right to split the input and run
// each part separately. A body that reads a value bound outside it, here
// `threshold`, is not a function of its element alone, and the checker
// refuses the binding rather than let a planner rely on a false shape.
threshold = 300
long : Elementwise<Int, Bool> = n => n > threshold

[412, 280, 310] | filter(long)
