/// MerkleTree implementation for POD2.
///
/// Current implementation is a wrapper on top of Plonky2's MerkleTree, but the future iteration
/// will replace it by the MerkleTree specified at https://0xparc.github.io/pod2/merkletree.html .
use anyhow::{anyhow, Result};
use itertools::Itertools;
use plonky2::field::types::Field;
use plonky2::hash::{
    hash_types::HashOut,
    merkle_proofs::{verify_merkle_proof, MerkleProof as PlonkyMerkleProof},
    merkle_tree::MerkleTree as PlonkyMerkleTree,
    poseidon::PoseidonHash,
};
use plonky2::plonk::config::GenericConfig;
use plonky2::plonk::config::Hasher;
use std::collections::HashMap;
use std::iter::IntoIterator;

use crate::middleware::{Hash, Value, C, D, F};

const CAP_HEIGHT: usize = 0;

/// MerkleTree currently is a wrapper on top of Plonky2's MerkleTree. A future iteration will
/// replace it by the MerkleTree specified at https://0xparc.github.io/pod2/merkletree.html .
#[derive(Clone, Debug)]
pub struct MerkleTree {
    tree: PlonkyMerkleTree<F, <C as GenericConfig<D>>::Hasher>,
    // keyindex: key -> index mapping. This is just for the current plonky-tree wrapper
    keyindex: HashMap<Value, usize>,
    // kvs are a field in the MerkleTree in order to be able to iterate over the keyvalues. This is
    // specific of the current implementation (Plonky2's tree wrapper), in the next iteration this
    // will not be needed since the tree implementation itself will offer the hashmap
    // functionality.
    pub kvs: HashMap<Value, Value>,
    // leaves_map is a map between the leaf (leaf=Hash(key,value)) and the actual (key, value). It
    // is used to get the actual value from a leaf for a given key (through the method
    // `MerkleTree.get`.
    leaves_map: HashMap<Hash, (Value, Value)>,
}

pub struct MerkleProof {
    existence: bool,
    index: usize,
    proof: PlonkyMerkleProof<F, <C as GenericConfig<D>>::Hasher>,
}

impl MerkleTree {
    /// builds a new `MerkleTree` where the leaves contain the given key-values
    pub fn new(kvs: &HashMap<Value, Value>) -> Self {
        let mut keyindex: HashMap<Value, usize> = HashMap::new();
        let mut leaves: Vec<Vec<F>> = Vec::new();
        let mut leaves_map: HashMap<Hash, (Value, Value)> = HashMap::new();
        // Note: current version iterates sorting by keys of the kvs, but the merkletree defined at
        // https://0xparc.github.io/pod2/merkletree.html will not need it since it will be
        // deterministic based on the keys values not on the order of the keys when added into the
        // tree.
        for (i, (k, v)) in kvs.iter().sorted_by_key(|kv| kv.0).enumerate() {
            let input: Vec<F> = [k.0, v.0].concat();
            let leaf = PoseidonHash::hash_no_pad(&input).elements;
            leaves.push(leaf.into());
            keyindex.insert(*k, i);
            leaves_map.insert(Hash(leaf), (*k, *v));
        }

        // pad to a power of two if needed
        let leaf_empty: Vec<F> = vec![F::ZERO, F::ZERO, F::ZERO, F::ZERO];
        for _ in leaves.len()..leaves.len().next_power_of_two() {
            leaves.push(leaf_empty.clone());
        }

        let tree = PlonkyMerkleTree::<F, <C as GenericConfig<D>>::Hasher>::new(leaves, CAP_HEIGHT);
        Self {
            tree,
            keyindex,
            kvs: kvs.clone(),
            leaves_map,
        }
    }
}

impl MerkleTree {
    /// returns the root of the tree
    pub fn root(&self) -> Hash {
        if self.tree.cap.is_empty() {
            return crate::middleware::NULL;
        }
        Hash(self.tree.cap.0[0].elements)
    }

    /// returns the value at the given key
    pub fn get(&self, key: &Value) -> Result<Value> {
        let i = self.keyindex.get(&key).ok_or(anyhow!("key not in tree"))?;
        let leaf_hash_raw = self.tree.get(*i);
        let leaf_hash_f: [F; 4] = leaf_hash_raw
            .try_into()
            .map_err(|_| anyhow!("unexpected length (len!=4)"))?;
        let leaf_hash: Hash = Hash(leaf_hash_f);
        let (_, value) = self.leaves_map.get(&leaf_hash).unwrap();
        Ok(*value)
    }

    /// returns a boolean indicating whether the key exists in the tree
    pub fn contains(&self, key: &Value) -> bool {
        self.keyindex.get(&key).is_some()
    }

    /// returns a proof of existence, which proves that the given key exists in
    /// the tree. It returns the `MerkleProof`.
    pub fn prove(&self, key: &Value) -> Result<MerkleProof> {
        let i = self.keyindex.get(&key).ok_or(anyhow!("key not in tree"))?;
        let proof = self.tree.prove(*i);
        Ok(MerkleProof {
            existence: true,
            index: *i,
            proof,
        })
    }

    /// returns a proof of non-existence, which proves that the given `key`
    /// does not exist in the tree
    pub fn prove_nonexistence(&self, _key: &Value) -> Result<MerkleProof> {
        // mock method
        println!("WARNING: MerkleTree::verify_nonexistence is currently a mock");
        Ok(MerkleProof {
            existence: false,
            index: 0,
            proof: PlonkyMerkleProof { siblings: vec![] },
        })
    }

    /// verifies an inclusion proof for the given `key` and `value`
    pub fn verify(root: Hash, proof: &MerkleProof, key: &Value, value: &Value) -> Result<()> {
        if !proof.existence {
            return Err(anyhow!(
                "expected proof of existence, found proof of non-existence"
            ));
        }
        let leaf = PoseidonHash::hash_no_pad(&[key.0, value.0].concat()).elements;
        let root = HashOut::from_vec(root.0.to_vec());
        verify_merkle_proof(leaf.into(), proof.index, root, &proof.proof)
    }

    /// verifies a non-inclusion proof for the given `key`, that is, the given
    /// `key` does not exist in the tree
    pub fn verify_nonexistence(_root: Hash, proof: &MerkleProof, _key: &Value) -> Result<()> {
        // mock method
        if proof.existence {
            return Err(anyhow!(
                "expected proof of non-existence, found proof of existence"
            ));
        }
        println!("WARNING: MerkleTree::verify_nonexistence is currently a mock");
        Ok(())
    }

    /// returns an iterator over the leaves of the tree
    pub fn iter(&self) -> std::collections::hash_map::Iter<Value, Value> {
        self.kvs.iter()
    }
}

impl<'a> IntoIterator for &'a MerkleTree {
    type Item = (&'a Value, &'a Value);
    type IntoIter = std::collections::hash_map::Iter<'a, Value, Value>;

    fn into_iter(self) -> Self::IntoIter {
        self.kvs.iter()
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;

    use crate::middleware::hash_str;

    #[test]
    fn test_merkletree() -> Result<()> {
        let (k0, v0) = (
            Value(hash_str("key_0".into()).0),
            Value(hash_str("value_0".into()).0),
        );
        let (k1, v1) = (
            Value(hash_str("key_1".into()).0),
            Value(hash_str("value_1".into()).0),
        );
        let (k2, v2) = (
            Value(hash_str("key_2".into()).0),
            Value(hash_str("value_2".into()).0),
        );

        let mut kvs = HashMap::new();
        kvs.insert(k0, v0);
        kvs.insert(k1, v1);
        kvs.insert(k2, v2);

        let tree = MerkleTree::new(&kvs);

        let proof = tree.prove(&k2)?;
        MerkleTree::verify(tree.root(), &proof, &k2, &v2)?;

        // expect verification to fail with different key / value
        assert!(MerkleTree::verify(tree.root(), &proof, &k2, &v0).is_err());
        assert!(MerkleTree::verify(tree.root(), &proof, &k0, &v2).is_err());

        // non-existence proofs
        let proof_ne = tree.prove_nonexistence(&k2)?;
        let _ = MerkleTree::verify_nonexistence(tree.root(), &proof_ne, &k2)?;

        // expect verification of existence fail for nonexistence proof
        let _ = MerkleTree::verify(tree.root(), &proof_ne, &k2, &v2).is_err();

        Ok(())
    }
}
