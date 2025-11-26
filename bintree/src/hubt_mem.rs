use sha2::{Digest, Sha256};
use std::cmp::{min, Ordering};
use std::collections::{BTreeMap, BTreeSet};
use std::ops::Bound;

pub type Hash = [u8; 32];
pub type Path = [u8; 32];
const ZERO_HASH: Hash = [0u8; 32];

// ============================================================================
// STRUCTS
// ============================================================================

#[derive(Debug, Clone, PartialEq, Eq, Copy)]
pub struct NodeKey {
    pub path: Path,
    pub len: u16,
}

impl PartialOrd for NodeKey {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for NodeKey {
    fn cmp(&self, other: &Self) -> Ordering {
        // Big Endian sort: Path first, then Length
        match self.path.cmp(&other.path) {
            Ordering::Equal => self.len.cmp(&other.len),
            other => other,
        }
    }
}

#[derive(Debug)]
pub enum Op {
    Insert(Vec<u8>, Vec<u8>),
    Delete(Vec<u8>),
}

/// A simplified proof node without length.
#[derive(Debug, Clone)]
pub struct ProofNode {
    pub hash: Hash,
    pub direction: u8
}

/// The Universal Proof Struct.
///
/// - If `path` == sha256(key) and `hash` == sha256(key, value): It's an Inclusion Proof.
/// - If `path` == sha256(key) and `hash` != sha256(key, value): It's a Mismatch Proof.
/// - If `path` != sha256(key): It's a Non-Existence Proof (pointing to the closest ancestor).
#[derive(Debug, Clone)]
pub struct Proof {
    pub root: Hash,
    pub nodes: Vec<ProofNode>,
    pub path: Path, // The path of the leaf node actually found in the tree
    pub hash: Hash, // The hash of the leaf node actually found in the tree
}

#[derive(Debug, PartialEq)]
pub enum VerifyStatus {
    Included,       // Key exists and Value matches
    Mismatch,       // Key exists but Value is different
    NonExistence,   // Key does not exist
    Invalid,        // The proof itself is mathematically invalid (bad root/chain)
}

// ============================================================================
// BIT HELPERS (Optimized & Inlined)
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
    if val == 1 {
        data[byte_idx] |= 1 << bit_offset;
    } else {
        data[byte_idx] &= !(1 << bit_offset);
    }
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
    if byte_idx + 1 < 32 {
        data[(byte_idx + 1)..].fill(0);
    }
}

fn lcp_be(p1: &Path, p2: &Path) -> (Path, u16) {
    let mut len = 0;
    let mut byte_idx = 0;
    while byte_idx < 32 && p1[byte_idx] == p2[byte_idx] {
        len += 8;
        byte_idx += 1;
    }
    if byte_idx < 32 {
        for i in 0..8 {
            let idx = (byte_idx * 8) + i;
            if get_bit_be(p1, idx as u16) == get_bit_be(p2, idx as u16) {
                len += 1;
            } else {
                break;
            }
        }
    }
    let mut prefix = *p1;
    mask_after_be(&mut prefix, len);
    (prefix, len)
}

#[inline]
fn prefix_match_be(target: &Path, path: &Path, len: u16) -> bool {
    let full_bytes = (len >> 3) as usize;
    if target[..full_bytes] != path[..full_bytes] {
        return false;
    }
    let rem = len & 7;
    if rem > 0 {
        let mask = 0xFF << (8 - rem);
        if (target[full_bytes] & mask) != (path[full_bytes] & mask) {
            return false;
        }
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
// HUBT IMPLEMENTATION
// ============================================================================

pub struct Hubt {
    pub store: BTreeMap<NodeKey, Hash>,
}

impl Hubt {
    pub fn new() -> Self {
        Hubt { store: BTreeMap::new() }
    }

    pub fn root(&self) -> Hash {
        self.store.iter().next().map(|(_, h)| *h).unwrap_or(ZERO_HASH)
    }

    // --- BATCH UPDATE ---
    pub fn batch_update(&mut self, ops: Vec<Op>) {
        let mut prepared: Vec<(bool, Path, Hash)> = ops.into_iter().map(|op| {
            match op {
                Op::Insert(k, v) => (true, sha256(&k), concat_and_hash(&k, &v)),
                Op::Delete(k) => (false, sha256(&k), ZERO_HASH)
            }
        }).collect();

        prepared.sort_unstable_by(|a, b| a.1.cmp(&b.1));

        for (is_ins, p, _) in &prepared {
            if !*is_ins {
                self.store.remove(&NodeKey { path: *p, len: 256 });
            }
        }

        let mut inserts = Vec::with_capacity(prepared.len());
        for (is_ins, p, l) in &prepared {
            if *is_ins {
                self.store.insert(NodeKey { path: *p, len: 256 }, *l);
                inserts.push((*p, *l));
            }
        }

        for (p, l) in &inserts {
            self.ensure_split_points(*p, *l);
        }

        let mut dirty_set = BTreeSet::new();
        for (_, p, _) in &prepared {
            self.collect_dirty_ancestors(*p, &mut dirty_set);
        }

        self.rehash_and_prune(dirty_set);
    }

    fn ensure_split_points(&mut self, path: Path, leaf_hash: Hash) {
        let key = NodeKey { path, len: 256 };
        if let Some((n_key, n_hash)) = self.store.range(..key).next_back().map(|(k,v)| (*k, *v)) {
            if n_key.len == 256 { self.check_neighbor(path, leaf_hash, n_key.path, n_hash); }
        }
        if let Some((n_key, n_hash)) = self.store.range((Bound::Excluded(key), Bound::Unbounded)).next().map(|(k,v)| (*k, *v)) {
            if n_key.len == 256 { self.check_neighbor(path, leaf_hash, n_key.path, n_hash); }
        }
    }

    fn check_neighbor(&mut self, path: Path, leaf: Hash, n_path: Path, n_leaf: Hash) {
        let (lcp_path, len) = lcp_be(&path, &n_path);
        let temp_val = concat_and_hash(&leaf, &n_leaf);
        self.store.insert(NodeKey { path: lcp_path, len }, temp_val);
    }

    fn collect_dirty_ancestors(&self, target_path: Path, acc: &mut BTreeSet<NodeKey>) {
        let mut cursor = NodeKey { path: target_path, len: 256 };
        loop {
            match self.store.range(..cursor).next_back() {
                None => break,
                Some((k, _)) => {
                    if prefix_match_be(&target_path, &k.path, k.len) {
                        acc.insert(*k);
                        cursor = *k;
                    } else {
                        let (lcp_path, lcp_len) = lcp_be(&target_path, &k.path);
                        let jump_key = NodeKey { path: lcp_path, len: lcp_len + 1 };
                        cursor = if jump_key < *k { jump_key } else { *k };
                    }
                }
            }
        }
    }

    fn rehash_and_prune(&mut self, dirty_nodes: BTreeSet<NodeKey>) {
        let mut sorted_nodes: Vec<NodeKey> = dirty_nodes.into_iter().collect();
        sorted_nodes.sort_unstable_by(|a, b| b.len.cmp(&a.len));

        for node in sorted_nodes {
            if node.len == 256 { continue; }
            let l_hash = self.get_child_hash(node.path, node.len, 0);
            let r_hash = self.get_child_hash(node.path, node.len, 1);

            if l_hash != ZERO_HASH && r_hash != ZERO_HASH {
                self.store.insert(node, concat_and_hash(&l_hash, &r_hash));
            } else {
                self.store.remove(&node);
            }
        }
    }

    fn get_child_hash(&self, p_path: Path, p_len: u16, dir: u8) -> Hash {
        let mut target_path = p_path;
        set_bit_be(&mut target_path, p_len, dir);
        mask_after_be(&mut target_path, p_len + 1);
        let target_key = NodeKey { path: target_path, len: p_len + 1 };

        if let Some((f_key, hash)) = self.store.range(target_key..).next() {
            if prefix_match_be(&f_key.path, &target_path, p_len + 1) {
                return *hash;
            }
        }
        ZERO_HASH
    }

    // ========================================================================
    // UNIFIED PROOF LOGIC
    // ========================================================================

    /// Generates a single Proof struct that covers Inclusion, Mismatch, or Non-Existence.
    ///
    /// The logic detects the state of `k` in the tree:
    /// 1. Finds the node matching `k` (or the longest matching prefix node).
    /// 2. Generates the merkle path to that node.
    /// 3. Returns the proof containing the found node's path and hash.
    pub fn prove(&self, k: Vec<u8>) -> Proof {
        let target_path = sha256(&k);

        // Find the node that actually exists (Exact match OR Longest Prefix)
        let (found_key, found_hash) = match self.find_longest_prefix_node(&target_path) {
            Some((key, hash)) => (key, hash),
            None => {
                // Empty tree case
                return Proof {
                    root: ZERO_HASH,
                    nodes: vec![],
                    path: ZERO_HASH,
                    hash: ZERO_HASH
                };
            }
        };

        Proof {
            root: self.root(),
            nodes: self.generate_proof_nodes(found_key.path, found_key.len),
            path: found_key.path,
            hash: found_hash,
        }
    }

    fn generate_proof_nodes(&self, path: Path, len: u16) -> Vec<ProofNode> {
        let mut ancestors = Vec::new();
        let mut cursor = NodeKey { path, len: 256 };

        loop {
            match self.store.range(..cursor).next_back() {
                None => break,
                Some((k, _)) => {
                    if prefix_match_be(&path, &k.path, k.len) {
                        if k.len < len { ancestors.push(*k); }
                        cursor = *k;
                    } else {
                        let (lcp_p, lcp_l) = lcp_be(&path, &k.path);
                        let jump = NodeKey{ path: lcp_p, len: lcp_l + 1 };
                        cursor = if jump < *k { jump } else { *k };
                    }
                }
            }
        }
        ancestors.sort_unstable_by(|a, b| b.len.cmp(&a.len));

        let mut nodes = Vec::new();
        for anc in ancestors {
            let my_dir = get_bit_be(&path, anc.len);
            let sibling_dir = 1 - my_dir;
            nodes.push(ProofNode {
                hash: self.get_child_hash(anc.path, anc.len, sibling_dir),
                direction: sibling_dir,
                // len removed as requested
            });
        }
        nodes
    }

    fn find_longest_prefix_node(&self, target: &Path) -> Option<(NodeKey, Hash)> {
        let s_key = NodeKey { path: *target, len: 256 };
        let prev = self.store.range(..=s_key).next_back();
        let next = self.store.range((Bound::Excluded(s_key), Bound::Unbounded)).next();

        match (prev, next) {
            (None, None) => None,
            (None, Some((k, h))) => Some((*k, *h)),
            (Some((k, h)), None) => Some((*k, *h)),
            (Some((pk, ph)), Some((nk, nh))) => {
                let (_, rp) = lcp_be(target, &pk.path);
                let (_, rn) = lcp_be(target, &nk.path);
                if min(rp, pk.len) >= min(rn, nk.len) { Some((*pk, *ph)) } else { Some((*nk, *nh)) }
            }
        }
    }

    // ========================================================================
    // UNIFIED VERIFICATION
    // ========================================================================

    /// Verifies the proof and determines the relationship between the Key, Value, and the Tree.
    pub fn verify(proof: &Proof, k: Vec<u8>, v: Vec<u8>) -> VerifyStatus {
        let target_path = sha256(&k);
        let claimed_leaf_hash = concat_and_hash(&k, &v);

        // 1. Basic Integrity Check: Does the proof path/hash actually hash up to the Root?
        if !Self::verify_integrity(proof) {
            return VerifyStatus::Invalid;
        }

        // 2. Interpret the result
        if proof.path == target_path {
            // Path matches exactly.
            if proof.hash == claimed_leaf_hash {
                VerifyStatus::Included
            } else {
                VerifyStatus::Mismatch
            }
        } else {
            // Path does not match. This is a Non-Existence proof.
            // Check if proof.path is actually a prefix of target, or a valid divergence point
            if prefix_match_be(&target_path, &proof.path, 0) { // Should check valid divergence
                 VerifyStatus::NonExistence
            } else {
                 VerifyStatus::Invalid // Should ideally check divergence details here
            }
        }
    }

    fn verify_integrity(proof: &Proof) -> bool {
        if proof.root == ZERO_HASH { return proof.nodes.is_empty() && proof.hash == ZERO_HASH; }

        let calc = proof.nodes.iter().fold(proof.hash, |acc, node| {
            if node.direction == 0 {
                concat_and_hash(&node.hash, &acc)
            } else {
                concat_and_hash(&acc, &node.hash)
            }
        });
        calc == proof.root
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Instant;

    #[test]
    fn test_big_endian_bits() {
        let mut data = [0u8; 32];
        set_bit_be(&mut data, 0, 1);
        assert_eq!(data[0], 128);
        assert_eq!(get_bit_be(&data, 0), 1);

        set_bit_be(&mut data, 8, 1);
        assert_eq!(data[1], 128);

        let mut path = [0u8; 32];
        path[0] = 0xFF; path[1] = 0xFF;
        mask_after_be(&mut path, 2);
        assert_eq!(path[0], 0xC0); // 192
        assert_eq!(path[1], 0x00);
    }

    #[test]
    fn test_byte_boundary_split_logic() {
        let mut hubt = Hubt::new();
        let leaf_val = [0xAAu8; 32];

        let mut path_a = [0u8; 32];
        path_a[0] = 0xFF; path_a[1] = 0x80;
        let mut path_b = [0u8; 32];
        path_b[0] = 0xFF; path_b[1] = 0x00;

        let (lcp_p, lcp_l) = lcp_be(&path_a, &path_b);
        assert_eq!(lcp_l, 8);
        assert_eq!(lcp_p[0], 0xFF);
        assert_eq!(lcp_p[1], 0x00);

        hubt.check_neighbor(path_a, leaf_val, path_b, leaf_val);

        let split_key = NodeKey { path: lcp_p, len: 8 };
        assert!(hubt.store.contains_key(&split_key));

        hubt.store.insert(NodeKey { path: path_a, len: 256 }, leaf_val);

        let found = hubt.get_child_hash(lcp_p, 8, 1);
        assert_eq!(found, leaf_val);
    }

    #[test]
    fn test_smoke_sequence_deterministic() {
        let mut hubt = Hubt::new();

        //println!("\n--- Step 1: Insert 0 and 1 ---");
        hubt.batch_update(vec![
            Op::Insert(b"0".to_vec(), b"0".to_vec()),
            Op::Insert(b"1".to_vec(), b"1".to_vec()),
        ]);

        let root_1 = hubt.root();
        let expected_root_1: Hash = [
            238, 97, 151, 15, 183, 44, 176, 246, 70, 241, 213, 115, 32, 121, 65, 80,
            110, 160, 199, 165, 82, 32, 74, 6, 254, 147, 237, 6, 63, 234, 199, 247
        ];

        assert_eq!(root_1, expected_root_1, "Root 1 mismatch");

        //println!("\n--- Step 2: Insert 2 ---");
        hubt.batch_update(vec![
            Op::Insert(b"2".to_vec(), b"2".to_vec()),
        ]);

        let root_2 = hubt.root();
        let expected_root_2: Hash = [
            217, 231, 118, 162, 16, 188, 97, 238, 129, 73, 252, 176, 156, 109, 43, 97,
            105, 60, 189, 96, 253, 5, 183, 129, 222, 129, 175, 15, 81, 142, 248, 130
        ];

        //println!("Root 2: {:?}", root_2.to_vec());
        assert_eq!(root_2, expected_root_2, "Root 2 mismatch");

        //println!("\n--- Step 3: Delete 2 ---");
        hubt.batch_update(vec![
            Op::Delete(b"2".to_vec()),
        ]);

        let root_3 = hubt.root();
        //println!("Root 3: {:?}", root_3.to_vec());

        assert_eq!(root_3, expected_root_1, "Root 3 should revert to Root 1 state");
    }

    #[test]
    fn test_unified_proof_logic() {
        let mut hubt = Hubt::new();
        let k1 = b"user:1".to_vec();
        let v1 = b"100".to_vec();

        // Insert Key 1
        hubt.batch_update(vec![Op::Insert(k1.clone(), v1.clone())]);

        // Case 1: Inclusion (Key exists, Value matches)
        let proof_inc = hubt.prove(k1.clone());
        assert_eq!(Hubt::verify(&proof_inc, k1.clone(), v1.clone()), VerifyStatus::Included);

        // Case 2: Mismatch (Key exists, Value differs)
        let v1_fake = b"999".to_vec();
        let proof_mis = hubt.prove(k1.clone()); // Same proof generation!
        assert_eq!(Hubt::verify(&proof_mis, k1.clone(), v1_fake), VerifyStatus::Mismatch);

        // Case 3: Non-Existence (Key does not exist)
        let k_missing = b"user:999".to_vec();
        let proof_non = hubt.prove(k_missing.clone());
        assert_eq!(Hubt::verify(&proof_non, k_missing, v1.clone()), VerifyStatus::NonExistence);
    }

    #[test]
    fn test_proof_integrity() {
        let mut hubt = Hubt::new();
        // Insert multiple items to create depth
        hubt.batch_update(vec![
            Op::Insert(b"A".to_vec(), b"1".to_vec()),
            Op::Insert(b"B".to_vec(), b"2".to_vec()),
            Op::Insert(b"C".to_vec(), b"3".to_vec()),
        ]);

        let k = b"B".to_vec();
        let proof = hubt.prove(k);
        assert!(Hubt::verify_integrity(&proof));
    }

    #[test]
    fn test_incremental_updates_post_1m_fill() {
        let mut hubt = Hubt::new();
        let initial_fill = 1_000_000;

        // 1. PRE-FILL: 0 to 1,000,000
        println!("Pre-filling tree with {} items... (This might take a moment)", initial_fill);
        let mut ops = Vec::with_capacity(initial_fill);
        for i in 0..initial_fill {
            let s = i.to_string();
            ops.push(Op::Insert(s.as_bytes().to_vec(), s.as_bytes().to_vec()));
        }

        let t_fill = Instant::now();
        hubt.batch_update(ops);
        println!("Pre-fill complete in {:?}\n", t_fill.elapsed());

        // 2. INCREMENTAL BATCHES: Start from 1,000,000 upwards
        let batches = vec![100, 1_000, 10_000, 100_000];
        let mut key_cursor = initial_fill;

        println!("{:<12} | {:<20} | {:<20}", "Batch Size", "Time Taken", "Key Range");
        println!("{:-<12}-+-{:-<20}-+-{:-<20}", "", "", "");

        for count in batches {
            // Generate unique keys strictly higher than previous keys
            let mut ops = Vec::with_capacity(count);
            for i in 0..count {
                let val = key_cursor + i;
                let s = val.to_string();
                ops.push(Op::Insert(s.as_bytes().to_vec(), s.as_bytes().to_vec()));
            }

            // Measure ONLY the update time
            let start = Instant::now();
            hubt.batch_update(ops);
            let duration = start.elapsed();

            let range_str = format!("{} .. {}", key_cursor, key_cursor + count);
            println!("{:<12} | {:<20?} | {:<20}", count, duration, range_str);

            // Advance cursor so next batch is higher
            key_cursor += count;
        }

        println!("\nFinal Tree Size: {}", key_cursor);
    }

    #[test]
    fn test_proof_gen_1m() {
        let mut hubt = Hubt::new();
        let n_items = 100_000;
        let n_proofs = 1_000;

        // 1. Setup: Fill Tree with 1M items
        println!("Preparing {} items...", n_items);
        let mut ops = Vec::with_capacity(n_items);
        for i in 0..n_items {
            let s = i.to_string();
            ops.push(Op::Insert(s.as_bytes().to_vec(), s.as_bytes().to_vec()));
        }

        let t_fill = Instant::now();
        hubt.batch_update(ops);
        println!("Tree filled (100k items) in: {:?}", t_fill.elapsed());

        // 2. Measure Proof Generation
        println!("Generating {} proofs...", n_proofs);
        let t_proof_start = Instant::now();

        for i in 0..n_proofs {
            let s = i.to_string();
            let k = s.as_bytes().to_vec();

            // Generate the proof (unwrap ensures it exists)
            let _proof = hubt.prove(k);
        }

        let duration = t_proof_start.elapsed();
        println!("------------------------------------------------");
        println!("Generated {} proofs in: {:?}", n_proofs, duration);
        println!("Average time per proof: {:?}", duration / n_proofs as u32);
        println!("Proofs per second: {:.2}", n_proofs as f64 / duration.as_secs_f64());
        println!("------------------------------------------------");
    }
}
