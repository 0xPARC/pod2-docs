#![allow(unused)]
#![allow(dead_code, unused_variables)]
// NOTE: starting in this file (merkletree_new.rs), once we have the implementation ready we just
// place it in the merkletree.rs file.
use anyhow::{anyhow, Result};
use itertools::Itertools;
use plonky2::field::types::Field;
use plonky2::hash::{hash_types::HashOut, poseidon::PoseidonHash};
use plonky2::plonk::config::GenericConfig;
use plonky2::plonk::config::Hasher;
use std::collections::HashMap;
use std::iter::IntoIterator;

use crate::middleware::{Hash, Value, C, D, F, NULL};

pub struct MerkleTree {
    max_depth: usize,
    root: Intermediate,
}

#[derive(Clone, Debug)]
enum Node {
    None,
    Leaf(Leaf),
    Intermediate(Intermediate),
}
impl Node {
    fn is_empty(self) -> bool {
        match self {
            Self::None => true,
            Self::Leaf(l) => false,
            Self::Intermediate(n) => false,
        }
    }
    fn hash(self) -> Hash {
        match self {
            Self::None => NULL,
            Self::Leaf(l) => l.hash(),
            Self::Intermediate(n) => n.hash(),
        }
    }
    fn add_leaf(&mut self, lvl: usize, leaf: Leaf) -> Result<()> {
        // TODO check that lvl<=maxlevels

        match self {
            Self::Intermediate(n) => {
                if leaf.path[lvl] {
                    if (*n.right).clone().is_empty() {
                        // empty sub-node, add the leaf here
                        n.right = Box::new(Node::Leaf(leaf));
                        return Ok(());
                    }
                    n.right.add_leaf(lvl + 1, leaf)?;
                } else {
                    if (*n.left).clone().is_empty() {
                        // empty sub-node, add the leaf here
                        n.left = Box::new(Node::Leaf(leaf));
                        return Ok(());
                    }
                    n.left.add_leaf(lvl + 1, leaf)?;
                }
            }
            Self::Leaf(l) => {
                // in this case, it means that we found a leaf in the new-leaf path, thus we need
                // to push both leaves (old-leaf and new-leaf) down the path till their paths
                // diverge.

                // first check that keys of both leafs are different
                // (l: old-leaf, leaf: new-leaf)
                if l.key == leaf.key {
                    // TODO decide if we want to return an error when trying to add a leaf that
                    // allready exists, or if we just ignore it
                    return Err(anyhow!("key already exists"));
                }
                let old_leaf = l.clone();
                // set self as an intermediate node
                *self = Node::Intermediate(Intermediate::empty());
                return self.down_till_divergence(lvl, old_leaf, leaf);
            }
            Self::None => {
                return Err(anyhow!("reached empty node, should not have entered"));
            }
        }
        Ok(())
    }

    fn down_till_divergence(&mut self, lvl: usize, old_leaf: Leaf, new_leaf: Leaf) -> Result<()> {
        // TODO check that lvl<=maxlevels

        if let Node::Intermediate(ref mut n) = self {
            // let current_node: Intermediate = *self;
            if old_leaf.path[lvl] != new_leaf.path[lvl] {
                // reached divergence in next level, set the leafs as childs at the current node
                if new_leaf.path[lvl] {
                    n.left = Box::new(Node::Leaf(old_leaf));
                    n.right = Box::new(Node::Leaf(new_leaf));
                } else {
                    n.left = Box::new(Node::Leaf(new_leaf));
                    n.right = Box::new(Node::Leaf(old_leaf));
                }
                return Ok(());
            }

            // no divergence yet, continue going down
            if new_leaf.path[lvl] {
                n.right = Box::new(Node::Intermediate(Intermediate::empty()));
                return n.right.down_till_divergence(lvl + 1, old_leaf, new_leaf);
            } else {
                n.left = Box::new(Node::Intermediate(Intermediate::empty()));
                return n.left.down_till_divergence(lvl + 1, old_leaf, new_leaf);
            }
        }
        Ok(())
    }
}

#[derive(Clone, Debug)]
struct Intermediate {
    left: Box<Node>,
    right: Box<Node>,
}
impl Intermediate {
    fn empty() -> Self {
        Self {
            left: Box::new(Node::None),
            right: Box::new(Node::None),
        }
    }

    // TODO move to a Node/Hashable trait?
    fn hash(self) -> Hash {
        let l_hash = self.left.hash();
        let r_hash = self.right.hash();
        let input: Vec<F> = [l_hash.0, r_hash.0].concat();
        Hash(PoseidonHash::hash_no_pad(&input).elements)
    }
}

#[derive(Clone, Debug)]
struct Leaf {
    path: Vec<bool>,
    key: Value,
    value: Value,
}
impl Leaf {
    fn new(key: Value, value: Value) -> Self {
        Self {
            path: keypath(key),
            key,
            value,
        }
    }
}
impl Leaf {
    // TODO move to a Node/Hashable trait?
    fn hash(self) -> Hash {
        let input: Vec<F> = [self.key.0, self.value.0].concat();
        Hash(PoseidonHash::hash_no_pad(&input).elements)
    }
}

// TODO 1: think if maybe the length of the returned vector can be <256 (8*bytes.len()), so that
// we can do fewer iterations. For example, if the tree.max_depth is set to 20, we just need 20
// iterations of the loop, not 256.
// TODO 2: which approach do we take with keys that are longer than the max-depth? ie, what
// happens when two keys share the same path for more bits than the max_depth?
fn keypath(k: Value) -> Vec<bool> {
    let bytes = k.to_bytes();
    (0..8 * bytes.len())
        .map(|n| bytes[n / 8] & (1 << (n % 8)) != 0)
        .collect()
}

pub struct MerkleProof {
    existence: bool,
}

impl MerkleTree {
    /// returns the root of the tree
    fn root(&self) -> Hash {
        todo!();
    }

    /// returns the value at the given key
    pub fn get(&self, key: &Value) -> Result<Value> {
        todo!();
    }

    /// returns a boolean indicating whether the key exists in the tree
    pub fn contains(&self, key: &Value) -> bool {
        todo!();
    }

    /// returns a proof of existence, which proves that the given key exists in
    /// the tree. It returns the `value` of the leaf at the given `key`, and
    /// the `MerkleProof`.
    fn prove(&self, key: &Value) -> Result<MerkleProof> {
        todo!();
    }

    /// returns a proof of non-existence, which proves that the given `key`
    /// does not exist in the tree
    fn prove_nonexistence(&self, key: &Value) -> Result<MerkleProof> {
        todo!();
    }

    /// verifies an inclusion proof for the given `key` and `value`
    fn verify(root: Hash, proof: &MerkleProof, key: &Value, value: &Value) -> Result<()> {
        todo!();
    }

    /// verifies a non-inclusion proof for the given `key`, that is, the given
    /// `key` does not exist in the tree
    fn verify_nonexistence(root: Hash, proof: &MerkleProof, key: &Value) -> Result<()> {
        todo!();
    }

    /// returns an iterator over the leaves of the tree
    fn iter(&self) -> std::collections::hash_map::Iter<Value, Value> {
        todo!();
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use crate::middleware::hash_str;

    #[test]
    fn test_keypath() -> Result<()> {
        let key = Value(hash_str("key".into()).0);
        // dbg!(keypath(key));

        Ok(())
    }
}
