use rocksdb::{ColumnFamily, Transaction, DB};
use sha2::{Digest, Sha256};
use rayon::prelude::*;
use std::cmp::Ordering;
use std::collections::BTreeSet;
use std::convert::TryInto;

// ============================================================================
// TYPES
// ============================================================================

pub type Hash = [u8; 32];
pub type Path = [u8; 32];
const ZERO_HASH: Hash = [0u8; 32];

#[derive(Debug, Clone, PartialEq, Eq, Copy)]
pub struct NodeKey {
    pub path: Path,
    pub len: u16,
}

// Big Endian Sort: Path first, then Length
impl PartialOrd for NodeKey {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for NodeKey {
    fn cmp(&self, other: &Self) -> Ordering {
        match self.path.cmp(&other.path) {
            Ordering::Equal => self.len.cmp(&other.len),
            other => other,
        }
    }
}

// Added Clone just in case, though into_par_iter handles ownership
#[derive(Debug, Clone)]
pub enum Op {
    Insert(Vec<u8>, Vec<u8>),
    Delete(Vec<u8>),
}

#[derive(Debug, Clone)]
pub struct ProofNode { pub hash: Hash, pub direction: u8, pub len: u16 }

#[derive(Debug)]
pub struct Proof { pub root: Hash, pub nodes: Vec<ProofNode> }

// ============================================================================
// BIT & HASH HELPERS
// ============================================================================

#[inline(always)]
fn get_bit_be(data: &[u8], index: u16) -> u8 {
    if index >= 256 { return 0; }
    let byte_idx = (index >> 3) as usize;
    let bit_offset = 7 - (index & 7);
    (data[byte_idx] >> bit_offset) & 1
}

#[inline(always)]
fn set_bit_be(data: &mut [u8], index: u16, val: u8) {
    if index >= 256 { return; }
    let byte_idx = (index >> 3) as usize;
    let bit_offset = 7 - (index & 7);
    if val == 1 { data[byte_idx] |= 1 << bit_offset; }
    else { data[byte_idx] &= !(1 << bit_offset); }
}

#[inline]
fn mask_after_be(data: &mut [u8], len: u16) {
    if len >= 256 { return; }
    let byte_idx = (len >> 3) as usize;
    let start_clean_bit = len;
    for i in start_clean_bit..((byte_idx as u16 + 1) << 3) {
        let off = 7 - (i & 7);
        data[byte_idx] &= !(1 << off);
    }
    if byte_idx + 1 < 32 { data[(byte_idx + 1)..].fill(0); }
}

#[inline]
fn lcp_be(p1: &Path, p2: &Path) -> (Path, u16) {
    let mut len = 0;
    let mut byte_idx = 0;
    while byte_idx < 32 && p1[byte_idx] == p2[byte_idx] {
        len += 8;
        byte_idx += 1;
    }
    if byte_idx < 32 {
        for i in 0..8 {
            let idx = (byte_idx << 3) + i;
            if get_bit_be(p1, idx as u16) == get_bit_be(p2, idx as u16) { len += 1; }
            else { break; }
        }
    }
    let mut prefix = *p1;
    mask_after_be(&mut prefix, len);
    (prefix, len)
}

#[inline]
fn prefix_match_be(target: &Path, path: &Path, len: u16) -> bool {
    let full_bytes = (len >> 3) as usize;
    if target[..full_bytes] != path[..full_bytes] { return false; }
    let rem = len & 7;
    if rem > 0 {
        let mask = 0xFF << (8 - rem);
        if (target[full_bytes] & mask) != (path[full_bytes] & mask) { return false; }
    }
    true
}

#[inline]
fn sha256(data: &[u8]) -> Hash {
    let mut hasher = Sha256::new();
    hasher.update(data);
    hasher.finalize().into()
}

#[inline]
fn concat_and_hash(a: &[u8], b: &[u8]) -> Hash {
    let mut hasher = Sha256::new();
    hasher.update(a);
    hasher.update(b);
    hasher.finalize().into()
}

// ============================================================================
// ROCKSDB SERIALIZATION HELPERS
// ============================================================================

// Serialize: Path (32 bytes) + Len (2 bytes Big Endian)
#[inline]
fn serialize_key(key: &NodeKey) -> Vec<u8> {
    let mut v = Vec::with_capacity(34);
    v.extend_from_slice(&key.path);
    v.extend_from_slice(&key.len.to_be_bytes());
    v
}

#[inline]
fn deserialize_key(data: &[u8]) -> NodeKey {
    let mut path = [0u8; 32];
    path.copy_from_slice(&data[0..32]);
    let len = u16::from_be_bytes([data[32], data[33]]);
    NodeKey { path, len }
}

// ============================================================================
// ROCKSDB HUBT
// ============================================================================

pub struct RocksHubt<'a> {
    txn: &'a Transaction<'a, DB>,
    cf: &'a ColumnFamily,
}

impl<'a> RocksHubt<'a> {
    pub fn new(txn: &'a Transaction<'a, DB>, cf: &'a ColumnFamily) -> Self {
        Self { txn, cf }
    }

    pub fn root(&self) -> Hash {
        // Find smallest node (Root is usually 00..00 len 0, or smallest leaf)
        let mut iter = self.txn.raw_iterator_cf(self.cf);
        iter.seek_to_first();
        if iter.valid() {
            iter.value().unwrap().try_into().unwrap()
        } else {
            ZERO_HASH
        }
    }

    pub fn batch_update(&mut self, ops: Vec<Op>) {
        // 1. Prepare Ops (Parallel Hash)
        // Ensure 'rayon' is in Cargo.toml dependencies!
        let mut prepared: Vec<(bool, Path, Hash)> = ops.into_par_iter().map(|op| {
            match op {
                Op::Insert(k, v) => (true, sha256(&k), concat_and_hash(&k, &v)),
                Op::Delete(k) => (false, sha256(&k), ZERO_HASH)
            }
        }).collect();

        prepared.par_sort_unstable_by(|a, b| a.1.cmp(&b.1));

        // 2. Write to DB (Optimistic Logic)
        let mut dirty_set = BTreeSet::new();

        // A. Remove Old Leaves
        for (is_ins, p, _) in &prepared {
            if !*is_ins {
                let key = NodeKey { path: *p, len: 256 };
                self.remove_raw(&key);
            }
        }

        // B. Insert New Leaves
        for (is_ins, p, l) in &prepared {
            if *is_ins {
                let key = NodeKey { path: *p, len: 256 };
                self.insert_raw(key, *l);
                dirty_set.insert(key);
            }
        }

        // C. Calculate & Insert Split Points (Skeleton)
        for window in prepared.windows(2) {
            let (lcp_p, lcp_len) = lcp_be(&window[0].1, &window[1].1);
            let key = NodeKey { path: lcp_p, len: lcp_len };
            self.ensure_node_exists(key, &mut dirty_set);
        }

        // Neighbors
        for (is_ins, p, _) in &prepared {
            if *is_ins {
                self.ensure_split_points(*p, &mut dirty_set);
            }
        }

        // D. Mark Ancestors Dirty
        for (_, p, _) in &prepared {
            self.collect_dirty_ancestors(*p, &mut dirty_set);
        }

        // 3. Rehash Bottom-Up
        self.rehash_and_prune(dirty_set);
    }

    // --- HELPER LOGIC ---

    fn insert_raw(&mut self, key: NodeKey, val: Hash) {
        let k = serialize_key(&key);
        let _ = self.txn.put_cf(self.cf, k, val);
    }

    fn remove_raw(&mut self, key: &NodeKey) {
        let k = serialize_key(key);
        let _ = self.txn.delete_cf(self.cf, k);
    }

    fn exists_raw(&self, key: &NodeKey) -> bool {
        let k = serialize_key(key);
        self.txn.get_cf(self.cf, k).unwrap().is_some()
    }

    fn ensure_node_exists(&mut self, key: NodeKey, dirty: &mut BTreeSet<NodeKey>) {
        if !self.exists_raw(&key) {
            self.insert_raw(key, ZERO_HASH);
            dirty.insert(key);
        }
    }

    fn ensure_split_points(&mut self, path: Path, dirty: &mut BTreeSet<NodeKey>) {
        let key = NodeKey { path, len: 256 };

        // Seek Prev
        if let Some((n_key, _)) = self.seek_prev(&key) {
            if n_key.len == 256 {
                let (lcp_p, lcp_l) = lcp_be(&path, &n_key.path);
                self.ensure_node_exists(NodeKey { path: lcp_p, len: lcp_l }, dirty);
            }
        }
        // Seek Next
        if let Some((n_key, _)) = self.seek_next(&key) {
            if n_key.len == 256 {
                let (lcp_p, lcp_l) = lcp_be(&path, &n_key.path);
                self.ensure_node_exists(NodeKey { path: lcp_p, len: lcp_l }, dirty);
            }
        }
    }

    fn collect_dirty_ancestors(&self, target_path: Path, dirty: &mut BTreeSet<NodeKey>) {
        let mut cursor = NodeKey { path: target_path, len: 256 };

        loop {
            // "seek_prev" finds node <= cursor
            match self.seek_prev(&cursor) {
                None => break,
                Some((k, _)) => {
                    let is_same = k == cursor;

                    if prefix_match_be(&target_path, &k.path, k.len) {
                        dirty.insert(k);
                        // Move cursor strictly before K to continue up
                        if k.len > 0 {
                            cursor = NodeKey { path: k.path, len: k.len - 1 };
                        } else {
                            break; // Root reached
                        }
                    } else {
                        // Mismatch -> Jump
                        let (lcp_path, lcp_len) = lcp_be(&target_path, &k.path);
                        let jump_key = NodeKey { path: lcp_path, len: lcp_len + 1 };

                        if jump_key < k {
                            cursor = jump_key;
                        } else {
                            if is_same {
                                if k.len > 0 { cursor = NodeKey{path: k.path, len: k.len - 1}; } else { break; }
                            } else {
                                cursor = k;
                            }
                        }
                    }
                }
            }
        }
    }

    fn rehash_and_prune(&mut self, dirty_nodes: BTreeSet<NodeKey>) {
        let mut sorted_nodes: Vec<NodeKey> = dirty_nodes.into_iter().collect();
        // Bottom-up sort
        sorted_nodes.sort_unstable_by(|a, b| b.len.cmp(&a.len));

        for node in sorted_nodes {
            if node.len == 256 { continue; }

            // Left Child (0)
            let mut l_path = node.path;
            set_bit_be(&mut l_path, node.len, 0);
            mask_after_be(&mut l_path, node.len + 1);
            let l_key = NodeKey { path: l_path, len: node.len + 1 };

            let l_hash = self.seek_next(&l_key)
                .filter(|(k, _)| prefix_match_be(&k.path, &l_path, node.len + 1))
                .map(|(_, h)| h).unwrap_or(ZERO_HASH);

            // Right Child (1)
            let mut r_path = node.path;
            set_bit_be(&mut r_path, node.len, 1);
            mask_after_be(&mut r_path, node.len + 1);
            let r_key = NodeKey { path: r_path, len: node.len + 1 };

            let r_hash = self.seek_next(&r_key)
                .filter(|(k, _)| prefix_match_be(&k.path, &r_path, node.len + 1))
                .map(|(_, h)| h).unwrap_or(ZERO_HASH);

            if l_hash != ZERO_HASH && r_hash != ZERO_HASH {
                self.insert_raw(node, concat_and_hash(&l_hash, &r_hash));
            } else {
                self.remove_raw(&node);
            }
        }
    }

    // --- ITERATOR WRAPPERS ---

    fn seek_prev(&self, key: &NodeKey) -> Option<(NodeKey, Hash)> {
        let k_bytes = serialize_key(key);
        let mut iter = self.txn.raw_iterator_cf(self.cf);
        iter.seek_for_prev(k_bytes);

        if iter.valid() {
            let found_k = deserialize_key(iter.key().unwrap());
            let found_v: Hash = iter.value().unwrap().try_into().unwrap();
            Some((found_k, found_v))
        } else {
            None
        }
    }

    fn seek_next(&self, key: &NodeKey) -> Option<(NodeKey, Hash)> {
        let k_bytes = serialize_key(key);
        let mut iter = self.txn.raw_iterator_cf(self.cf);
        iter.seek(k_bytes);

        if iter.valid() {
            let found_k = deserialize_key(iter.key().unwrap());
            if found_k == *key {
                iter.next();
            }
        }

        if iter.valid() {
            let found_k = deserialize_key(iter.key().unwrap());
            let found_v: Hash = iter.value().unwrap().try_into().unwrap();
            Some((found_k, found_v))
        } else {
            None
        }
    }

    // --- PROOF ---

    pub fn prove(&self, k: Vec<u8>, v: Vec<u8>) -> Option<Proof> {
        let path = sha256(&k);
        let leaf_val = concat_and_hash(&k, &v);

        let key = NodeKey { path, len: 256 };
        if let Some(h) = self.txn.get_cf(self.cf, serialize_key(&key)).ok().flatten() {
            if h == &leaf_val[..] {
                return Some(Proof {
                    root: self.root(),
                    nodes: self.generate_proof_nodes(path, 256),
                });
            }
        }
        None
    }

    fn generate_proof_nodes(&self, path: Path, len: u16) -> Vec<ProofNode> {
        let mut ancestors = Vec::new();
        let mut cursor = NodeKey { path, len: 256 };

        loop {
            match self.seek_prev(&cursor) {
                None => break,
                Some((k, _)) => {
                    let is_same = k == cursor;
                    if prefix_match_be(&path, &k.path, k.len) {
                        if k.len < len { ancestors.push(k); }
                        if k.len > 0 { cursor = NodeKey{path: k.path, len: k.len-1}; } else { break; }
                    } else {
                         let (lcp_p, lcp_l) = lcp_be(&path, &k.path);
                         let jump = NodeKey{ path: lcp_p, len: lcp_l + 1 };
                         if jump < k { cursor = jump; }
                         else if is_same {
                             if k.len > 0 { cursor = NodeKey{path:k.path, len:k.len-1}; } else { break; }
                         } else { cursor = k; }
                    }
                }
            }
        }
        ancestors.sort_unstable_by(|a, b| b.len.cmp(&a.len));

        let mut nodes = Vec::new();
        for anc in ancestors {
            let my_dir = get_bit_be(&path, anc.len);
            let sibling_dir = 1 - my_dir;

            let mut t_path = anc.path;
            set_bit_be(&mut t_path, anc.len, sibling_dir);
            mask_after_be(&mut t_path, anc.len + 1);
            let t_key = NodeKey { path: t_path, len: anc.len + 1 };

            let s_hash = self.seek_next(&t_key)
                .filter(|(k, _)| prefix_match_be(&k.path, &t_path, anc.len + 1))
                .map(|(_, h)| h).unwrap_or(ZERO_HASH);

            nodes.push(ProofNode { hash: s_hash, direction: sibling_dir, len: anc.len });
        }
        nodes
    }
}
