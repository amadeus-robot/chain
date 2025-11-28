use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use vecpak::{from_slice, to_vec};

fn main() {
    let person_struct = Person {
        name: "Alice".to_string(),
        age: 30,
        active: true,
    };

    let person_term = vecpak::Term::PropList(vec![
        (
            vecpak::Term::Binary(b"age".to_vec()),
            vecpak::Term::VarInt(30),
        ),
        (
            vecpak::Term::Binary(b"name".to_vec()),
            vecpak::Term::Binary(b"Alice".to_vec()),
        ),
        (
            vecpak::Term::Binary(b"active".to_vec()),
            vecpak::Term::Bool(true),
        ),
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
        (
            vecpak::Term::Binary(b"creator".to_vec()),
            vecpak::Term::Binary(b"system".to_vec()),
        ),
        (
            vecpak::Term::Binary(b"timestamp".to_vec()),
            vecpak::Term::VarInt(1678886400),
        ),
    ]);
    let metadata2_term = vecpak::Term::PropList(vec![
        (
            vecpak::Term::Binary(b"creator".to_vec()),
            vecpak::Term::Binary(b"user".to_vec()),
        ),
        (
            vecpak::Term::Binary(b"timestamp".to_vec()),
            vecpak::Term::VarInt(1678886460),
        ),
    ]);
    let metadata_map_term = vecpak::Term::PropList(vec![
        (vecpak::Term::Binary(b"editor".to_vec()), metadata2_term),
        (vecpak::Term::Binary(b"source".to_vec()), metadata1_term),
    ]);
    let complex_data_term = vecpak::Term::PropList(vec![
        (
            vecpak::Term::Binary(b"id".to_vec()),
            vecpak::Term::VarInt(12345),
        ),
        (
            vecpak::Term::Binary(b"name".to_vec()),
            vecpak::Term::Binary(b"test_data".to_vec()),
        ),
        (
            vecpak::Term::Binary(b"tags".to_vec()),
            vecpak::Term::List(vec![
                vecpak::Term::Binary(b"tag1".to_vec()),
                vecpak::Term::Binary(b"tag2".to_vec()),
            ]),
        ),
        (
            vecpak::Term::Binary(b"metadata".to_vec()),
            metadata_map_term,
        ),
        (
            vecpak::Term::Binary(b"raw_data".to_vec()),
            vecpak::Term::Binary(vec![0, 1, 2, 3, 4, 5]),
        ),
        (
            vecpak::Term::Binary(b"is_active".to_vec()),
            vecpak::Term::Bool(true),
        ),
    ]);

    let encoded_term_complex = vecpak::encode(complex_data_term.clone());
    let decoded_struct_complex: ComplexData = from_slice(&encoded_term_complex).unwrap();
    let encoded_struct_complex = to_vec(&complex_data_struct).unwrap();
    let decoded_term_complex: vecpak::Term = vecpak::decode(&encoded_struct_complex).unwrap();

    assert_eq!(complex_data_struct, decoded_struct_complex);
    assert_eq!(complex_data_term, decoded_term_complex);
    assert_eq!(encoded_term_complex, encoded_struct_complex);

    let mutations_struct = Mutations(vec![
        Mutation::Put {
            key: b"key1".to_vec(),
            value: b"value1".to_vec(),
        },
        Mutation::Delete {
            key: b"key2".to_vec(),
        },
        Mutation::SetBit {
            key: b"key3".to_vec(),
            value: 42,
            bloomsize: 1024,
        },
        Mutation::ClearBit {
            key: b"key4".to_vec(),
            value: 7,
        },
    ]);

    let mutations_term = vecpak::Term::List(vec![
        vecpak::Term::PropList(vec![
            (
                vecpak::Term::Binary(b"op".to_vec()),
                vecpak::Term::Binary(b"put".to_vec()),
            ),
            (
                vecpak::Term::Binary(b"key".to_vec()),
                vecpak::Term::Binary(b"key1".to_vec()),
            ),
            (
                vecpak::Term::Binary(b"value".to_vec()),
                vecpak::Term::Binary(b"value1".to_vec()),
            ),
        ]),
        vecpak::Term::PropList(vec![
            (
                vecpak::Term::Binary(b"op".to_vec()),
                vecpak::Term::Binary(b"delete".to_vec()),
            ),
            (
                vecpak::Term::Binary(b"key".to_vec()),
                vecpak::Term::Binary(b"key2".to_vec()),
            ),
        ]),
        vecpak::Term::PropList(vec![
            (
                vecpak::Term::Binary(b"op".to_vec()),
                vecpak::Term::Binary(b"set_bit".to_vec()),
            ),
            (
                vecpak::Term::Binary(b"key".to_vec()),
                vecpak::Term::Binary(b"key3".to_vec()),
            ),
            (
                vecpak::Term::Binary(b"value".to_vec()),
                vecpak::Term::VarInt(42),
            ),
            (
                vecpak::Term::Binary(b"bloomsize".to_vec()),
                vecpak::Term::VarInt(1024),
            ),
        ]),
        vecpak::Term::PropList(vec![
            (
                vecpak::Term::Binary(b"op".to_vec()),
                vecpak::Term::Binary(b"clear_bit".to_vec()),
            ),
            (
                vecpak::Term::Binary(b"key".to_vec()),
                vecpak::Term::Binary(b"key4".to_vec()),
            ),
            (
                vecpak::Term::Binary(b"value".to_vec()),
                vecpak::Term::VarInt(7),
            ),
        ]),
    ]);

    let encoded_term_mutations = vecpak::encode(mutations_term.clone());
    let decoded_struct_mutations: Mutations = from_slice(&encoded_term_mutations).unwrap();
    let encoded_struct_mutations = to_vec(&mutations_struct).unwrap();
    let decoded_term_mutations: vecpak::Term = vecpak::decode(&encoded_struct_mutations).unwrap();

    assert_eq!(mutations_struct, decoded_struct_mutations);
    assert_eq!(mutations_term, decoded_term_mutations);
    assert_eq!(encoded_term_mutations, encoded_struct_mutations);

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

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
enum Mutation {
    Put {
        #[serde(with = "serde_bytes")]
        key: Vec<u8>,
        #[serde(with = "serde_bytes")]
        value: Vec<u8>,
    },
    Delete {
        #[serde(with = "serde_bytes")]
        key: Vec<u8>,
    },
    SetBit {
        #[serde(with = "serde_bytes")]
        key: Vec<u8>,
        value: u64,
        bloomsize: u64,
    },
    ClearBit {
        #[serde(with = "serde_bytes")]
        key: Vec<u8>,
        value: u64,
    },
}

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
struct Mutations(Vec<Mutation>);
