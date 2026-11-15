// status: valid-now
// feature: encrypt_to writes an age message to an X25519 recipient, and its identity decrypts it
// implementation: implemented
// environment: pure
// expected-type: Option<String>
// expected-json: "J45.909"

from crypto import generate_key_pair, encrypt_to, decrypt_with

type Patient {
    patient   : String
    diagnosis : String
}

pair = generate_key_pair(X25519)
record = Patient(patient = "p-104", diagnosis = "J45.909")
sealed = record.diagnosis | encrypt_to([pair.public])
sealed | decrypt_with(pair.private) | and_then(to_text)
