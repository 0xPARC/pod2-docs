//! The frontend includes the user-level abstractions and user-friendly types to define and work
//! with Pods.

use anyhow::Result;
use itertools::Itertools;
use plonky2::field::types::Field;
use std::collections::HashMap;
use std::convert::From;
use std::fmt;
use std::io::{self, Write};

use crate::middleware::{
    self, hash_str, Hash, MainPodInputs, NativeOperation, Params, PodId, PodProver, PodSigner, F,
    SELF,
};

#[derive(Clone, Debug, Default, Hash, PartialEq, Eq)]
pub enum PodType {
    #[default]
    Signed = 1,
    Main,
}

// An Origin, which represents a reference to an ancestor POD.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Default)]
pub struct Origin(pub PodType, pub PodId);

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MerkleTree {
    pub root: u8, // TODO
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Value {
    String(String),
    Int(i64),
    MerkleTree(MerkleTree),
}

impl From<&str> for Value {
    fn from(s: &str) -> Self {
        Value::String(s.to_string())
    }
}

impl From<i64> for Value {
    fn from(v: i64) -> Self {
        Value::Int(v)
    }
}

impl From<&Value> for middleware::Value {
    fn from(v: &Value) -> Self {
        match v {
            Value::String(s) => middleware::Value(hash_str(s).0),
            Value::Int(v) => middleware::Value::from(*v),
            // TODO
            Value::MerkleTree(mt) => middleware::Value([
                F::from_canonical_u64(mt.root as u64),
                F::ZERO,
                F::ZERO,
                F::ZERO,
            ]),
        }
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::String(s) => write!(f, "\"{}\"", s),
            Value::Int(v) => write!(f, "{}", v),
            Value::MerkleTree(mt) => write!(f, "mt:{}", mt.root),
        }
    }
}

#[derive(Clone, Debug)]
pub struct SignedPodBuilder {
    pub params: Params,
    pub kvs: HashMap<String, Value>,
}

impl SignedPodBuilder {
    pub fn new(params: Params) -> Self {
        Self {
            params,
            kvs: HashMap::new(),
        }
    }

    pub fn insert(&mut self, key: impl Into<String>, value: impl Into<Value>) {
        self.kvs.insert(key.into(), value.into());
    }

    pub fn sign<S: PodSigner>(&self, signer: &mut S) -> SignedPod {
        let mut kvs = HashMap::new();
        let mut key_string_map = HashMap::new();
        for (k, v) in self.kvs.iter() {
            let k_hash = hash_str(k);
            kvs.insert(k_hash, middleware::Value::from(v));
            key_string_map.insert(k_hash, k.clone());
        }
        let pod = signer.sign(&self.params, &kvs);
        SignedPod {
            pod,
            key_string_map,
        }
    }
}

/// SignedPod is a wrapper on top of backend::SignedPod, which additionally stores the
/// string<-->hash relation of the keys.
#[derive(Debug)]
pub struct SignedPod {
    pub pod: Box<dyn middleware::SignedPod>,
    /// HashMap to store the reverse relation between key strings and key hashes
    pub key_string_map: HashMap<Hash, String>,
}

impl SignedPod {
    pub fn id(&self) -> PodId {
        self.pod.id()
    }
    pub fn origin(&self) -> Origin {
        Origin(PodType::Signed, self.id())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NativeStatement {
    Equal = 2,
    NotEqual,
    Gt,
    Lt,
    Contains,
    NotContains,
    SumOf,
    ProductOf,
    MaxOf,
}

impl From<NativeStatement> for middleware::NativeStatement {
    fn from(v: NativeStatement) -> Self {
        Self::from_repr(v as usize).unwrap()
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct AnchoredKey(pub Origin, pub String);

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StatementArg {
    Literal(Value),
    Ref(AnchoredKey),
}

impl fmt::Display for StatementArg {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Literal(v) => write!(f, "{}", v),
            Self::Ref(r) => write!(f, "{}.{}", r.0 .1, r.1),
        }
    }
}

impl From<Value> for StatementArg {
    fn from(v: Value) -> Self {
        StatementArg::Literal(v)
    }
}

impl From<&str> for StatementArg {
    fn from(s: &str) -> Self {
        StatementArg::Literal(Value::from(s))
    }
}

impl From<i64> for StatementArg {
    fn from(v: i64) -> Self {
        StatementArg::Literal(Value::from(v))
    }
}

impl From<(Origin, &str)> for StatementArg {
    fn from((origin, key): (Origin, &str)) -> Self {
        StatementArg::Ref(AnchoredKey(origin, key.to_string()))
    }
}

impl From<(&SignedPod, &str)> for StatementArg {
    fn from((pod, key): (&SignedPod, &str)) -> Self {
        StatementArg::Ref(AnchoredKey(pod.origin(), key.to_string()))
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Statement(pub NativeStatement, pub Vec<StatementArg>);

impl fmt::Display for Statement {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?} ", self.0)?;
        for (i, arg) in self.1.iter().enumerate() {
            if i != 0 {
                write!(f, " ")?;
            }
            write!(f, "{}", arg)?;
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum OperationArg {
    Statement(Statement),
    Key(AnchoredKey),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Operation(pub NativeOperation, pub Vec<OperationArg>);

#[derive(Debug)]
pub struct MainPodBuilder {
    pub params: Params,
    pub input_signed_pods: Vec<Box<dyn middleware::SignedPod>>,
    pub input_main_pods: Vec<Box<dyn middleware::MainPod>>,
    pub statements: Vec<Statement>,
    pub operations: Vec<Operation>,
}

impl MainPodBuilder {
    pub fn add_signed_pod(&mut self, pod: Box<dyn middleware::SignedPod>) {
        self.input_signed_pods.push(pod);
    }
    pub fn add_main_pod(&mut self, pod: Box<dyn middleware::MainPod>) {
        self.input_main_pods.push(pod);
    }
    pub fn insert(&mut self, st_op: (Statement, Operation)) {
        let (st, op) = st_op;
        self.statements.push(st);
        self.operations.push(op);
    }

    pub fn prove<P: PodProver>(&self, prover: &mut P) -> MainPod {
        let compiler = MainPodCompiler::new(&self.params);
        let inputs = MainPodCompilerInputs {
            statements: &self.statements,
            operations: &self.operations,
        };
        let (statements, operations) = compiler.compile(inputs).expect("TODO");

        let inputs = middleware::MainPodInputs {
            signed_pods: &self.input_signed_pods,
            main_pods: &self.input_main_pods,
            statements: &statements,
            operations: &operations,
        };
        let pod = prover.prove(&self.params, inputs);
        MainPod { pod }
    }
}

#[derive(Debug)]
pub struct MainPod {
    pub pod: Box<dyn middleware::MainPod>,
    // TODO: metadata
}

impl MainPod {
    pub fn id(&self) -> PodId {
        self.pod.id()
    }
    pub fn origin(&self) -> Origin {
        Origin(PodType::Signed, self.id())
    }
}

struct MainPodCompilerInputs<'a> {
    pub statements: &'a [Statement],
    pub operations: &'a [Operation],
}

struct MainPodCompiler {
    params: Params,
    // Internal state
    const_cnt: usize,
    front_operations: Vec<Operation>,
    // Output
    statements: Vec<middleware::Statement>,
    operations: Vec<middleware::Operation>,
}

impl MainPodCompiler {
    fn new(params: &Params) -> Self {
        Self {
            params: params.clone(),
            const_cnt: 0,
            front_operations: Vec::new(),
            statements: Vec::new(),
            operations: Vec::new(),
        }
    }

    fn max_priv_statements(&self) -> usize {
        self.params.max_statements - self.params.max_public_statements
    }

    fn push_st_front_op(&mut self, st: middleware::Statement, op: Operation) {
        self.statements.push(st);
        self.front_operations.push(op);
    }

    fn compile_st(&mut self, st: &Statement, op: &Operation) {
        let mut st_args = Vec::new();
        let Statement(front_st_typ, front_st_args) = st;
        for front_st_arg in front_st_args {
            let key = match front_st_arg {
                StatementArg::Literal(v) => {
                    let key = format!("_c{}", self.const_cnt);
                    let key_hash = hash_str(&key);
                    self.const_cnt += 1;
                    let value_of_args = vec![
                        middleware::StatementArg::Ref(middleware::AnchoredKey(SELF, key_hash)),
                        middleware::StatementArg::Literal(middleware::Value::from(v)),
                    ];
                    self.push_st_front_op(
                        middleware::Statement(middleware::NativeStatement::ValueOf, value_of_args),
                        Operation(middleware::NativeOperation::NewEntry, vec![]),
                    );
                    middleware::AnchoredKey(SELF, key_hash)
                }
                StatementArg::Ref(k) => middleware::AnchoredKey(k.0 .1, hash_str(&k.1)),
            };
            st_args.push(middleware::StatementArg::Ref(key));
            if st_args.len() > self.params.max_statement_args {
                panic!("too many statement st_args");
            }
        }

        self.push_st_front_op(
            middleware::Statement(middleware::NativeStatement::from(*front_st_typ), st_args),
            op.clone(),
        );
    }

    pub fn compile<'a>(
        mut self,
        inputs: MainPodCompilerInputs<'a>,
    ) -> Result<(Vec<middleware::Statement>, Vec<middleware::Operation>)> {
        let MainPodCompilerInputs {
            statements,
            operations,
        } = inputs;
        for (st, op) in statements.iter().zip_eq(operations.iter()) {
            self.compile_st(st, op);
            if self.statements.len() > self.params.max_statements {
                panic!("too many statements");
            }
        }
        Ok((self.statements, self.operations))
    }
}

pub struct Printer {}

impl Printer {
    pub fn fmt_op_arg(&self, w: &mut dyn Write, arg: &OperationArg) -> io::Result<()> {
        match arg {
            OperationArg::Statement(s) => write!(w, "{}", s),
            OperationArg::Key(r) => write!(w, "{}.{}", r.0 .1, r.1),
        }
    }

    pub fn fmt_op(&self, w: &mut dyn Write, op: &Operation) -> io::Result<()> {
        write!(w, "{:?} ", op.0)?;
        for (i, arg) in op.1.iter().enumerate() {
            if i != 0 {
                write!(w, " ")?;
            }
            self.fmt_op_arg(w, arg)?;
        }
        Ok(())
    }

    pub fn fmt_signed_pod(&self, w: &mut dyn Write, pod: &SignedPod) -> io::Result<()> {
        writeln!(w, "SignedPod (id:{}):", pod.id())?;
        // Note: current version iterates sorting by keys of the kvs, but the merkletree defined at
        // https://0xparc.github.io/pod2/merkletree.html will not need it since it will be
        // deterministic based on the keys values not on the order of the keys when added into the
        // tree.
        for (k, v) in pod.pod.kvs().iter().sorted_by_key(|kv| kv.0) {
            writeln!(w, "  - {}: {}", k, v)?;
        }
        Ok(())
    }

    pub fn fmt_main_pod_builder(&self, w: &mut dyn Write, pod: &MainPodBuilder) -> io::Result<()> {
        writeln!(w, "MainPod:")?;
        writeln!(w, "  input_signed_pods:")?;
        for in_pod in &pod.input_signed_pods {
            writeln!(w, "    - {}", in_pod.id())?;
        }
        writeln!(w, "  input_main_pods:")?;
        for in_pod in &pod.input_main_pods {
            writeln!(w, "    - {}", in_pod.id())?;
        }
        writeln!(w, "  statements:")?;
        for (st, op) in pod.statements.iter().zip_eq(pod.operations.iter()) {
            write!(w, "    - {} <- ", st)?;
            self.fmt_op(w, op)?;
            write!(w, "\n")?;
        }
        Ok(())
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use crate::middleware::Hash;
    use hex::FromHex;
    use std::io;

    fn pod_id(hex: &str) -> PodId {
        PodId(Hash::from_hex(hex).unwrap())
    }

    fn auto() -> Operation {
        Operation(NativeOperation::Auto, vec![])
    }

    macro_rules! args {
        ($($arg:expr),+) => {vec![$(StatementArg::from($arg)),*]}
    }

    macro_rules! st {
        (eq, $($arg:expr),+) => { Statement(NativeStatement::Equal, args!($($arg),*)) };
        (ne, $($arg:expr),+) => { Statement(NativeStatement::NotEqual, args!($($arg),*)) };
        (gt, $($arg:expr),+) => { Statement(NativeStatement::Gt, args!($($arg),*)) };
        (lt, $($arg:expr),+) => { Statement(NativeStatement::Lt, args!($($arg),*)) };
        (contains, $($arg:expr),+) => { Statement(NativeStatement::Contains, args!($($arg),*)) };
        (not_contains, $($arg:expr),+) => { Statement(NativeStatement::NotContains, args!($($arg),*)) };
        (sum_of, $($arg:expr),+) => { Statement(NativeStatement::SumOf, args!($($arg),*)) };
        (product_of, $($arg:expr),+) => { Statement(NativeStatement::product_of, args!($($arg),*)) };
        (max_of, $($arg:expr),+) => { Statement(NativeStatement::max_of, args!($($arg),*)) };
    }

    pub fn data_zu_kyc(params: Params) -> Result<(SignedPod, SignedPod, MainPod)> {
        let mut kvs = HashMap::new();
        kvs.insert("idNumber".into(), "4242424242".into());
        kvs.insert("dateOfBirth".into(), 1169909384.into());
        kvs.insert("socialSecurityNumber".into(), "G2121210".into());
        let gov_id = SignedPod::new(&params, kvs)?;

        let mut kvs = HashMap::new();
        kvs.insert("socialSecurityNumber".into(), "G2121210".into());
        kvs.insert("startDate".into(), 1706367566.into());
        let pay_stub = SignedPod::new(&params, kvs)?;

        let sanction_list = Value::MerkleTree(MerkleTree { root: 1 });
        let now_minus_18y: i64 = 1169909388;
        let now_minus_1y: i64 = 1706367566;
        let mut statements: Vec<(Statement, Operation)> = Vec::new();
        statements.push((
            st!(not_contains, sanction_list, (&gov_id, "idNumber")),
            auto(),
        ));
        statements.push((st!(lt, (&gov_id, "dateOfBirth"), now_minus_18y), auto()));
        statements.push((
            st!(
                eq,
                (&gov_id, "socialSecurityNumber"),
                (&pay_stub, "socialSecurityNumber")
            ),
            auto(),
        ));
        statements.push((st!(eq, (&pay_stub, "startDate"), now_minus_1y), auto()));
        let kyc = MainPod {
            params: params.clone(),
            id: pod_id("3300000000000000000000000000000000000000000000000000000000000000"),
            input_signed_pods: vec![gov_id.clone(), pay_stub.clone()],
            input_main_pods: vec![],
            statements,
        };

        Ok((gov_id, pay_stub, kyc))
    }

    #[test]
    fn test_front_0() -> Result<()> {
        let (gov_id, pay_stub, kyc) = data_zu_kyc(Params::default())?;

        let printer = Printer {};
        let mut w = io::stdout();
        printer.fmt_signed_pod(&mut w, &gov_id).unwrap();
        printer.fmt_signed_pod(&mut w, &pay_stub).unwrap();
        printer.fmt_main_pod(&mut w, &kyc).unwrap();

        Ok(())
    }
}
