use crate::hash::{hashv, Hash};

use crate::Node;
// We need to discern between leaf and intermediate nodes to prevent trivial second
// pre-image attacks.
// https://flawed.net.nz/2018/02/21/attacking-merkle-trees-with-a-second-preimage-attack
pub const LEAF_PREFIX: &[u8] = &[0];
pub const INTERMEDIATE_PREFIX: &[u8] = &[1];

#[macro_export]
macro_rules! hash_leaf {
    {$d:ident} => {
        hashv(&[LEAF_PREFIX, $d])
    }
}

#[macro_export]
macro_rules! hash_intermediate {
    {$l:expr, $r:expr} => {
        hashv(&[INTERMEDIATE_PREFIX, $l.as_ref(), $r.as_ref()])
    }
}

#[derive(Debug)]
pub struct MerkleTree {
    pub leaf_count: usize,
    pub nodes: Vec<Node>,
    pub n: Vec<Hash>,
}
//
// #[derive(Debug, PartialEq, Eq)]
// pub struct ProofEntry<'a>(&'a Hash, Option<&'a Hash>, Option<&'a Hash>);
//
// impl<'a> ProofEntry<'a> {
//     pub fn new(
//         target: &'a Hash,
//         left_sibling: Option<&'a Hash>,
//         right_sibling: Option<&'a Hash>,
//     ) -> Self {
//         assert!(left_sibling.is_none() ^ right_sibling.is_none());
//         Self(target, left_sibling, right_sibling)
//     }
// }
//
// #[derive(Debug, Default, PartialEq, Eq)]
// pub struct Proof<'a>(Vec<ProofEntry<'a>>);
//
// impl<'a> Proof<'a> {
//     pub fn push(&mut self, entry: ProofEntry<'a>) {
//         self.0.push(entry)
//     }
//
//     pub fn verify(&self, candidate: Hash) -> bool {
//         let result = self.0.iter().try_fold(candidate, |candidate, pe| {
//             let lsib = pe.1.unwrap_or(&candidate);
//             let rsib = pe.2.unwrap_or(&candidate);
//             let hash = hash_intermediate!(lsib, rsib);
//
//             if hash == *pe.0 {
//                 Some(hash)
//             } else {
//                 None
//             }
//         });
//         result.is_some()
//     }
// }

impl MerkleTree {
    pub fn get_nodes(&self) -> Vec<Node> {
        self.nodes.clone()
    }
    #[inline]
    pub fn next_level_len(level_len: usize) -> usize {
        if level_len == 1 {
            0
        } else {
            (level_len + 1) / 2
        }
    }

    pub fn calculate_vec_capacity(leaf_count: usize) -> usize {
        // the most nodes consuming case is when n-1 is full balanced binary tree
        // then n will cause the previous tree add a left only path to the root
        // this cause the total nodes number increased by tree height, we use this
        // condition as the max nodes consuming case.
        // n is current leaf nodes number
        // assuming n-1 is a full balanced binary tree, n-1 tree nodes number will be
        // 2(n-1) - 1, n tree height is closed to log2(n) + 1
        // so the max nodes number is 2(n-1) - 1 + log2(n) + 1, finally we can use
        // 2n + log2(n+1) as a safe capacity value.
        // test results:
        // 8192 leaf nodes(full balanced):
        // computed cap is 16398, actually using is 16383
        // 8193 leaf nodes:(full balanced plus 1 leaf):
        // computed cap is 16400, actually using is 16398
        // about performance: current used fast_math log2 code is constant algo time
        if leaf_count > 0 {
            fast_math::log2_raw(leaf_count as f32) as usize + 2 * leaf_count + 1
        } else {
            0
        }
    }

    pub fn empty_tree(leaf_count: usize, batch_count: usize) -> Self {
        let cap = MerkleTree::calculate_vec_capacity(leaf_count);
        let mut mt = MerkleTree {
            leaf_count,
            n: Vec::with_capacity(cap),
            nodes: Vec::with_capacity(cap),
        };

        // for item in items {
        //     let item = item.as_ref();
        //     let hash = hash_leaf!(item);
        //     mt.nodes.push(hash);
        // }

        mt.nodes
            .append(&mut vec![Node::digest(0, batch_count); leaf_count]);

        let mut level_len = MerkleTree::next_level_len(leaf_count);
        let mut level_start = leaf_count;
        // let mut prev_level_len = leaf_count;
        // let mut prev_level_start = 0;
        while level_len > 0 {
            for i in 0..level_len {
                let prev_level_idx = 2 * i;
                // let lsib = &mt.nodes[prev_level_start + prev_level_idx];
                // let rsib = if prev_level_idx + 1 < prev_level_len {
                //     &mt.nodes[prev_level_start + prev_level_idx + 1]
                // } else {
                //     // Duplicate last entry if the level length is odd
                //     &mt.nodes[prev_level_start + prev_level_idx]
                // };
                //
                // let hash = hash_intermediate!(lsib, rsib);
                mt.nodes.push(Node::default());
            }
            // prev_level_start = level_start;
            // prev_level_len = level_len;
            level_start += level_len;
            level_len = MerkleTree::next_level_len(level_len);
        }

        mt
    }
    pub fn new<T: AsRef<[u8]>>(items: &[T]) -> Self {
        let cap = MerkleTree::calculate_vec_capacity(items.len());
        let mut mt = MerkleTree {
            leaf_count: items.len(),
            nodes: Vec::with_capacity(cap),
            n: Vec::with_capacity(cap),
        };

        for item in items {
            let item = item.as_ref();
            let hash = hash_leaf!(item);
            // let n = Node::default();
            // n.data
            mt.n.push(hash);
        }

        let mut level_len = MerkleTree::next_level_len(items.len());
        let mut level_start = items.len();
        let mut prev_level_len = items.len();
        let mut prev_level_start = 0;
        while level_len > 0 {
            for i in 0..level_len {
                let prev_level_idx = 2 * i;
                let lsib = &mt.n[prev_level_start + prev_level_idx];
                let rsib = if prev_level_idx + 1 < prev_level_len {
                    &mt.n[prev_level_start + prev_level_idx + 1]
                } else {
                    // Duplicate last entry if the level length is odd
                    &mt.n[prev_level_start + prev_level_idx]
                };

                let hash = hash_intermediate!(lsib, rsib);
                mt.n.push(hash);
            }
            prev_level_start = level_start;
            prev_level_len = level_len;
            level_start += level_len;
            level_len = MerkleTree::next_level_len(level_len);
        }

        mt
    }

    pub fn get_root(&self) -> Option<&Hash> {
        self.n.iter().last()
    }

    // pub fn find_path(&self, index: usize) -> Option<Proof> {
    //     if index >= self.leaf_count {
    //         return None;
    //     }
    //
    //     let mut level_len = self.leaf_count;
    //     let mut level_start = 0;
    //     let mut path = Proof::default();
    //     let mut node_index = index;
    //     let mut lsib = None;
    //     let mut rsib = None;
    //     while level_len > 0 {
    //         let level = &self.nodes[level_start..(level_start + level_len)];
    //
    //         let target = &level[node_index];
    //         if lsib.is_some() || rsib.is_some() {
    //             path.push(ProofEntry::new(target, lsib, rsib));
    //         }
    //         if node_index % 2 == 0 {
    //             lsib = None;
    //             rsib = if node_index + 1 < level.len() {
    //                 Some(&level[node_index + 1])
    //             } else {
    //                 Some(&level[node_index])
    //             };
    //         } else {
    //             lsib = Some(&level[node_index - 1]);
    //             rsib = None;
    //         }
    //         node_index /= 2;
    //
    //         level_start += level_len;
    //         level_len = MerkleTree::next_level_len(level_len);
    //     }
    //     Some(path)
    // }
}
