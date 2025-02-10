use crate::frontend::{MainPodBuilder, MerkleTree, SignedPod, SignedPodBuilder, Value};
use crate::middleware::Params;
use crate::op;

pub fn zu_kyc_sign_pod_builders(params: &Params) -> (SignedPodBuilder, SignedPodBuilder) {
    let mut gov_id = SignedPodBuilder::new(params);
    gov_id.insert("idNumber", "4242424242");
    gov_id.insert("dateOfBirth", 1169909384);
    gov_id.insert("socialSecurityNumber", "G2121210");

    let mut pay_stub = SignedPodBuilder::new(params);
    pay_stub.insert("socialSecurityNumber", "G2121210");
    pay_stub.insert("startDate", 1706367566);

    (gov_id, pay_stub)
}

pub fn zu_kyc_pod_builder(
    params: &Params,
    gov_id: &SignedPod,
    pay_stub: &SignedPod,
) -> MainPodBuilder {
    let sanction_list = Value::MerkleTree(MerkleTree { root: 1 });
    let now_minus_18y: i64 = 1169909388;
    let now_minus_1y: i64 = 1706367566;

    let mut kyc = MainPodBuilder::new(&params);
    kyc.add_signed_pod(&gov_id);
    kyc.add_signed_pod(&pay_stub);
    kyc.pub_op(op!(not_contains, &sanction_list, (gov_id, "idNumber")));
    kyc.pub_op(op!(lt, (gov_id, "dateOfBirth"), now_minus_18y));
    kyc.pub_op(op!(
        eq,
        (gov_id, "socialSecurityNumber"),
        (pay_stub, "socialSecurityNumber")
    ));
    kyc.pub_op(op!(eq, (pay_stub, "startDate"), now_minus_1y));

    kyc
}
