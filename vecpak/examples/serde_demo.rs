use serde::{Serialize, Deserialize};
use vecpak::{to_vec, from_slice};
use std::collections::HashMap;

fn main() {
    let person_struct = Person {
        name: "Alice".to_string(),
        age: 30,
        active: true,
    };

    let person_term = vecpak::Term::PropList(vec![
        (vecpak::Term::Binary(b"age".to_vec()), vecpak::Term::VarInt(30)),
        (vecpak::Term::Binary(b"name".to_vec()), vecpak::Term::Binary(b"Alice".to_vec())),
        (vecpak::Term::Binary(b"active".to_vec()), vecpak::Term::Bool(true)),
    ]);

    let encoded_term = vecpak::encode(person_term.clone());
    let decoded_struct: Person = from_slice(&encoded_term).unwrap();
    let encoded_struct = to_vec(&person_struct).unwrap();
    let decoded_term: vecpak::Term = vecpak::decode(&encoded_struct).unwrap();

    assert_eq!(person_struct, decoded_struct);
    assert_eq!(person_term, decoded_term);
    assert_eq!(encoded_term, encoded_struct);

    let metadata1 = Metadata {
        creator: "system".to_string(),
        timestamp: 1678886400,
    };
    let metadata2 = Metadata {
        creator: "user".to_string(),
        timestamp: 1678886460,
    };
    let metadata_map = HashMap::from([
        ("source".to_string(), metadata1.clone()),
        ("editor".to_string(), metadata2.clone()),
    ]);
    let complex_data_struct = ComplexData {
        id: 12345,
        name: "test_data".to_string(),
        is_active: true,
        tags: vec!["tag1".to_string(), "tag2".to_string()],
        metadata: metadata_map,
        raw_data: vec![0, 1, 2, 3, 4, 5],
    };

    let metadata1_term = vecpak::Term::PropList(vec![
        (vecpak::Term::Binary(b"creator".to_vec()), vecpak::Term::Binary(b"system".to_vec())),
        (vecpak::Term::Binary(b"timestamp".to_vec()), vecpak::Term::VarInt(1678886400)),
    ]);
    let metadata2_term = vecpak::Term::PropList(vec![
        (vecpak::Term::Binary(b"creator".to_vec()), vecpak::Term::Binary(b"user".to_vec())),
        (vecpak::Term::Binary(b"timestamp".to_vec()), vecpak::Term::VarInt(1678886460)),
    ]);
    let metadata_map_term = vecpak::Term::PropList(vec![
        (vecpak::Term::Binary(b"editor".to_vec()), metadata2_term),
        (vecpak::Term::Binary(b"source".to_vec()), metadata1_term),
    ]);
    let complex_data_term = vecpak::Term::PropList(vec![
        (vecpak::Term::Binary(b"id".to_vec()), vecpak::Term::VarInt(12345)),
        (vecpak::Term::Binary(b"name".to_vec()), vecpak::Term::Binary(b"test_data".to_vec())),
        (vecpak::Term::Binary(b"tags".to_vec()), vecpak::Term::List(vec![
            vecpak::Term::Binary(b"tag1".to_vec()),
            vecpak::Term::Binary(b"tag2".to_vec()),
        ])),
        (vecpak::Term::Binary(b"metadata".to_vec()), metadata_map_term),
        (vecpak::Term::Binary(b"raw_data".to_vec()), vecpak::Term::Binary(vec![0, 1, 2, 3, 4, 5])),
        (vecpak::Term::Binary(b"is_active".to_vec()), vecpak::Term::Bool(true)),
    ]);

    let encoded_term_complex = vecpak::encode(complex_data_term.clone());
    let decoded_struct_complex: ComplexData = from_slice(&encoded_term_complex).unwrap();
    let encoded_struct_complex = to_vec(&complex_data_struct).unwrap();
    let decoded_term_complex: vecpak::Term = vecpak::decode(&encoded_struct_complex).unwrap();

    assert_eq!(complex_data_struct, decoded_struct_complex);
    assert_eq!(complex_data_term, decoded_term_complex);
    assert_eq!(encoded_term_complex, encoded_struct_complex);

    println!("all tests passed");
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
struct Person {
    name: String,
    age: u32,
    active: bool,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
struct Metadata {
    creator: String,
    timestamp: i64,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
struct ComplexData {
    id: u64,
    name: String,
    is_active: bool,
    tags: Vec<String>,
    metadata: HashMap<String, Metadata>,
    #[serde(with = "serde_bytes")]
    raw_data: Vec<u8>,
}
