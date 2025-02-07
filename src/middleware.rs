//! The middleware includes the type definitions and the traits used to connect the frontend and
//! the backend.

use anyhow::Result;
use dyn_clone::DynClone;
use hex::{FromHex, FromHexError};
use itertools::Itertools;
use plonky2::field::goldilocks_field::GoldilocksField;
use plonky2::field::types::{Field, PrimeField64};
use plonky2::hash::poseidon::PoseidonHash;
use plonky2::plonk::config::{Hasher, PoseidonGoldilocksConfig};
use std::any::Any;
use std::cmp::{Ord, Ordering};
use std::collections::HashMap;
use std::fmt;
use strum_macros::FromRepr;

pub const KEY_SIGNER: &str = "_signer";
pub const KEY_TYPE: &str = "_type";
pub const STATEMENT_ARG_F_LEN: usize = 8;

/// F is the native field we use everywhere.  Currently it's Goldilocks from plonky2
pub type F = GoldilocksField;
/// C is the Plonky2 config used in POD2 to work with Plonky2 recursion.
pub type C = PoseidonGoldilocksConfig;
/// D defines the extension degree of the field used in the Plonky2 proofs (quadratic extension).
pub const D: usize = 2;

#[derive(Clone, Copy, Debug, Default, Hash, PartialEq, Eq)]
pub struct Value(pub [F; 4]);

impl Ord for Value {
    fn cmp(&self, other: &Self) -> Ordering {
        for (lhs, rhs) in self.0.iter().zip(other.0.iter()).rev() {
            let (lhs, rhs) = (lhs.to_canonical_u64(), rhs.to_canonical_u64());
            if lhs < rhs {
                return Ordering::Less;
            } else if lhs > rhs {
                return Ordering::Greater;
            }
        }
        return Ordering::Equal;
    }
}

impl PartialOrd for Value {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl From<i64> for Value {
    fn from(v: i64) -> Self {
        let lo = F::from_canonical_u64((v as u64) & 0xffffffff);
        let hi = F::from_canonical_u64((v as u64) >> 32);
        Value([lo, hi, F::ZERO, F::ZERO])
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.0[2].is_zero() && self.0[3].is_zero() {
            // Assume this is an integer
            let (l0, l1) = (self.0[0].to_canonical_u64(), self.0[1].to_canonical_u64());
            assert!(l0 < (1 << 32));
            assert!(l1 < (1 << 32));
            write!(f, "{}", l0 + l1 * (1 << 32))
        } else {
            // Assume this is a hash
            Hash(self.0).fmt(f)
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Hash, Eq, PartialEq)]
pub struct Hash(pub [F; 4]);

impl ToFields for Hash {
    fn to_fields(self) -> (Vec<F>, usize) {
        (self.0.to_vec(), 4)
    }
}

impl Ord for Hash {
    fn cmp(&self, other: &Self) -> Ordering {
        Value(self.0).cmp(&Value(other.0))
    }
}

impl PartialOrd for Hash {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

pub const NULL: Hash = Hash([F::ZERO, F::ZERO, F::ZERO, F::ZERO]);

impl fmt::Display for Hash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let v0 = self.0[0].to_canonical_u64();
        for i in 0..4 {
            write!(f, "{:02x}", (v0 >> (i * 8)) & 0xff)?;
        }
        write!(f, "…")
    }
}

impl FromHex for Hash {
    type Error = FromHexError;

    fn from_hex<T: AsRef<[u8]>>(hex: T) -> Result<Self, Self::Error> {
        // In little endian
        let bytes = <[u8; 32]>::from_hex(hex)?;
        let mut buf: [u8; 8] = [0; 8];
        let mut inner = [F::ZERO; 4];
        for i in 0..4 {
            buf.copy_from_slice(&bytes[8 * i..8 * (i + 1)]);
            inner[i] = F::from_canonical_u64(u64::from_le_bytes(buf));
        }
        Ok(Self(inner))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub struct PodId(pub Hash);

impl ToFields for PodId {
    fn to_fields(self) -> (Vec<F>, usize) {
        self.0.to_fields()
    }
}

pub const SELF: PodId = PodId(Hash([F::ONE, F::ZERO, F::ZERO, F::ZERO]));

impl fmt::Display for PodId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if *self == SELF {
            write!(f, "self")
        } else if self.0 == NULL {
            write!(f, "null")
        } else {
            write!(f, "{}", self.0)
        }
    }
}

pub enum PodType {
    None = 0,
    MockSigned = 1,
    MockMain = 2,
    Signed = 3,
    Main = 4,
}

impl From<PodType> for Value {
    fn from(v: PodType) -> Self {
        Value::from(v as i64)
    }
}

pub fn hash_str(s: &str) -> Hash {
    let mut input = s.as_bytes().to_vec();
    input.push(1); // padding
                   // Merge 7 bytes into 1 field, because the field is slightly below 64 bits
    let input: Vec<F> = input
        .chunks(7)
        .map(|bytes| {
            let mut v: u64 = 0;
            for b in bytes.iter().rev() {
                v <<= 8;
                v += *b as u64;
            }
            F::from_canonical_u64(v)
        })
        .collect();
    Hash(PoseidonHash::hash_no_pad(&input).elements)
}

#[derive(Clone, Debug, Copy)]
pub struct Params {
    pub max_input_signed_pods: usize,
    pub max_input_main_pods: usize,
    pub max_statements: usize,
    pub max_signed_pod_values: usize,
    pub max_public_statements: usize,
    pub max_statement_args: usize,
    pub max_operation_args: usize,
}

impl Params {
    pub fn max_priv_statements(&self) -> usize {
        self.max_statements - self.max_public_statements
    }
}

impl Default for Params {
    fn default() -> Self {
        Self {
            max_input_signed_pods: 3,
            max_input_main_pods: 3,
            max_statements: 20,
            max_signed_pod_values: 8,
            max_public_statements: 10,
            max_statement_args: 5,
            max_operation_args: 5,
        }
    }
}

pub trait SignedPod: fmt::Debug + DynClone {
    fn verify(&self) -> bool;
    fn id(&self) -> PodId;
    // NOTE: Maybe replace this by
    // - `get(key: Hash) -> Option<Value>`
    // - `iter() -> impl Iter<(Hash, Value)>`
    fn kvs(&self) -> HashMap<Hash, Value>;
    fn pub_statements(&self) -> Vec<Statement> {
        let id = self.id();
        let mut statements = Vec::new();
        for (k, v) in self.kvs().iter().sorted_by_key(|kv| kv.0) {
            statements.push(Statement(
                NativeStatement::ValueOf,
                vec![
                    StatementArg::Key(AnchoredKey(id, *k)),
                    StatementArg::Literal(*v),
                ],
            ));
        }
        statements
    }
    // Used for downcasting
    fn into_any(self: Box<Self>) -> Box<dyn Any>;
}

// impl Clone for Box<dyn SignedPod>
dyn_clone::clone_trait_object!(SignedPod);

/// This is a filler type that fulfills the SignedPod trait and always verifies.  It's empty.  This
/// can be used to simulate padding in a circuit.
#[derive(Debug, Clone)]
pub struct NoneSignedPod {}

impl SignedPod for NoneSignedPod {
    fn verify(&self) -> bool {
        true
    }
    fn id(&self) -> PodId {
        PodId(NULL)
    }
    fn kvs(&self) -> HashMap<Hash, Value> {
        HashMap::new()
    }
    fn pub_statements(&self) -> Vec<Statement> {
        Vec::new()
    }
    fn into_any(self: Box<Self>) -> Box<dyn Any> {
        self
    }
}

pub trait PodSigner {
    fn sign(&mut self, params: &Params, kvs: &HashMap<Hash, Value>) -> Result<Box<dyn SignedPod>>;
}

#[derive(Clone, Copy, Debug, FromRepr, PartialEq, Eq)]
pub enum NativeStatement {
    None = 0,
    ValueOf = 1,
    Equal = 2,
    NotEqual = 3,
    Gt = 4,
    Lt = 5,
    Contains = 6,
    NotContains = 7,
    SumOf = 8,
    ProductOf = 9,
    MaxOf = 10,
}

impl ToFields for NativeStatement {
    fn to_fields(self) -> (Vec<F>, usize) {
        (vec![F::from_canonical_u64(self as u64)], 1)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct AnchoredKey(pub PodId, pub Hash);

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StatementArg {
    None,
    Literal(Value),
    Key(AnchoredKey),
}

impl fmt::Display for StatementArg {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            StatementArg::None => write!(f, "none"),
            StatementArg::Literal(v) => write!(f, "{}", v),
            StatementArg::Key(r) => write!(f, "{}.{}", r.0, r.1),
        }
    }
}

impl StatementArg {
    pub fn is_none(&self) -> bool {
        matches!(self, Self::None)
    }
}

impl ToFields for StatementArg {
    fn to_fields(self) -> (Vec<F>, usize) {
        // NOTE: current version returns always the same amount of field elements in the returned
        // vector, which means that the `None` case is padded with 8 zeroes, and the `Literal` case
        // is padded with 4 zeroes. Since the returned vector will mostly be hashed (and reproduced
        // in-circuit), we might be interested into reducing the length of it. If that's the case,
        // we can check if it makes sense to make it dependant on the concrete StatementArg; that
        // is, when dealing with a `None` it would be a single field element (zero value), and when
        // dealing with `Literal` it would be of length 4.
        let f = match self {
            StatementArg::None => vec![F::ZERO; STATEMENT_ARG_F_LEN],
            StatementArg::Literal(v) => {
                let value_f = v.0.to_vec();
                [
                    value_f.clone(),
                    vec![F::ZERO; STATEMENT_ARG_F_LEN - value_f.len()],
                ]
                .concat()
            }
            StatementArg::Key(ak) => {
                let (podid_f, _) = ak.0.to_fields();
                let (hash_f, _) = ak.1.to_fields();
                [podid_f, hash_f].concat()
            }
        };
        assert_eq!(f.len(), STATEMENT_ARG_F_LEN); // sanity check
        (f, STATEMENT_ARG_F_LEN)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Statement(pub NativeStatement, pub Vec<StatementArg>);

impl fmt::Display for Statement {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?} ", self.0)?;
        for (i, arg) in self.1.iter().enumerate() {
            if !(!f.alternate() && arg.is_none()) {
                if i != 0 {
                    write!(f, " ")?;
                }
                write!(f, "{}", arg)?;
            }
        }
        Ok(())
    }
}

impl Statement {
    pub fn is_none(&self) -> bool {
        matches!(self.0, NativeStatement::None)
    }
}

impl ToFields for Statement {
    fn to_fields(self) -> (Vec<F>, usize) {
        let (native_statement_f, native_statement_f_len) = self.0.to_fields();
        let (vec_statementarg_f, vec_statementarg_f_len) = self
            .1
            .into_iter()
            .map(|statement_arg| statement_arg.to_fields())
            .fold((Vec::new(), 0), |mut acc, (f, l)| {
                acc.0.extend(f);
                acc.1 += l;
                acc
            });
        (
            [native_statement_f, vec_statementarg_f].concat(),
            native_statement_f_len + vec_statementarg_f_len,
        )
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NativeOperation {
    None = 0,
    NewEntry = 1,
    CopyStatement = 2,
    EqualFromEntries = 3,
    NotEqualFromEntries = 4,
    GtFromEntries = 5,
    LtFromEntries = 6,
    TransitiveEqualFromStatements = 7,
    GtToNotEqual = 8,
    LtToNotEqual = 9,
    ContainsFromEntries = 10,
    NotContainsFromEntries = 11,
    RenameContainedBy = 12,
    SumOf = 13,
    ProductOf = 14,
    MaxOf = 15,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum OperationArg {
    None,
    Statement(Statement),
    Key(AnchoredKey),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Operation(pub NativeOperation, pub Vec<OperationArg>);

pub trait MainPod: fmt::Debug + DynClone {
    fn verify(&self) -> bool;
    fn id(&self) -> PodId;
    fn pub_statements(&self) -> Vec<Statement>;
    // Used for downcasting
    fn into_any(self: Box<Self>) -> Box<dyn Any>;
}

// impl Clone for Box<dyn SignedPod>
dyn_clone::clone_trait_object!(MainPod);

/// This is a filler type that fulfills the MainPod trait and always verifies.  It's empty.  This
/// can be used to simulate padding in a circuit.
#[derive(Debug, Clone)]
pub struct NoneMainPod {}

impl MainPod for NoneMainPod {
    fn verify(&self) -> bool {
        true
    }
    fn id(&self) -> PodId {
        PodId(NULL)
    }
    fn pub_statements(&self) -> Vec<Statement> {
        Vec::new()
    }
    fn into_any(self: Box<Self>) -> Box<dyn Any> {
        self
    }
}

#[derive(Debug)]
pub struct MainPodInputs<'a> {
    pub signed_pods: &'a [&'a Box<dyn SignedPod>],
    pub main_pods: &'a [&'a Box<dyn MainPod>],
    pub statements: &'a [Statement],
    pub operations: &'a [Operation],
    /// Statements that need to be made public (they can come from input pods or input
    /// statements)
    pub public_statements: &'a [Statement],
}

pub trait PodProver {
    fn prove(&mut self, params: &Params, inputs: MainPodInputs) -> Result<Box<dyn MainPod>>;
}

pub trait ToFields {
    /// returns Vec<F> representation of the type, and a usize indicating how many field elements
    /// does the vector contain
    fn to_fields(self) -> (Vec<F>, usize);
}
