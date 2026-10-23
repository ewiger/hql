// status: valid-now
// feature: a JSON source read through its boundary: `name` reads the nested `profile.display_name`
// implementation: implemented
// environment: knowledge-v1
// vault: sources/station
// expected-type: List<String>
// expected: ["u-1 Ada", "u-2 Grace"]

type Profile : Data = {
    id   : String
    name : String
}

profiles : Seq<Profile> = source("profiles")
profiles | map(p => fmt("{} {}", p.id, p.name)) | List
