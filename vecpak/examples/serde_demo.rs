use serde::{Serialize, Deserialize};
use vecpak::{to_vec, from_slice};
use std::collections::HashMap;

#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct Person {
    name: String,
    age: u32,
    active: bool,
}

fn main() {
    let num: i64 = 42;
    let bytes = to_vec(&num).unwrap();
    let decoded: i64 = from_slice(&bytes).unwrap();
    assert_eq!(num, decoded);

    let text = "hello world";
    let bytes = to_vec(&text).unwrap();
    let decoded: String = from_slice(&bytes).unwrap();
    assert_eq!(text, decoded);

    let person = Person {
        name: "Alice".to_string(),
        age: 30,
        active: true,
    };
    let bytes = to_vec(&person).unwrap();
    let decoded: Person = from_slice(&bytes).unwrap();
    assert_eq!(person, decoded);

    let list = vec![1, 2, 3, 4, 5];
    let bytes = to_vec(&list).unwrap();
    let decoded: Vec<i32> = from_slice(&bytes).unwrap();
    assert_eq!(list, decoded);

    let mut map = HashMap::new();
    map.insert("fruit".to_string(), "apple".to_string());
    map.insert("color".to_string(), "red".to_string());
    let bytes = to_vec(&map).unwrap();
    let decoded: HashMap<String, String> = from_slice(&bytes).unwrap();
    assert_eq!(map, decoded);

    let some: Option<i32> = Some(123);
    let bytes = to_vec(&some).unwrap();
    let decoded: Option<i32> = from_slice(&bytes).unwrap();
    assert_eq!(some, decoded);

    let none: Option<i32> = None;
    let bytes = to_vec(&none).unwrap();
    let decoded: Option<i32> = from_slice(&bytes).unwrap();
    assert_eq!(none, decoded);

    println!("all tests passed");
}
