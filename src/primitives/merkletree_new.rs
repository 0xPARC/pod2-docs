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
use std::fmt;
use std::iter::IntoIterator;

use crate::middleware::{Hash, Value, C, D, F, NULL};

pub struct MerkleTree {
    max_depth: usize,
    root: Node,
}

impl MerkleTree {
    /// builds a new `MerkleTree` where the leaves contain the given key-values
    pub fn new(max_depth: usize, kvs: &HashMap<Value, Value>) -> Result<Self> {
        let mut root = Node::Intermediate(Intermediate::empty());

        for (k, v) in kvs.iter() {
            let leaf = Leaf::new(max_depth, *k, *v)?;
            root.add_leaf(0, max_depth, leaf)?;
        }

        let _ = root.compute_hash();
        Ok(Self { max_depth, root })
    }
}

impl fmt::Display for MerkleTree {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(
            f,
            "\nPaste in GraphViz (https://dreampuf.github.io/GraphvizOnline/):\n-----"
        );
        writeln!(f, "digraph hierarchy {{");
        writeln!(f, "node [fontname=Monospace,fontsize=10,shape=box]");
        write!(f, "{}", self.root);
        writeln!(f, "\n}}\n-----")
    }
}

#[derive(Clone, Debug)]
pub struct MerkleProof {
    existence: bool,
    siblings: Vec<Hash>,
}

impl fmt::Display for MerkleProof {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (i, s) in self.siblings.iter().enumerate() {
            if i > 0 {
                write!(f, ", ");
            }
            write!(f, "{}", s);
        }
        Ok(())
    }
}

impl MerkleTree {
    /// returns the root of the tree
    fn root(&self) -> Hash {
        self.root.hash()
    }

    /// returns the value at the given key
    pub fn get(&self, key: &Value) -> Result<Value> {
        let path = keypath(self.max_depth, *key)?;
        let (v, _) = self.root.down(0, self.max_depth, path, None)?;
        Ok(v)
    }

    /// returns a boolean indicating whether the key exists in the tree
    pub fn contains(&self, key: &Value) -> bool {
        // WIP once thiserror is added to pod2, this method is just like `.get` but returning
        // true/false if the error matches the key-non-existing error returned by `down`
        // let path = keypath(self.max_depth, *key)?;
        // match self.root.down(0, self.max_depth, path, None) {
        //     Ok((_, _)) => true,
        //     Err("leaf not found")) => false,
        //     Err(_) => false,
        // }
        unimplemented!();
    }

    /// returns a proof of existence, which proves that the given key exists in
    /// the tree. It returns the `value` of the leaf at the given `key`, and
    /// the `MerkleProof`.
    fn prove(&self, key: &Value) -> Result<(Value, MerkleProof)> {
        let path = keypath(self.max_depth, *key)?;
        let (v, siblings) = self.root.down(0, self.max_depth, path, Some(Vec::new()))?;
        Ok((
            v,
            MerkleProof {
                existence: true,
                // `unwrap` is safe since we've called `down` passing a vector
                siblings: siblings.unwrap(),
            },
        ))
    }

    /// returns a proof of non-existence, which proves that the given `key`
    /// does not exist in the tree
    fn prove_nonexistence(&self, key: &Value) -> Result<MerkleProof> {
        // note: non-existence of a key can be in 2 cases:
        // - the expected leaf does not exist
        // - the expected leaf does exist in the tree, but it has a different `key`
        // both cases prove that the given key don't exist in the tree.
        todo!();
    }

    /// verifies an inclusion proof for the given `key` and `value`
    fn verify(
        max_depth: usize,
        root: Hash,
        proof: &MerkleProof,
        key: &Value,
        value: &Value,
    ) -> Result<()> {
        if proof.siblings.len() >= max_depth {
            return Err(anyhow!("max depth reached"));
        }

        let path = keypath(max_depth, *key)?;
        let input: Vec<F> = [key.0, value.0].concat();
        let mut h = Hash(PoseidonHash::hash_no_pad(&input).elements);
        for (i, sibling) in proof.siblings.iter().enumerate().rev() {
            let input: Vec<F> = if path[i] {
                [sibling.0, h.0].concat()
            } else {
                [h.0, sibling.0].concat()
            };
            h = Hash(PoseidonHash::hash_no_pad(&input).elements);
        }

        if h != root {
            return Err(anyhow!("proof of inclusion does not verify"));
        }
        Ok(())
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

#[derive(Clone, Debug)]
enum Node {
    None,
    Leaf(Leaf),
    Intermediate(Intermediate),
}

impl fmt::Display for Node {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Intermediate(n) => {
                writeln!(
                    f,
                    "\"{}\" -> {{ \"{}\" \"{}\" }}",
                    n.hash(),
                    n.left.hash(),
                    n.right.hash()
                );
                write!(f, "{}", n.left);
                write!(f, "{}", n.right)
            }
            Self::Leaf(l) => {
                writeln!(f, "\"{}\" [style=filled]", l.hash());
                writeln!(f, "\"k:{}\\nv:{}\" [style=dashed]", l.key, l.value);
                writeln!(
                    f,
                    "\"{}\" -> {{ \"k:{}\\nv:{}\" }}",
                    l.hash(),
                    l.key,
                    l.value,
                )
            }
            Self::None => Ok(()),
        }
    }
}

impl Node {
    fn is_empty(&self) -> bool {
        match self {
            Self::None => true,
            Self::Leaf(l) => false,
            Self::Intermediate(n) => false,
        }
    }
    fn compute_hash(&mut self) -> Hash {
        match self {
            Self::None => NULL,
            Self::Leaf(l) => l.compute_hash(),
            Self::Intermediate(n) => n.compute_hash(),
        }
    }
    fn hash(&self) -> Hash {
        match self {
            Self::None => NULL,
            Self::Leaf(l) => l.hash(),
            Self::Intermediate(n) => n.hash(),
        }
    }

    /// goes down from the current node till finding a leaf or reaching the max_depth. The
    /// `siblings` parameter is used to store the siblings while going down to the leaf, if the
    /// given parameter is set to `None`, then no siblings are stored. In this way, the same method
    /// `down` can be used by MerkleTree methods `get`, `contains`, `prove` and
    /// `prove_nonexistence`.
    fn down(
        &self,
        lvl: usize,
        max_depth: usize,
        path: Vec<bool>,
        mut siblings: Option<Vec<Hash>>,
    ) -> Result<(Value, Option<Vec<Hash>>)> {
        if lvl >= max_depth {
            return Err(anyhow!("max depth reached"));
        }

        match self {
            Self::Intermediate(n) => {
                if path[lvl] {
                    if let Some(ref mut s) = siblings {
                        s.push(n.left.hash());
                    }
                    return n.right.down(lvl + 1, max_depth, path, siblings);
                } else {
                    if let Some(ref mut s) = siblings {
                        s.push(n.right.hash());
                    }
                    return n.left.down(lvl + 1, max_depth, path, siblings);
                }
            }
            Self::Leaf(l) => {
                return Ok((l.value, siblings));
            }
            Self::None => {
                return Err(anyhow!("leaf not found"));
            }
        }
        Err(anyhow!("leaf not found"))
    }

    // adds the leaf at the tree from the current node (self), without computing any hash
    fn add_leaf(&mut self, lvl: usize, max_depth: usize, leaf: Leaf) -> Result<()> {
        if lvl >= max_depth {
            return Err(anyhow!("max depth reached"));
        }

        match self {
            Self::Intermediate(n) => {
                if leaf.path[lvl] {
                    if n.right.is_empty() {
                        // empty sub-node, add the leaf here
                        n.right = Box::new(Node::Leaf(leaf));
                        return Ok(());
                    }
                    n.right.add_leaf(lvl + 1, max_depth, leaf)?;
                } else {
                    if n.left.is_empty() {
                        // empty sub-node, add the leaf here
                        n.left = Box::new(Node::Leaf(leaf));
                        return Ok(());
                    }
                    n.left.add_leaf(lvl + 1, max_depth, leaf)?;
                }
            }
            Self::Leaf(l) => {
                // in this case, it means that we found a leaf in the new-leaf path, thus we need
                // to push both leaves (old-leaf and new-leaf) down the path till their paths
                // diverge.

                // first check that keys of both leaves are different
                // (l=old-leaf, leaf=new-leaf)
                if l.key == leaf.key {
                    // TODO decide if we want to return an error when trying to add a leaf that
                    // already exists, or if we just ignore it. For the moment we return the error
                    // if the key already exists in the leaf.
                    return Err(anyhow!("key already exists"));
                }
                let old_leaf = l.clone();
                // set self as an intermediate node
                *self = Node::Intermediate(Intermediate::empty());
                return self.down_till_divergence(lvl, max_depth, old_leaf, leaf);
            }
            Self::None => {
                return Err(anyhow!("reached empty node, should not have entered"));
            }
        }
        Ok(())
    }

    /// goes down through a 'virtual' path till finding a divergence. This method is used for when
    /// adding a new leaf another already existing leaf is found, so that both leaves (new and old)
    /// are pushed down the path till their keys diverge.
    fn down_till_divergence(
        &mut self,
        lvl: usize,
        max_depth: usize,
        old_leaf: Leaf,
        new_leaf: Leaf,
    ) -> Result<()> {
        if lvl >= max_depth {
            return Err(anyhow!("max depth reached"));
        }

        if let Node::Intermediate(ref mut n) = self {
            // let current_node: Intermediate = *self;
            if old_leaf.path[lvl] != new_leaf.path[lvl] {
                // reached divergence in next level, set the leaves as children at the current node
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
                return n
                    .right
                    .down_till_divergence(lvl + 1, max_depth, old_leaf, new_leaf);
            } else {
                n.left = Box::new(Node::Intermediate(Intermediate::empty()));
                return n
                    .left
                    .down_till_divergence(lvl + 1, max_depth, old_leaf, new_leaf);
            }
        }
        Ok(())
    }
}

#[derive(Clone, Debug)]
struct Intermediate {
    hash: Option<Hash>,
    left: Box<Node>,
    right: Box<Node>,
}
impl Intermediate {
    fn empty() -> Self {
        Self {
            hash: None,
            left: Box::new(Node::None),
            right: Box::new(Node::None),
        }
    }
    fn compute_hash(&mut self) -> Hash {
        if self.left.clone().is_empty() && self.right.clone().is_empty() {
            self.hash = Some(NULL);
            return NULL;
        }
        let l_hash = self.left.compute_hash();
        let r_hash = self.right.compute_hash();
        let input: Vec<F> = [l_hash.0, r_hash.0].concat();
        let h = Hash(PoseidonHash::hash_no_pad(&input).elements);
        self.hash = Some(h);
        h
    }
    fn hash(&self) -> Hash {
        self.hash.unwrap()
    }
}

#[derive(Clone, Debug)]
struct Leaf {
    hash: Option<Hash>,
    path: Vec<bool>,
    key: Value,
    value: Value,
}
impl Leaf {
    fn new(max_depth: usize, key: Value, value: Value) -> Result<Self> {
        Ok(Self {
            hash: None,
            path: keypath(max_depth, key)?,
            key,
            value,
        })
    }
    fn compute_hash(&mut self) -> Hash {
        let input: Vec<F> = [self.key.0, self.value.0].concat();
        let h = Hash(PoseidonHash::hash_no_pad(&input).elements);
        self.hash = Some(h);
        h
    }
    fn hash(&self) -> Hash {
        self.hash.unwrap()
    }
}

// NOTE 1: think if maybe the length of the returned vector can be <256 (8*bytes.len()), so that
// we can do fewer iterations. For example, if the tree.max_depth is set to 20, we just need 20
// iterations of the loop, not 256.
// NOTE 2: which approach do we take with keys that are longer than the max-depth? ie, what
// happens when two keys share the same path for more bits than the max_depth?
/// returns the path of the given key
fn keypath(max_depth: usize, k: Value) -> Result<Vec<bool>> {
    let bytes = k.to_bytes();
    if max_depth > 8 * bytes.len() {
        // note that our current keys are of Value type, which are 4 Goldilocks field elements, ie
        // ~256 bits, therefore the max_depth can not be bigger than 256.
        return Err(anyhow!(
            "key to short (key length: {}) for the max_depth: {}",
            8 * bytes.len(),
            max_depth
        ));
    }
    Ok((0..max_depth)
        .map(|n| bytes[n / 8] & (1 << (n % 8)) != 0)
        .collect())
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use crate::middleware::hash_str;

    #[test]
    fn test_merkletree() -> Result<()> {
        let mut kvs = HashMap::new();
        for i in 0..8 {
            if i == 1 {
                continue;
            }
            kvs.insert(Value::from(i), Value::from(1000 + i));
        }
        let key = Value::from(13);
        let value = Value::from(1013);
        kvs.insert(key, value);

        let tree = MerkleTree::new(32, &kvs)?;
        // when printing the tree, it should print the same tree as in
        // https://0xparc.github.io/pod2/merkletree.html#example-2
        println!("{}", tree);

        let (v, proof) = tree.prove(&Value::from(13))?;
        assert_eq!(v, Value::from(1013));
        println!("{}", proof);

        MerkleTree::verify(32, tree.root(), &proof, &key, &value)?;

        Ok(())
    }
}
