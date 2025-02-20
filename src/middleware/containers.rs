/// This file implements the types defined at
/// https://0xparc.github.io/pod2/values.html#dictionary-array-set .
use anyhow::Result;
use plonky2::hash::poseidon::PoseidonHash;
use plonky2::plonk::config::Hasher;
use std::collections::HashMap;

use super::{Hash, Value, EMPTY};
use crate::constants::MAX_DEPTH;
use crate::primitives::merkletree::{MerkleProof, MerkleTree};

/// Dictionary: the user original keys and values are hashed to be used in the leaf.
///    leaf.key=hash(original_key)
///    leaf.value=hash(original_value)
#[derive(Clone, Debug)]
pub struct Dictionary {
    // exposed with pub(crate) so that it can be modified at tests
    pub(crate) mt: MerkleTree,
}

impl Dictionary {
    pub fn new(kvs: &HashMap<Hash, Value>) -> Result<Self> {
        let kvs: HashMap<Value, Value> = kvs.into_iter().map(|(&k, &v)| (Value(k.0), v)).collect();
        Ok(Self {
            mt: MerkleTree::new(MAX_DEPTH, &kvs)?,
        })
    }
    pub fn commitment(&self) -> Hash {
        self.mt.root()
    }
    pub fn get(&self, key: &Value) -> Result<Value> {
        self.mt.get(key)
    }
    pub fn prove(&self, key: &Value) -> Result<(Value, MerkleProof)> {
        self.mt.prove(key)
    }
    pub fn prove_nonexistence(&self, key: &Value) -> Result<MerkleProof> {
        self.mt.prove_nonexistence(key)
    }
    pub fn verify(root: Hash, proof: &MerkleProof, key: &Value, value: &Value) -> Result<()> {
        MerkleTree::verify(MAX_DEPTH, root, proof, key, value)
    }
    pub fn verify_nonexistence(root: Hash, proof: &MerkleProof, key: &Value) -> Result<()> {
        MerkleTree::verify_nonexistence(MAX_DEPTH, root, proof, key)
    }
    pub fn iter(&self) -> crate::primitives::merkletree::Iter {
        self.mt.iter()
    }
}
impl<'a> IntoIterator for &'a Dictionary {
    type Item = (&'a Value, &'a Value);
    type IntoIter = crate::primitives::merkletree::Iter<'a>;

    fn into_iter(self) -> Self::IntoIter {
        self.mt.iter()
    }
}

impl PartialEq for Dictionary {
    fn eq(&self, other: &Self) -> bool {
        self.mt.root() == other.mt.root()
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

impl Set {
    pub fn new(set: &Vec<Value>) -> Result<Self> {
        let kvs: HashMap<Value, Value> = set
            .into_iter()
            .map(|e| {
                let h = PoseidonHash::hash_no_pad(&e.0).elements;
                (Value(h), EMPTY)
            })
            .collect();
        Ok(Self {
            mt: MerkleTree::new(MAX_DEPTH, &kvs)?,
        })
    }
    pub fn commitment(&self) -> Hash {
        self.mt.root()
    }
    pub fn contains(&self, value: &Value) -> Result<bool> {
        self.mt.contains(value)
    }
    pub fn prove(&self, value: &Value) -> Result<MerkleProof> {
        let (_, proof) = self.mt.prove(value)?;
        Ok(proof)
    }
    pub fn prove_nonexistence(&self, value: &Value) -> Result<MerkleProof> {
        self.mt.prove_nonexistence(value)
    }
    pub fn verify(root: Hash, proof: &MerkleProof, value: &Value) -> Result<()> {
        MerkleTree::verify(MAX_DEPTH, root, proof, value, &EMPTY)
    }
    pub fn verify_nonexistence(root: Hash, proof: &MerkleProof, value: &Value) -> Result<()> {
        MerkleTree::verify_nonexistence(MAX_DEPTH, root, proof, value)
    }
    pub fn iter(&self) -> crate::primitives::merkletree::Iter {
        self.mt.iter()
    }
}

impl PartialEq for Set {
    fn eq(&self, other: &Self) -> bool {
        self.mt.root() == other.mt.root()
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

impl Array {
    pub fn new(array: &Vec<Value>) -> Result<Self> {
        let kvs: HashMap<Value, Value> = array
            .into_iter()
            .enumerate()
            .map(|(i, &e)| (Value::from(i as i64), e))
            .collect();

        Ok(Self {
            mt: MerkleTree::new(MAX_DEPTH, &kvs)?,
        })
    }
    pub fn commitment(&self) -> Hash {
        self.mt.root()
    }
    pub fn get(&self, i: usize) -> Result<Value> {
        self.mt.get(&Value::from(i as i64))
    }
    pub fn prove(&self, i: usize) -> Result<(Value, MerkleProof)> {
        self.mt.prove(&Value::from(i as i64))
    }
    pub fn verify(root: Hash, proof: &MerkleProof, i: usize, value: &Value) -> Result<()> {
        MerkleTree::verify(MAX_DEPTH, root, proof, &Value::from(i as i64), value)
    }
    pub fn iter(&self) -> crate::primitives::merkletree::Iter {
        self.mt.iter()
    }
}

impl PartialEq for Array {
    fn eq(&self, other: &Self) -> bool {
        self.mt.root() == other.mt.root()
    }
}
impl Eq for Array {}
