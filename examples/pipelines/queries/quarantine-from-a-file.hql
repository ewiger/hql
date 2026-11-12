// status: valid-now
// feature: raw events validated against a contract, the broken rules of each rejected one
// implementation: implemented
// environment: knowledge-v1
// vault: pipelines/shop
// expected-type: Seq<Data>
// expected: [{id: 3, rules: ["EventData.amount.type"]}, {id: 5, rules: ["EventData.customer.missing"]}, {id: 7, rules: ["EventData.non_negative"]}, {id: 8, rules: ["EventData.known_kind"]}]

type EventData : DataContract = {
    id       : Int
    kind     : String
    customer : String
    amount   : Decimal
    at       : String

    require amount >= 0 as non_negative
    require ["order_placed", "order_shipped", "payment_failed"] | contains(kind) as known_kind
}

source("raw_events")
| validate(EventData)
| errs()
| map(e => {id: e.value.id, rules: e.violations | map(v => v.rule)})
