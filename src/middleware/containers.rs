/// This file implements the types defined at
/// https://0xparc.github.io/pod2/values.html#dictionary-array-set .
use anyhow::Result;
use plonky2::hash::poseidon::PoseidonHash;
use plonky2::plonk::config::Hasher;
use std::collections::HashMap;

use super::{Hash, Value, EMPTY};
use crate::primitives::merkletree::{MerkleProof, MerkleTree, MerkleTreeTrait};

/// Container is a wrapper of a MerkleTree, used to achieve Dictionary, Set, Array frontend types.
/// It offers all the methods of the trait `MerkleTreeTrait`, with an additional constructor `new`
/// that allows each specific type (ie. Dictionary, Set, Array) to define how each type is
/// constructed (for example a Dictionary is built from HashMap<Hash,Value>, whereas a set is built
/// from Vec<Value>).
pub trait Container: MerkleTreeTrait {
    type Raw: Clone;

    fn new(raw: &Self::Raw) -> Self;
}

/// Dictionary: the user original keys and values are hashed to be used in the leaf.
///    leaf.key=hash(original_key)
///    leaf.value=hash(original_value)
#[derive(Clone, Debug)]
pub struct Dictionary {
    // exposed with pub(crate) so that it can be modified at tests
    pub(crate) mt: MerkleTree,
}

impl Container for Dictionary {
    type Raw = HashMap<Hash, Value>;

    fn new(raw: &Self::Raw) -> Self {
        let kvs: HashMap<Value, Value> = raw.into_iter().map(|(&k, &v)| (Value(k.0), v)).collect();
        Self {
            mt: MerkleTree::build(&kvs),
        }
    }
}

impl MerkleTreeTrait for Dictionary {
    fn root(&self) -> Hash {
        self.mt.root()
    }
    fn get(&self, key: &Value) -> Result<Value> {
        self.mt.get(key)
    }
    fn prove(&self, key: &Value) -> Result<MerkleProof> {
        self.mt.prove(key)
    }
    fn prove_nonexistence(&self, key: &Value) -> Result<MerkleProof> {
        self.mt.prove_nonexistence(key)
    }
    fn verify(root: Hash, proof: &MerkleProof, key: &Value, value: &Value) -> Result<()> {
        MerkleTree::verify(root, proof, key, value)
    }
    fn verify_nonexistence(root: Hash, proof: &MerkleProof, key: &Value) -> Result<()> {
        MerkleTree::verify_nonexistence(root, proof, key)
    }
    fn iter(&self) -> std::collections::hash_map::Iter<Value, Value> {
        self.mt.iter()
    }
}
impl<'a> IntoIterator for &'a Dictionary {
    type Item = (&'a Value, &'a Value);
    type IntoIter = std::collections::hash_map::Iter<'a, Value, Value>;

    fn into_iter(self) -> Self::IntoIter {
        self.mt.iter()
    }
}

impl PartialEq for Dictionary {
    fn eq(&self, other: &Self) -> bool {
        self.mt.root() == other.mt.root() && self.mt.root() == other.mt.root()
    }
}
impl Eq for Dictionary {}

/// Set: the value field of the leaf is unused, and the key contains the hash of the element.
///    leaf.key=hash(original_value)
///    leaf.value=0
#[derive(Clone, Debug)]
pub struct Set {
    mt: MerkleTree,
}

impl Container for Set {
    type Raw = Vec<Value>;

    fn new(raw: &Self::Raw) -> Self {
        let kvs: HashMap<Value, Value> = raw
            .into_iter()
            .map(|e| {
                let h = PoseidonHash::hash_no_pad(&e.0).elements;
                (Value(h), EMPTY)
            })
            .collect();
        Self {
            mt: MerkleTree::build(&kvs),
        }
    }
}

impl MerkleTreeTrait for Set {
    fn root(&self) -> Hash {
        self.mt.root()
    }
    fn get(&self, key: &Value) -> Result<Value> {
        self.mt.get(key)
    }
    fn prove(&self, key: &Value) -> Result<MerkleProof> {
        self.mt.prove(key)
    }
    fn prove_nonexistence(&self, key: &Value) -> Result<MerkleProof> {
        self.mt.prove_nonexistence(key)
    }
    fn verify(root: Hash, proof: &MerkleProof, key: &Value, value: &Value) -> Result<()> {
        MerkleTree::verify(root, proof, key, value)
    }
    fn verify_nonexistence(root: Hash, proof: &MerkleProof, key: &Value) -> Result<()> {
        MerkleTree::verify_nonexistence(root, proof, key)
    }
    fn iter(&self) -> std::collections::hash_map::Iter<Value, Value> {
        self.mt.iter()
    }
}

impl PartialEq for Set {
    fn eq(&self, other: &Self) -> bool {
        self.mt.root() == other.mt.root() && self.mt.root() == other.mt.root()
    }
}
impl Eq for Set {}

/// Array: the elements are placed at the value field of each leaf, and the key field is just the
/// array index (integer).
///    leaf.key=i
///    leaf.value=original_value
#[derive(Clone, Debug)]
pub struct Array {
    mt: MerkleTree,
}

impl Container for Array {
    type Raw = Vec<Value>;

    fn new(raw: &Self::Raw) -> Self {
        let kvs: HashMap<Value, Value> = raw
            .into_iter()
            .enumerate()
            .map(|(i, &e)| (Value::from(i as i64), e))
            .collect();

        Self {
            mt: MerkleTree::build(&kvs),
        }
    }
}

impl MerkleTreeTrait for Array {
    fn root(&self) -> Hash {
        self.mt.root()
    }
    fn get(&self, key: &Value) -> Result<Value> {
        self.mt.get(key)
    }
    fn prove(&self, key: &Value) -> Result<MerkleProof> {
        self.mt.prove(key)
    }
    fn prove_nonexistence(&self, key: &Value) -> Result<MerkleProof> {
        self.mt.prove_nonexistence(key)
    }
    fn verify(root: Hash, proof: &MerkleProof, key: &Value, value: &Value) -> Result<()> {
        MerkleTree::verify(root, proof, key, value)
    }
    fn verify_nonexistence(root: Hash, proof: &MerkleProof, key: &Value) -> Result<()> {
        MerkleTree::verify_nonexistence(root, proof, key)
    }
    fn iter(&self) -> std::collections::hash_map::Iter<Value, Value> {
        self.mt.iter()
    }
}

impl PartialEq for Array {
    fn eq(&self, other: &Self) -> bool {
        self.mt.root() == other.mt.root() && self.mt.root() == other.mt.root()
    }
}
impl Eq for Array {}
