pub mod hash;
mod merkle;

use hash::{hashv, Hash};
use merkle::*;
use rayon::prelude::*;

pub const DIGEST: [u8; 32] = [0u8; 32];

#[derive(Default, Clone, Debug)]
pub struct Node {
    pub batch: u8, // batch.len() + 2 for compound node
    pub order: u8, // 0=left, 1=right
    pub data: Hash,
    pub node_type: NodeType,
    pub index: usize,
}

#[derive(PartialEq, Default, Clone, Debug)]
pub enum NodeType {
    Checkpoint,
    Compound,
    Leaf,
    Inter,
    #[default]
    Digest,
}
impl Node {
    pub fn is_digest(&self) -> bool {
        self.node_type == NodeType::Digest
    }
    pub fn digest(i: usize, batch_id: usize) -> Self {
        let mut n = Self::default();
        n.batch = (batch_id) as u8;
        n.index = i;
        n
    }
    pub fn is_compound(&self) -> bool {
        self.node_type == NodeType::Compound
    }
}
pub struct AsyncMerkleTree {
    pub nodes: Vec<Node>,
    pub checkpoints: Vec<Node>,
    // pub root: Hash,
    pub batch_count: usize,
    // pub batch_lens: Vec<u8>,
    pub leaf_count: usize,
}

impl AsyncMerkleTree {
    pub fn build_digest_tree(batches: Vec<(u32, &[&[u8]], usize)>, leaf_count: usize) -> Vec<Node> {
        let mut mtnodes: Vec<Node> =
            Vec::with_capacity(MerkleTree::calculate_vec_capacity(batches.len()));
        let mut indexes = vec![];
        for b in batches {
            indexes.append(&mut vec![b.0 + 1; b.1.len()]);
        }
        mtnodes.append(
            &mut indexes
                .into_par_iter()
                .enumerate()
                .map(|(i, leaf_item)| Node::digest(i, leaf_item as usize))
                .collect(),
        );
        mtnodes
    }
    pub fn init(leaf_count: usize, batch_count: usize, d_id: u8) -> Self {
        //let mt = MerkleTree::empty_tree(leaf_count, batch_count);
        let mtnodes: Vec<Node> = Vec::with_capacity(MerkleTree::calculate_vec_capacity(leaf_count));
        // mtnodes.append(
        //     &mut (0..leaf_count)
        //         .into_par_iter()
        //         .map(|i| Node::digest(i, d_id as usize))
        //         .collect(),
        // );
        Self {
            nodes: mtnodes,
            // nodes: mt.get_nodes(),
            checkpoints: vec![],
            batch_count,
            // batch_lens: vec![],
            leaf_count,
        }
    }

    pub fn append_batch(&mut self, items: &[&[u8]], batch_id: u8, start: usize) -> Vec<Node> {
        // let cap = MerkleTree::calculate_vec_capacity(self.leaf_count);
        let mut checkpoints = vec![];
        // let mut mt = MerkleTree {
        //     leaf_count: self.leaf_count,
        //     nodes: Vec::with_capacity(cap),
        // };

        for (i, item) in items.into_iter().enumerate() {
            let item = item.as_ref();
            let hash = hash_leaf!(item);
            let node = Node {
                batch: batch_id,
                order: 255,
                data: hash,
                index: start + i,
                node_type: NodeType::Leaf,
            };
            // println!("start: {}, i: {}, len: {}", start, i, items.len());
            self.nodes[start + i] = node;
        }
        // println!("leaf nodes: {:?}", self.nodes);

        let mut level_len = MerkleTree::next_level_len(self.leaf_count);
        let mut level_start = self.leaf_count;
        let mut prev_level_len = self.leaf_count;
        let mut prev_level_start = 0;
        let mut counter = self.leaf_count;
        while level_len > 0 {
            for i in 0..level_len {
                let prev_level_idx = 2 * i;
                let mut lsib = self.nodes[prev_level_start + prev_level_idx].clone();
                lsib.order = 0;
                // lsib.index = prev_level_start + prev_level_idx;
                let mut rsib = if prev_level_idx + 1 < prev_level_len {
                    // println!(
                    //     "prev_level_start: {:?}, prev_level_idx: {:?}",
                    //     prev_level_start, prev_level_idx
                    // );
                    let mut rnode = self.nodes[prev_level_start + prev_level_idx + 1].clone();
                    // rnode.index = prev_level_start + prev_level_idx + 1;
                    rnode.order = 1;
                    rnode
                } else {
                    // Duplicate last entry if the level length is odd
                    let mut rnode = self.nodes[prev_level_start + prev_level_idx].clone();
                    // rnode.index = prev_level_start + prev_level_idx;
                    rnode.order = 1;
                    rnode
                };

                // println!("lsib: {:?}, rsib:{:?}, level: {:?}", lsib, rsib, level_len);
                let new_node_hash = hash_intermediate!(lsib.data, rsib.data);
                if lsib.batch != rsib.batch {
                    // println!("mismatch batch");
                    if !lsib.is_digest() && !lsib.is_compound() {
                        // println!("lsib not digest");
                        lsib.node_type = NodeType::Checkpoint;
                        checkpoints.push(lsib.clone());
                    }

                    if !rsib.is_digest() && !rsib.is_compound() {
                        // println!("rsib not digest");
                        rsib.node_type = NodeType::Checkpoint;
                        checkpoints.push(rsib);
                    }

                    let c_node = Node {
                        batch: (self.batch_count + 2) as u8,
                        order: 0, //temp 0
                        data: new_node_hash,
                        index: counter,
                        node_type: NodeType::Compound,
                    };
                    counter += 1;
                    self.nodes.push(c_node);
                } else {
                    let new_node = Node {
                        batch: lsib.batch,
                        order: 0, //temp 0
                        data: new_node_hash,
                        index: counter,
                        node_type: if lsib.is_digest() && rsib.is_digest() {
                            NodeType::Digest
                        } else if lsib.is_compound() && rsib.is_compound() {
                            NodeType::Compound
                        } else {
                            NodeType::Inter
                        },
                    };
                    counter += 1;
                    self.nodes.push(new_node);
                }
            }
            prev_level_start = level_start;
            prev_level_len = level_len;
            level_start += level_len;
            level_len = MerkleTree::next_level_len(level_len);
        }
        checkpoints
        // mt
    }

    pub fn commit(checkpoints: Vec<Vec<Node>>) -> Hash {
        let mut checkpoints: Vec<Node> = checkpoints.into_iter().flatten().collect();
        checkpoints.par_sort_unstable_by(|a, b| a.index.cmp(&b.index));
        let mut comp_node = checkpoints[0].data.clone();
        for c in checkpoints[1..].into_iter() {
            if c.order == 1 {
                comp_node = hash_intermediate!(comp_node, c.data);
                continue;
            }
            comp_node = hash_intermediate!(c.data, comp_node);
        }
        comp_node
    }
}
#[cfg(test)]
mod tests {
    use std::time::Instant;

    use super::*;
    // use fast_merkle_tree::*;

    const BLUE: &[&[u8]] = &[b"my".as_slice(); 80_000];
    const RED: &[&[u8]] = &[b"bad".as_slice(); 20_000];

    use lazy_static::lazy_static;
    lazy_static! {
        pub static ref PAR_THREAD_POOL: rayon::ThreadPool = rayon::ThreadPoolBuilder::new()
            .num_threads(2)
            .thread_name(|i| format!("solBstoreProc{i:02}"))
            .build()
            .unwrap();
    }

    #[tokio::test]
    async fn test_amt() {
        let batches = vec![(0u32, BLUE, 0), (1, RED, BLUE.len())];

        let i1 = Instant::now();
        let digest_tree = AsyncMerkleTree::build_digest_tree(batches.clone(), 100_000);
        let a1 = i1.elapsed();

        println!("empty tree took: {:?}", a1.as_millis());

        let response: Vec<Vec<Node>> = PAR_THREAD_POOL.install(|| {
            batches
                .clone()
                .into_par_iter()
                .map(|(i, leaf_batch, start)| {
                    let mut amt = AsyncMerkleTree::init(100_000, 2, (i + 1) as u8);
                    amt.nodes = digest_tree.clone();

                    let i1 = Instant::now();
                    let checkpoints = amt.append_batch(leaf_batch, (i + 1) as u8, start);
                    let t1 = i1.elapsed();

                    println!("thread {} took: {}", i, t1.as_millis());
                    checkpoints
                })
                .collect()
        });

        let i2 = Instant::now();
        let root = AsyncMerkleTree::commit(response);
        let t2 = i2.elapsed();
        println!("async root: {:?}, time: {:?}", root, t2.as_millis());

        let testbatches: Vec<&[u8]> = batches
            .clone()
            .into_iter()
            .map(|b| b.1.to_vec())
            .flatten()
            .collect();

        let i3 = Instant::now();
        let mt = MerkleTree::new(testbatches.as_slice());
        let t3 = i3.elapsed();

        println!(
            "sync root: {:?}, time: {}",
            mt.get_root().unwrap(),
            t3.as_millis()
        );

        assert!(mt.get_root().unwrap() == &root);
        // tree.insert(&[0u8; 32]);
        // tree.insert(&[0u8; 32]);
        // tree.insert(&[0u8; 32]);
        // tree.insert(&[0u8; 32]);
        // println!(
        //     "root: {:?}",
        //     response // bs58::encode(hash::hashv(&[&[1], &[0u8; 32]]).0).into_string(),
        // );
        // for (n, it) in response.iter() {
        //     println!("{} [", n);
        //     for i in it {
        //         println!("node: {:?}", i);
        //     }
        //     println!("]");
        // }
    }
}
