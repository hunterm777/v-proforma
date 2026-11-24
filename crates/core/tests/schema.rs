//! schema/deal.schema.json is generated from the deal types, so the schema
//! and the parser cannot drift. Regenerate with:
//! `BLESS=1 cargo test -p proforma-core --features schema --test schema`

#![cfg(feature = "schema")]

#[test]
fn committed_schema_matches_the_types() {
    let s = schemars::schema_for!(proforma_core::deal::Deal);
    let text = serde_json::to_string_pretty(&s).unwrap() + "\n";
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../schema/deal.schema.json");
    if std::env::var("BLESS").is_ok() {
        std::fs::write(path, &text).unwrap();
    }
    assert_eq!(
        std::fs::read_to_string(path).unwrap(),
        text,
        "run with BLESS=1 to regenerate"
    );
}
