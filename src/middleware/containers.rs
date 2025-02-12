/// This file implements the types defined at
/// https://0xparc.github.io/pod2/values.html#dictionary-array-set .
use plonky2::hash::poseidon::PoseidonHash;
use plonky2::plonk::config::Hasher;
use std::collections::HashMap;

use super::{Hash, Value, EMPTY};
use crate::primitives::merkletree::MerkleTree;

/// Container is a wrapper of a MerkleTree, used to achieve Dictionary, Set, Array frontend types.
pub trait Container {
    type Raw: Clone;

    fn new(raw: &Self::Raw) -> Self;

    /// returns the commitment to the container
    fn cm(&self) -> Hash;
}

/// Dictionary: the user original keys and values are hashed to be used in the leaf.
///    leaf.key=hash(original_key)
///    leaf.value=hash(original_value)
#[derive(Clone, Debug)]
pub struct Dictionary {
    pub mt: MerkleTree,
}

impl Container for Dictionary {
    type Raw = HashMap<Hash, Value>;

    fn new(raw: &Self::Raw) -> Self {
        let kvs: HashMap<Value, Value> = raw.into_iter().map(|(&k, &v)| (Value(k.0), v)).collect();
        Self {
            mt: MerkleTree::new(&kvs),
        }
    }

    fn cm(&self) -> Hash {
        self.mt.root()
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
            mt: MerkleTree::new(&kvs),
        }
    }

    fn cm(&self) -> Hash {
        self.mt.root()
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
            mt: MerkleTree::new(&kvs),
        }
    }

    fn cm(&self) -> Hash {
        self.mt.root()
    }
}

impl PartialEq for Array {
    fn eq(&self, other: &Self) -> bool {
        self.mt.root() == other.mt.root() && self.mt.root() == other.mt.root()
    }
}
impl Eq for Array {}
