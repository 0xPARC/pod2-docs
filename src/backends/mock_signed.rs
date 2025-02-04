use crate::middleware::{
    hash_str, Hash, Params, PodId, PodSigner, PodType, SignedPod, Value, KEY_SIGNER, KEY_TYPE,
};
use itertools::Itertools;
use std::any::Any;
use std::collections::HashMap;

pub struct MockSigner {
    pub pk: String,
}

fn calculate_pod_id(kvs: &HashMap<Hash, Value>) -> PodId {
    let mut s = String::new();
    for (k, v) in kvs.iter().sorted_by_key(|kv| kv.0) {
        s += &format!("{}:{},", k, v);
    }
    PodId(hash_str(&s))
}

impl PodSigner for MockSigner {
    fn sign(&mut self, _params: &Params, kvs: &HashMap<Hash, Value>) -> Box<dyn SignedPod> {
        let mut kvs = kvs.clone();
        let pk_hash = hash_str(&self.pk);
        kvs.insert(hash_str(&KEY_SIGNER), Value(pk_hash.0));
        kvs.insert(hash_str(&KEY_TYPE), Value::from(PodType::MockSigned));

        let id = calculate_pod_id(&kvs);
        let signature = format!("{}_signed_by_{}", id, pk_hash);
        Box::new(MockSignedPod {
            kvs: kvs.clone(),
            id,
            signature,
        })
    }
}

#[derive(Clone, Debug)]
pub struct MockSignedPod {
    pub id: PodId,
    pub signature: String,
    pub kvs: HashMap<Hash, Value>,
}

impl SignedPod for MockSignedPod {
    fn verify(&self) -> bool {
        // Verify type
        if Some(&Value::from(PodType::MockSigned)) != self.kvs.get(&hash_str(&KEY_TYPE)) {
            return false;
        }

        // Verify id
        let id = calculate_pod_id(&self.kvs);
        if id != self.id {
            return false;
        }

        // Verify signature
        let pk_hash = match self.kvs.get(&hash_str(&KEY_SIGNER)) {
            Some(v) => v,
            None => return false,
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
        self.kvs.clone()
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
        let pod = pod.sign(&mut signer);
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
        bad_pod
            .kvs
            .insert(hash_str(KEY_SIGNER), Value(PodId(NULL).0 .0));
        assert_eq!(bad_pod.verify(), false);

        let mut bad_pod = pod.clone();
        bad_pod.kvs.insert(hash_str(KEY_TYPE), Value::from(0));
        assert_eq!(bad_pod.verify(), false);
    }
}
