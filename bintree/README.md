# Hot Unified Binary Tree
An unordered binary tree with hot paths 

<img width="1024" height="1024" alt="image" src="https://github.com/user-attachments/assets/d225737a-cca0-4e64-840a-2cc8c0a8385a" />

### Inspiration
https://eips.ethereum.org/EIPS/eip-7864  
https://github.com/varun-doshi/eth-binary-tree  
  
### Features
Default Hash: sha2  
First 8bytes is hotpath aka namespace, next 24bytes is key  
Insert 32byte key and 32byte value  
Delete 32byte key - deterministic prune 
Membership Proof (k/v exists)
Non-Membership Proof (k missing)
Mismatch Proof (k exists v changed)

```
RUSTFLAGS="-C target-cpu=native" cargo test --release -- --nocapture
RUSTFLAGS="-C target-cpu=native" cargo run --release

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
```

## HUBT Performance
<img width="955" height="564" alt="image" src="https://github.com/user-attachments/assets/b34ce16b-6311-4a84-acc6-d87b749684f9" />

## Node Structure

| Architecture | On-Disk Key | On-Disk Value | Total Bytes (Approx) | Structural Overhead |
| :--- | :--- | :--- | :--- | :--- |
| **Ethereum MPT** | 32B (Hash of Node) | RLP List: `[Hash0, Hash1... Hash15, Val]` | **~500 - 600 Bytes** | **Critical** (Stores 16 fixed slots for pointers) |
| **Jellyfish (JMT)** | 36B (Version + Path) | `[Bitmask(4B) + LeftHash? + RightHash?]` | **~40 - 70 Bytes** | **Moderate** (Explicit pointers/masks stored) |
| **HUBT** | 34B (Path + Len) | `[Hash]` | **66 Bytes (Fixed)** | **Zero** (Topology derived from **Sort Order**) |


## Storage at 1 Billion Items

| Metric | Ethereum MPT (Hexary) | JMT (Binary) | HUBT (Linearized) |
| :--- | :--- | :--- | :--- |
| **Tree Depth** | ~7-8 Levels | ~30 Levels | ~30 Levels |
| **Total Internal Nodes** | ~1.2 Billion | ~2 Billion | ~2 Billion |
| **Raw Disk Usage** | ~600 GB | ~120 GB | ~132 GB |
| **Compression Potential** | **0%** (High Entropy Hashes in Keys) | **Medium** (Sorted Path Prefixes) | **High** (**Superior** Prefix Compression) |
| **Est. Final Size** | **~600 GB** | **~80 GB** | **~45 - 60 GB** |


## CPU & I/O

| Metric | Ethereum MPT | JMT (Aptos/Sui) | HUBT |
| :--- | :--- | :--- | :--- |
| **Traversal Logic** | **Random Pointer Chasing** | **Key Lookup per Level** | **Iterator Sliding / LCP Jump** |
| **DB Operation** | `DB.Get(Hash)` | `DB.Get(Version+Path)` | `Iterator.Next()` / `Prev()` |
| **Disk IOPS req.** | **7-8 Random Seeks** | **~30 Mixed Seeks** | **1 Seek + Sequential Reads** |
| **Cache Locality** | **None** (Scattered data) | **Medium** | **Extreme** (Nodes are contiguous) |
| **Latency Bottleneck** | Disk Seek Latency | High Call Overhead & Binary Depth | **RAM / Decompression Speed** |
