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
