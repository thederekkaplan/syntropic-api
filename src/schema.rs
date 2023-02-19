// @generated automatically by Diesel CLI.

diesel::table! {
    message (id) {
        id -> Bytea,
        timestamp -> Timestamptz,
        body -> Text,
    }
}
