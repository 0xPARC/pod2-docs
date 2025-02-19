//! The middleware includes the type definitions and the traits used to connect the frontend and
//! the backend.

use anyhow::{anyhow, Error, Result};
use dyn_clone::DynClone;
use hex::{FromHex, FromHexError};
use plonky2::field::goldilocks_field::GoldilocksField;
use plonky2::field::types::{Field, PrimeField64};
use plonky2::hash::poseidon::PoseidonHash;
use plonky2::plonk::config::{Hasher, PoseidonGoldilocksConfig};
use std::any::Any;
use std::cmp::{Ord, Ordering};
use std::collections::HashMap;
use std::fmt;
use strum_macros::FromRepr;

pub mod containers;

pub const KEY_SIGNER: &str = "_signer";
pub const KEY_TYPE: &str = "_type";
pub const STATEMENT_ARG_F_LEN: usize = 8;

/// F is the native field we use everywhere.  Currently it's Goldilocks from plonky2
pub type F = GoldilocksField;
/// C is the Plonky2 config used in POD2 to work with Plonky2 recursion.
pub type C = PoseidonGoldilocksConfig;
/// D defines the extension degree of the field used in the Plonky2 proofs (quadratic extension).
pub const D: usize = 2;

pub type Entry = (String, Value);

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

impl From<Hash> for Value {
    fn from(h: Hash) -> Self {
        Value(h.0)
    }
}

impl TryInto<i64> for Value {
    type Error = Error;
    fn try_into(self) -> std::result::Result<i64, Self::Error> {
        let value = self.0;
        if &value[2..] != &[F::ZERO, F::ZERO]
            || value[..2]
                .iter()
                .all(|x| x.to_canonical_u64() > u32::MAX as u64)
        {
            Err(anyhow!("Value not an element of the i64 embedding."))
        } else {
            Ok((value[0].to_canonical_u64() + value[1].to_canonical_u64() << 32) as i64)
        }
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

pub const EMPTY: Value = Value([F::ZERO, F::ZERO, F::ZERO, F::ZERO]);
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
/// AnchoredKey is a tuple containing (OriginId: PodId, key: Hash)
pub struct AnchoredKey(pub PodId, pub Hash);

impl AnchoredKey {
    pub fn origin(&self) -> PodId {
        self.0
    }
    pub fn key(&self) -> Hash {
        self.1
    }
}

// TODO: Incorporate custom statements into this enum.
/// Type encapsulating statements with their associated arguments.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Statement {
    None,
    ValueOf(AnchoredKey, Value),
    Equal(AnchoredKey, AnchoredKey),
    NotEqual(AnchoredKey, AnchoredKey),
    Gt(AnchoredKey, AnchoredKey),
    Lt(AnchoredKey, AnchoredKey),
    Contains(AnchoredKey, AnchoredKey),
    NotContains(AnchoredKey, AnchoredKey),
    SumOf(AnchoredKey, AnchoredKey, AnchoredKey),
    ProductOf(AnchoredKey, AnchoredKey, AnchoredKey),
    MaxOf(AnchoredKey, AnchoredKey, AnchoredKey),
}

impl Statement {
    pub fn is_none(&self) -> bool {
        self == &Self::None
    }
    pub fn code(&self) -> NativeStatement {
        match self {
            Self::None => NativeStatement::None,
            Self::ValueOf(_, _) => NativeStatement::ValueOf,
            Self::Equal(_, _) => NativeStatement::Equal,
            Self::NotEqual(_, _) => NativeStatement::NotEqual,
            Self::Gt(_, _) => NativeStatement::Gt,
            Self::Lt(_, _) => NativeStatement::Lt,
            Self::Contains(_, _) => NativeStatement::Contains,
            Self::NotContains(_, _) => NativeStatement::NotContains,
            Self::SumOf(_, _, _) => NativeStatement::SumOf,
            Self::ProductOf(_, _, _) => NativeStatement::ProductOf,
            Self::MaxOf(_, _, _) => NativeStatement::MaxOf,
        }
    }
    pub fn args(&self) -> Vec<StatementArg> {
        use StatementArg::*;
        match self.clone() {
            Self::None => vec![],
            Self::ValueOf(ak, v) => vec![Key(ak), Literal(v)],
            Self::Equal(ak1, ak2) => vec![Key(ak1), Key(ak2)],
            Self::NotEqual(ak1, ak2) => vec![Key(ak1), Key(ak2)],
            Self::Gt(ak1, ak2) => vec![Key(ak1), Key(ak2)],
            Self::Lt(ak1, ak2) => vec![Key(ak1), Key(ak2)],
            Self::Contains(ak1, ak2) => vec![Key(ak1), Key(ak2)],
            Self::NotContains(ak1, ak2) => vec![Key(ak1), Key(ak2)],
            Self::SumOf(ak1, ak2, ak3) => vec![Key(ak1), Key(ak2), Key(ak3)],
            Self::ProductOf(ak1, ak2, ak3) => vec![Key(ak1), Key(ak2), Key(ak3)],
            Self::MaxOf(ak1, ak2, ak3) => vec![Key(ak1), Key(ak2), Key(ak3)],
        }
    }
}

impl ToFields for Statement {
    fn to_fields(self) -> (Vec<F>, usize) {
        let (native_statement_f, native_statement_f_len) = self.code().to_fields();
        let (vec_statementarg_f, vec_statementarg_f_len) = self
            .args()
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

impl fmt::Display for Statement {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?} ", self.code())?;
        for (i, arg) in self.args().iter().enumerate() {
            if i != 0 {
                write!(f, " ")?;
            }
            write!(f, "{}", arg)?;
        }
        Ok(())
    }
}

/// Statement argument type. Useful for statement decompositions.
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
    pub fn literal(&self) -> Result<Value> {
        match self {
            Self::Literal(value) => Ok(*value),
            _ => Err(anyhow!("Statement argument {:?} is not a literal.", self)),
        }
    }
    pub fn key(&self) -> Result<AnchoredKey> {
        match self {
            Self::Key(ak) => Ok(ak.clone()),
            _ => Err(anyhow!("Statement argument {:?} is not a key.", self)),
        }
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

// TODO: Refine this enum.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Operation {
    None,
    NewEntry,
    CopyStatement(Statement),
    EqualFromEntries(Statement, Statement),
    NotEqualFromEntries(Statement, Statement),
    GtFromEntries(Statement, Statement),
    LtFromEntries(Statement, Statement),
    TransitiveEqualFromStatements(Statement, Statement),
    GtToNotEqual(Statement),
    LtToNotEqual(Statement),
    ContainsFromEntries(Statement, Statement),
    NotContainsFromEntries(Statement, Statement),
    RenameContainedBy(Statement, Statement),
    SumOf(Statement, Statement, Statement),
    ProductOf(Statement, Statement, Statement),
    MaxOf(Statement, Statement, Statement),
}

impl Operation {
    pub fn code(&self) -> NativeOperation {
        use NativeOperation::*;
        match self {
            Self::None => None,
            Self::NewEntry => NewEntry,
            Self::CopyStatement(_) => CopyStatement,
            Self::EqualFromEntries(_, _) => EqualFromEntries,
            Self::NotEqualFromEntries(_, _) => NotEqualFromEntries,
            Self::GtFromEntries(_, _) => GtFromEntries,
            Self::LtFromEntries(_, _) => LtFromEntries,
            Self::TransitiveEqualFromStatements(_, _) => TransitiveEqualFromStatements,
            Self::GtToNotEqual(_) => GtToNotEqual,
            Self::LtToNotEqual(_) => LtToNotEqual,
            Self::ContainsFromEntries(_, _) => ContainsFromEntries,
            Self::NotContainsFromEntries(_, _) => NotContainsFromEntries,
            Self::RenameContainedBy(_, _) => RenameContainedBy,
            Self::SumOf(_, _, _) => SumOf,
            Self::ProductOf(_, _, _) => ProductOf,
            Self::MaxOf(_, _, _) => MaxOf,
        }
    }

    pub fn args(&self) -> Vec<Statement> {
        match self.clone() {
            Self::None => vec![],
            Self::NewEntry => vec![],
            Self::CopyStatement(s) => vec![s],
            Self::EqualFromEntries(s1, s2) => vec![s1, s2],
            Self::NotEqualFromEntries(s1, s2) => vec![s1, s2],
            Self::GtFromEntries(s1, s2) => vec![s1, s2],
            Self::LtFromEntries(s1, s2) => vec![s1, s2],
            Self::TransitiveEqualFromStatements(s1, s2) => vec![s1, s2],
            Self::GtToNotEqual(s) => vec![s],
            Self::LtToNotEqual(s) => vec![s],
            Self::ContainsFromEntries(s1, s2) => vec![s1, s2],
            Self::NotContainsFromEntries(s1, s2) => vec![s1, s2],
            Self::RenameContainedBy(s1, s2) => vec![s1, s2],
            Self::SumOf(s1, s2, s3) => vec![s1, s2, s3],
            Self::ProductOf(s1, s2, s3) => vec![s1, s2, s3],
            Self::MaxOf(s1, s2, s3) => vec![s1, s2, s3],
        }
    }
    /// Forms operation from op-code and arguments.
    pub fn op(op_code: NativeOperation, args: &[Statement]) -> Result<Self> {
        type NO = NativeOperation;
        let arg_tup = (
            args.get(0).cloned(),
            args.get(1).cloned(),
            args.get(2).cloned(),
        );
        Ok(match (op_code, arg_tup, args.len()) {
            (NO::None, (None, None, None), 0) => Self::None,
            (NO::NewEntry, (None, None, None), 0) => Self::NewEntry,
            (NO::CopyStatement, (Some(s), None, None), 1) => Self::CopyStatement(s),
            (NO::EqualFromEntries, (Some(s1), Some(s2), None), 2) => Self::EqualFromEntries(s1, s2),
            (NO::NotEqualFromEntries, (Some(s1), Some(s2), None), 2) => {
                Self::NotEqualFromEntries(s1, s2)
            }
            (NO::GtFromEntries, (Some(s1), Some(s2), None), 2) => Self::GtFromEntries(s1, s2),
            (NO::LtFromEntries, (Some(s1), Some(s2), None), 2) => Self::LtFromEntries(s1, s2),
            (NO::ContainsFromEntries, (Some(s1), Some(s2), None), 2) => {
                Self::ContainsFromEntries(s1, s2)
            }
            (NO::NotContainsFromEntries, (Some(s1), Some(s2), None), 2) => {
                Self::NotContainsFromEntries(s1, s2)
            }
            (NO::RenameContainedBy, (Some(s1), Some(s2), None), 2) => {
                Self::RenameContainedBy(s1, s2)
            }
            (NO::SumOf, (Some(s1), Some(s2), Some(s3)), 3) => Self::SumOf(s1, s2, s3),
            (NO::ProductOf, (Some(s1), Some(s2), Some(s3)), 3) => Self::ProductOf(s1, s2, s3),
            (NO::MaxOf, (Some(s1), Some(s2), Some(s3)), 3) => Self::MaxOf(s1, s2, s3),
            _ => Err(anyhow!(
                "Ill-formed operation {:?} with arguments {:?}.",
                op_code,
                args
            ))?,
        })
    }
    /// Checks the given operation against a statement.
    pub fn check(&self, output_statement: &Statement) -> Result<bool> {
        use Statement::*;
        match (self, output_statement) {
            (Self::None, None) => Ok(true),
            (Self::NewEntry, ValueOf(AnchoredKey(pod_id, _), _)) => Ok(pod_id == &SELF),
            (Self::CopyStatement(s1), s2) => Ok(s1 == s2),
            (Self::EqualFromEntries(ValueOf(ak1, v1), ValueOf(ak2, v2)), Equal(ak3, ak4)) => {
                Ok(v1 == v2 && ak3 == ak1 && ak4 == ak2)
            }
            (Self::NotEqualFromEntries(ValueOf(ak1, v1), ValueOf(ak2, v2)), NotEqual(ak3, ak4)) => {
                Ok(v1 != v2 && ak3 == ak1 && ak4 == ak2)
            }
            (Self::GtFromEntries(ValueOf(ak1, v1), ValueOf(ak2, v2)), Gt(ak3, ak4)) => {
                Ok(v1 > v2 && ak3 == ak1 && ak4 == ak2)
            }
            (Self::LtFromEntries(ValueOf(ak1, v1), ValueOf(ak2, v2)), Lt(ak3, ak4)) => {
                Ok(v1 < v2 && ak3 == ak1 && ak4 == ak2)
            }
            (Self::ContainsFromEntries(_, _), Contains(_, _)) =>
            /* TODO */
            {
                Ok(true)
            }
            (Self::NotContainsFromEntries(_, _), NotContains(_, _)) =>
            /* TODO */
            {
                Ok(true)
            }
            (
                Self::TransitiveEqualFromStatements(Equal(ak1, ak2), Equal(ak3, ak4)),
                Equal(ak5, ak6),
            ) => Ok(ak2 == ak3 && ak5 == ak1 && ak6 == ak4),
            (Self::GtToNotEqual(Gt(ak1, ak2)), NotEqual(ak3, ak4)) => Ok(ak1 == ak3 && ak2 == ak4),
            (Self::LtToNotEqual(Lt(ak1, ak2)), NotEqual(ak3, ak4)) => Ok(ak1 == ak3 && ak2 == ak4),
            (Self::RenameContainedBy(Contains(ak1, ak2), Equal(ak3, ak4)), Contains(ak5, ak6)) => {
                Ok(ak1 == ak3 && ak4 == ak5 && ak2 == ak6)
            }
            (
                Self::SumOf(ValueOf(ak1, v1), ValueOf(ak2, v2), ValueOf(ak3, v3)),
                SumOf(ak4, ak5, ak6),
            ) => {
                let v1: i64 = v1.clone().try_into()?;
                let v2: i64 = v2.clone().try_into()?;
                let v3: i64 = v3.clone().try_into()?;
                Ok((v1 == v2 + v3) && ak4 == ak1 && ak5 == ak2 && ak6 == ak3)
            }
            _ => Err(anyhow!(
                "Invalid deduction: {:?} ⇏ {:#}",
                self,
                output_statement
            )),
        }
    }
}

pub trait Pod: fmt::Debug + DynClone {
    fn verify(&self) -> bool;
    fn id(&self) -> PodId;
    fn pub_statements(&self) -> Vec<Statement>;
    /// Extract key-values from ValueOf public statements
    fn kvs(&self) -> HashMap<AnchoredKey, Value> {
        self.pub_statements()
            .into_iter()
            .filter_map(|st| match st.code() {
                NativeStatement::ValueOf => {
                    let args = st.args();
                    Some((
                        args[0].key().expect("key"),
                        args[1].literal().expect("literal"),
                    ))
                }
                _ => None,
            })
            .collect()
    }
    // Used for downcasting
    fn into_any(self: Box<Self>) -> Box<dyn Any>;
}

// impl Clone for Box<dyn SignedPod>
dyn_clone::clone_trait_object!(Pod);

pub trait PodSigner {
    fn sign(&mut self, params: &Params, kvs: &HashMap<Hash, Value>) -> Result<Box<dyn Pod>>;
}

/// This is a filler type that fulfills the Pod trait and always verifies.  It's empty.  This
/// can be used to simulate padding in a circuit.
#[derive(Debug, Clone)]
pub struct NonePod {}

impl Pod for NonePod {
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
    pub signed_pods: &'a [&'a Box<dyn Pod>],
    pub main_pods: &'a [&'a Box<dyn Pod>],
    pub statements: &'a [Statement],
    pub operations: &'a [Operation],
    /// Statements that need to be made public (they can come from input pods or input
    /// statements)
    pub public_statements: &'a [Statement],
}

pub trait PodProver {
    fn prove(&mut self, params: &Params, inputs: MainPodInputs) -> Result<Box<dyn Pod>>;
}

pub trait ToFields {
    /// returns Vec<F> representation of the type, and a usize indicating how many field elements
    /// does the vector contain
    fn to_fields(self) -> (Vec<F>, usize);
}
