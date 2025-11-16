# chain

## VecPak Bindings

| Language | Link                                 |
| -------- | ------------------------------------ |
| Rust   | [README](vecpak/README.md)                    |
| Elixir   | [README](vecpak/bindings/ex/README.md)      |
| JS       | [README](vecpak/bindings/js/README.md)      |
  

### On-Chain Utilities and Hub

```
RUSTFLAGS="-C target-cpu=native" cargo test --release
RUSTFLAGS="-C target-cpu=native" cargo run --release
```

```
https://eips.ethereum.org/EIPS/eip-7864

bintree/rs_0
10m elements initial | 55409 ms
10k update | 6 ms

Possibly improvements to segment state/ tx/ contract/account/<pk> into seperate branchs (so hot accounts dont thrash the entire tree)
```
