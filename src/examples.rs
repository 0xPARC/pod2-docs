use anyhow::Result;
use std::collections::HashMap;

use crate::backends::mock_signed::MockSigner;
use crate::frontend::{MainPodBuilder, SignedPod, SignedPodBuilder, Value};
use crate::middleware::{containers::Dictionary, Params, PodType, KEY_SIGNER, KEY_TYPE};
use crate::op;

// ZuKYC

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
) -> Result<MainPodBuilder> {
    let sanction_list = Value::Dictionary(Dictionary::new(&HashMap::new())?); // empty dictionary
    let now_minus_18y: i64 = 1169909388;
    let now_minus_1y: i64 = 1706367566;

    let mut kyc = MainPodBuilder::new(params);
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

    Ok(kyc)
}

// GreatBoy

pub fn good_boy_sign_pod_builder(params: &Params, user: &str, age: i64) -> SignedPodBuilder {
    let mut good_boy = SignedPodBuilder::new(params);
    good_boy.insert("user", user);
    good_boy.insert("age", age);

    good_boy
}

pub fn friend_sign_pod_builder(params: &Params, friend: &str) -> SignedPodBuilder {
    let mut friend_pod = SignedPodBuilder::new(params);
    friend_pod.insert("friend", friend);

    friend_pod
}

pub fn great_boy_pod_builder(
    params: &Params,
    good_boy_pods: [&SignedPod; 4],
    friend_pods: [&SignedPod; 2],
    good_boy_issuers: &Value,
    receiver: &str,
) -> MainPodBuilder {
    // Attestment chain (issuer -> good boy -> great boy):
    // issuer 0 -> good_boy_pods[0] => good boy 0
    // issuer 1 -> good_boy_pods[1] => good boy 0
    // issuer 2 -> good_boy_pods[2] => good boy 1
    // issuer 3 -> good_boy_pods[3] => good boy 1
    // good boy 0 -> friend_pods[0] => receiver
    // good boy 1 -> friend_pods[1] => receiver

    let mut great_boy = MainPodBuilder::new(params);
    for i in 0..4 {
        great_boy.add_signed_pod(&good_boy_pods[i]);
    }
    for i in 0..2 {
        great_boy.add_signed_pod(&friend_pods[i]);
    }

    for good_boy_idx in 0..2 {
        // Type check
        great_boy.pub_op(op!(
            eq,
            (friend_pods[good_boy_idx], KEY_TYPE),
            PodType::MockSigned as i64
        ));
        for issuer_idx in 0..2 {
            // Type check
            great_boy.pub_op(op!(
                eq,
                (good_boy_pods[good_boy_idx * 2 + issuer_idx], KEY_TYPE),
                PodType::MockSigned as i64
            ));
            // Each good boy POD comes from a valid issuer
            great_boy.pub_op(op!(
                contains,
                good_boy_issuers,
                (good_boy_pods[good_boy_idx * 2 + issuer_idx], KEY_SIGNER)
            ));
            // Each good boy has 2 good boy pods
            great_boy.pub_op(op!(
                eq,
                (good_boy_pods[good_boy_idx * 2 + issuer_idx], "user"),
                (friend_pods[good_boy_idx], KEY_SIGNER)
            ));
        }
        // The good boy PODs from each good boy have different issuers
        great_boy.pub_op(op!(
            ne,
            (good_boy_pods[good_boy_idx * 2 + 0], KEY_SIGNER),
            (good_boy_pods[good_boy_idx * 2 + 1], KEY_SIGNER)
        ));
        // Each good boy is receivers' friend
        great_boy.pub_op(op!(eq, (friend_pods[good_boy_idx], "friend"), receiver));
    }
    // The two good boys are different
    great_boy.pub_op(op!(
        ne,
        (friend_pods[0], KEY_SIGNER),
        (friend_pods[1], KEY_SIGNER)
    ));

    great_boy
}

pub fn great_boy_pod_full_flow() -> Result<MainPodBuilder> {
    let params = Params {
        max_input_signed_pods: 6,
        max_statements: 100,
        max_public_statements: 50,
        ..Default::default()
    };

    let good_boy_issuers = ["Giggles", "Macrosoft", "FaeBook"];
    let mut giggles_signer = MockSigner {
        pk: good_boy_issuers[0].into(),
    };
    let mut macrosoft_signer = MockSigner {
        pk: good_boy_issuers[1].into(),
    };
    let mut faebook_signer = MockSigner {
        pk: good_boy_issuers[2].into(),
    };
    let bob = "Bob";
    let charlie = "Charlie";
    let alice = "Alice";
    let mut bob_signer = MockSigner { pk: bob.into() };
    let mut charlie_signer = MockSigner { pk: charlie.into() };

    // Bob receives two good_boy pods from Giggles and Macrosoft.

    let bob = "Bob";
    let mut bob_good_boys = Vec::new();

    let good_boy = good_boy_sign_pod_builder(&params, &bob, 36);
    bob_good_boys.push(good_boy.sign(&mut giggles_signer).unwrap());
    bob_good_boys.push(good_boy.sign(&mut macrosoft_signer).unwrap());

    // Charlie receives two good_boy pods from Macrosoft and Faebook

    let charlie = "Charlie";
    let mut charlie_good_boys = Vec::new();

    let good_boy = good_boy_sign_pod_builder(&params, &charlie, 27);
    charlie_good_boys.push(good_boy.sign(&mut macrosoft_signer).unwrap());
    charlie_good_boys.push(good_boy.sign(&mut faebook_signer).unwrap());

    // Bob and Charlie send Alice a Friend POD

    let mut alice_friend_pods = Vec::new();
    let friend = friend_sign_pod_builder(&params, &alice);
    alice_friend_pods.push(friend.sign(&mut bob_signer).unwrap());
    alice_friend_pods.push(friend.sign(&mut charlie_signer).unwrap());

    let good_boy_issuers_dict = Value::Dictionary(Dictionary::new(&HashMap::new())?); // empty
    Ok(great_boy_pod_builder(
        &params,
        [
            &bob_good_boys[0],
            &bob_good_boys[1],
            &charlie_good_boys[0],
            &charlie_good_boys[1],
        ],
        [&alice_friend_pods[0], &alice_friend_pods[1]],
        &good_boy_issuers_dict,
        alice,
    ))
}

// Tickets

pub fn tickets_sign_pod_builder(params: &Params) -> SignedPodBuilder {
    // Create a signed pod with all atomic types (string, int, bool)
    let mut builder = SignedPodBuilder::new(params);
    builder.insert("eventId", 123);
    builder.insert("productId", 456);
    builder.insert("attendeeName", "John Doe");
    builder.insert("attendeeEmail", "john.doe@example.com");
    builder.insert("isConsumed", true);
    builder.insert("isRevoked", false);
    builder
}

pub fn tickets_pod_builder(
    params: &Params,
    signed_pod: &SignedPod,
    expected_event_id: i64,
    expect_consumed: bool,
    blacklisted_emails: &Value,
) -> MainPodBuilder {
    // Create a main pod referencing this signed pod with some statements
    let mut builder = MainPodBuilder::new(params);
    builder.add_signed_pod(signed_pod);
    builder.pub_op(op!(eq, (signed_pod, "eventId"), expected_event_id));
    builder.pub_op(op!(eq, (signed_pod, "isConsumed"), expect_consumed));
    builder.pub_op(op!(eq, (signed_pod, "isRevoked"), false));
    builder.pub_op(op!(
        not_contains,
        blacklisted_emails,
        (signed_pod, "attendeeEmail")
    ));
    builder
}

pub fn tickets_pod_full_flow() -> Result<MainPodBuilder> {
    let params = Params::default();
    let builder = tickets_sign_pod_builder(&params);
    let signed_pod = builder.sign(&mut MockSigner { pk: "test".into() }).unwrap();
    Ok(tickets_pod_builder(
        &params,
        &signed_pod,
        123,
        true,
        &Value::Dictionary(Dictionary::new(&HashMap::new())?),
    ))
}
