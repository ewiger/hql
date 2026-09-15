// status: design-question
// feature: projection lambda
// implementation: pending
// environment: knowledge-v1
// expected-type: List[String]
// expected: Alice, Carol
// note: Compare lambda.hql; receiver and nested placeholder scope need a rule.
// alternatives: functions/lambda.hql

people
| filter(.age > 30)
| map(.name)
