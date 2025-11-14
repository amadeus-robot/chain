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
  
### TODO  
  
A serde implementation so Term Enum can be skipped, HashMap properly supported and RUST types directly mapped.  
Decode/Encode bigger numbers than i128 for VarInt.  

### Usage

```rust
use vecpak::{encode, decode, Term};

encode(Term::Binary(b"peter piper picked a pack of pickled peppers"))?;
encode(Term::VarInt(67 as i128))?;
let bin = encode(Term::Map(
    [
        (Term::Binary(b"fruit"), Term::Binary(b"apple")),
        (Term::Binary(b"color"), Term::Binary(b"red")),
        (Term::Binary(b"amount"), Term::VarInt(1 as i128)),
    ]
))?;

decode(bin.as_slice())?;
```
