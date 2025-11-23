# chain

## VecPak Bindings

| Language | Link                                 |
| -------- | ------------------------------------ |
| Rust   | [README](vecpak/README.md)                    |
| Elixir   | [README](vecpak/bindings/ex/README.md)      |
| JS       | [README](vecpak/bindings/js/README.md)      |
  

### On-Chain Utilities and Hub

```
RUSTFLAGS="-C target-cpu=native" cargo test --release -- --nocapture
RUSTFLAGS="-C target-cpu=native" cargo run --release
```

```
https://eips.ethereum.org/EIPS/eip-7864

bintree
Possibly improvements to segment state/ tx/ contract/account/<pk> into seperate branchs (so hot accounts dont thrash the entire tree)

Tree filled (1M items) in: 5477.802334ms

Batch Size   | Time Taken           | Key Range           
-------------+----------------------+---------------------
100          | 1.324716ms           | 1000000 .. 1000100  
1000         | 17.642214ms          | 1000100 .. 1001100  
10000        | 122.972819ms         | 1001100 .. 1011100  
100000       | 950.695083ms         | 1011100 .. 1111100 

Generated 1000 proofs in: 9.860575ms
Average time per proof: 9.86µs
Proofs per second: 101413.96



## HUBT
---

## 1. Node Anatomy Comparison

| Architecture | On-Disk Key | On-Disk Value | Total Bytes (Approx) | Structural Overhead |
| :--- | :--- | :--- | :--- | :--- |
| **Ethereum MPT** | 32B (Hash of Node) | RLP List: `[Hash0, Hash1... Hash15, Val]` | **~500 - 600 Bytes** | **Critical** (Stores 16 fixed slots for pointers) |
| **Jellyfish (JMT)** | 36B (Version + Path) | `[Bitmask(4B) + LeftHash? + RightHash?]` | **~40 - 70 Bytes** | **Moderate** (Explicit pointers/masks stored) |
| **HUBT** | 34B (Path + Len) | `[Hash]` | **66 Bytes (Fixed)** | **Zero** (Topology derived from **Sort Order**) |

---

## 2. Storage Impact at 1 Billion Items

| Metric | Ethereum MPT (Hexary) | JMT (Binary) | HUBT (Linearized) |
| :--- | :--- | :--- | :--- |
| **Tree Depth** | ~7-8 Levels | ~30 Levels | ~30 Levels |
| **Total Internal Nodes** | ~1.2 Billion | ~2 Billion | ~2 Billion |
| **Raw Disk Usage** | ~600 GB | ~120 GB | ~132 GB |
| **Compression Potential** | **0%** (High Entropy Hashes in Keys) | **Medium** (Sorted Path Prefixes) | **High** (**Superior** Prefix Compression) |
| **Est. Final Size** | **~600 GB** | **~80 GB** | **~45 - 60 GB** |

---

## 3. CPU & I/O Performance

| Metric | Ethereum MPT | JMT (Aptos/Sui) | HUBT |
| :--- | :--- | :--- | :--- |
| **Traversal Logic** | **Random Pointer Chasing** | **Key Lookup per Level** | **Iterator Sliding / LCP Jump** |
| **DB Operation** | `DB.Get(Hash)` | `DB.Get(Version+Path)` | `Iterator.Next()` / `Prev()` |
| **Disk IOPS req.** | **7-8 Random Seeks** | **~30 Mixed Seeks** | **1 Seek + Sequential Reads** |
| **Cache Locality** | **None** (Scattered data) | **Medium** | **Extreme** (Nodes are contiguous) |
| **Latency Bottleneck** | Disk Seek Latency | High Call Overhead & Binary Depth | **RAM / Decompression Speed** |

---

### Structure Representation

```text
[ VISUALIZING THE STORAGE DIFFERENCE ]

1. Standard Merkle Tree (Ethereum MPT)
   [KEY: Hash A] -> [VALUE: Ptr to B, Ptr to C, Ptr to D...]
   
   * Requires storing "Arrows" (Pointers) within the Value.
   * Nodes A, B, C, D are scattered randomly on disk, forcing **Random I/O**.

2. HUBT (Linearized)
   [KEY: Path A | Len 1] -> [VALUE: Hash]
   [KEY: Path A | Len 2] -> [VALUE: Hash]
   [KEY: Path A | Len 3] -> [VALUE: Hash]
   
   * **Zero** "Arrows" stored.
   * Connection is derived by the sort order.
   * Nodes are **physically adjacent** on disk, allowing **Sequential I/O**.
```
