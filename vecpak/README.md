# VecPak
A deterministic no-schema simple serializer  

<img width="1024" height="1024" alt="image" src="https://github.com/user-attachments/assets/7e5c7eee-e338-47f1-b706-d696cf3e3627" />


### Standards

First try the ones below  
  
RLP  
dCBOR  
BCS  
Cosmos ADR-027  
SCALE  
Borsh  
SSZ  
  
If they dont fit your use case continue reading.  

### Spec

```
7 Type Tags

0 nil
1 false
2 true
3 VarInt
5 Binary
6 List
7 Map
```

4 is unlucky so its not included as a tag, it gives you a 2bit ECC error when used.  
  
Map keys can be anything (making Tuple usage easy) and are ordered by encoded byte representation.

### Features

- Serde implementation - serialize/deserialize any Rust type directly
- HashMap support with canonical key ordering
- Direct type mapping without Term enum

### Limitations

- Floats not supported
- VarInt numbers >i128 are not supported

### Usage

```rust
use serde::{Serialize, Deserialize};
use vecpak::{to_vec, from_slice};
use std::collections::HashMap;

#[derive(Serialize, Deserialize)]
struct MyData {
    name: String,
    age: u32,
    active: bool,
}

let data = MyData { name: "Alice".into(), age: 30, active: true };
let bytes = vecpak::to_vec(&data)?;
let decoded: MyData = vecpak::from_slice(&bytes)?;
```