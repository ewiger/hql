// status: design-question
// feature: sum type spelling
// implementation: pending
// environment: pure
// expected-type: Option<Int>
// expected: Some(1)
// note: The same declaration without the bar, because pipes are fundamental to HQL and a bar that means composition in expressions and alternation in types is a grammar cost. A keyword block also gives variants somewhere to carry documentation.
// alternatives: std-lib/option-declaration.hql

type Option<T> = variants {
    Some(T)
    None
}

first : Option<Int> = Some(1)
first
