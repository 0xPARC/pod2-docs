use crate::middleware::{
    containers::Dictionary, hash_str, Hash, Params, PodId, PodSigner, PodType, SignedPod, Value,
    KEY_SIGNER, KEY_TYPE,
};
use crate::primitives::merkletree::MerkleTree;
use anyhow::Result;
use std::any::Any;
use std::collections::HashMap;

pub struct MockSigner {
    pub pk: String,
}

impl PodSigner for MockSigner {
    fn sign(&mut self, _params: &Params, kvs: &HashMap<Hash, Value>) -> Result<Box<dyn SignedPod>> {
        let mut kvs = kvs.clone();
        let pk_hash = hash_str(&self.pk);
        kvs.insert(hash_str(&KEY_SIGNER), Value(pk_hash.0));
        kvs.insert(hash_str(&KEY_TYPE), Value::from(PodType::MockSigned));

        let dict = Dictionary::new(&kvs);
        let id = PodId(dict.commitment());
        let signature = format!("{}_signed_by_{}", id, pk_hash);
        Ok(Box::new(MockSignedPod {
            dict,
            id,
            signature,
        }))
    }
}

#[derive(Clone, Debug)]
pub struct MockSignedPod {
    id: PodId,
    signature: String,
    dict: Dictionary,
}

impl SignedPod for MockSignedPod {
    fn verify(&self) -> bool {
        // Verify type
        let value_at_type = match self.dict.get(&hash_str(&KEY_TYPE).into()) {
            Ok(v) => v,
            Err(_) => return false,
        };
        if Value::from(PodType::MockSigned) != value_at_type {
            return false;
        }

        // Verify id
        let mt = MerkleTree::new(
            &self
                .dict
                .iter()
                .map(|(&k, &v)| (k, v))
                .collect::<HashMap<Value, Value>>(),
        );
        let id = PodId(mt.root());
        if id != self.id {
            return false;
        }

        // Verify signature
        let pk_hash = match self.dict.get(&hash_str(&KEY_SIGNER).into()) {
            Ok(v) => v,
            Err(_) => return false,
        };
        let signature = format!("{}_signed_by_{}", id, pk_hash);
        if signature != self.signature {
            return false;
        }

        return true;
    }

    fn id(&self) -> PodId {
        self.id
    }

    fn kvs(&self) -> HashMap<Hash, Value> {
        self.dict
            .into_iter()
            .map(|(&k, &v)| (Hash(k.0), v))
            .collect()
    }

    fn into_any(self: Box<Self>) -> Box<dyn Any> {
        self
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use crate::frontend;
    use crate::middleware::{self, F, NULL};
    use plonky2::field::types::Field;

    #[test]
    fn test_mock_signed_0() {
        let params = middleware::Params::default();
        let mut pod = frontend::SignedPodBuilder::new(&params);
        pod.insert("idNumber", "4242424242");
        pod.insert("dateOfBirth", 1169909384);
        pod.insert("socialSecurityNumber", "G2121210");

        let mut signer = MockSigner { pk: "Molly".into() };
        let pod = pod.sign(&mut signer).unwrap();
        let pod = pod.pod.into_any().downcast::<MockSignedPod>().unwrap();

        assert_eq!(pod.verify(), true);
        println!("id: {}", pod.id());
        println!("kvs: {:?}", pod.kvs());

        let mut bad_pod = pod.clone();
        bad_pod.signature = "".into();
        assert_eq!(bad_pod.verify(), false);

        let mut bad_pod = pod.clone();
        bad_pod.id.0 .0[0] = F::ZERO;
        assert_eq!(bad_pod.verify(), false);

        let mut bad_pod = pod.clone();
        let mut bad_kvs = bad_pod.kvs();
        bad_kvs.insert(hash_str(KEY_SIGNER), Value(PodId(NULL).0 .0));
        let bad_kvs_mt = &bad_kvs
            .into_iter()
            .map(|(k, v)| (Value(k.0), v))
            .collect::<HashMap<Value, Value>>();
        let bad_mt = MerkleTree::new(&bad_kvs_mt);
        bad_pod.dict.mt = bad_mt;
        assert_eq!(bad_pod.verify(), false);

        let mut bad_pod = pod.clone();
        let mut bad_kvs = bad_pod.kvs();
        bad_kvs.insert(hash_str(KEY_TYPE), Value::from(0));
        let bad_kvs_mt = &bad_kvs
            .into_iter()
            .map(|(k, v)| (Value(k.0), v))
            .collect::<HashMap<Value, Value>>();
        let bad_mt = MerkleTree::new(&bad_kvs_mt);
        bad_pod.dict.mt = bad_mt;
        assert_eq!(bad_pod.verify(), false);
    }
}
