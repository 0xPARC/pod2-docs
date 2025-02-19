//! The frontend includes the user-level abstractions and user-friendly types to define and work
//! with Pods.

use anyhow::{anyhow, Error, Result};
use itertools::Itertools;
use std::collections::HashMap;
use std::convert::From;
use std::fmt;

use crate::middleware::{
    self,
    containers::{Array, Dictionary, Set},
    hash_str, Hash, MainPodInputs, NativeOperation, NativeStatement, Params, PodId, PodProver,
    PodSigner, SELF,
};

/// This type is just for presentation purposes.
#[derive(Clone, Debug, Default, Hash, PartialEq, Eq)]
pub enum PodClass {
    #[default]
    Signed,
    Main,
}

// An Origin, which represents a reference to an ancestor POD.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Default)]
pub struct Origin(pub PodClass, pub PodId);

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Value {
    String(String),
    Int(i64),
    Bool(bool),
    Dictionary(Dictionary),
    Set(Set),
    Array(Array),
    Raw(middleware::Value),
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

impl From<bool> for Value {
    fn from(b: bool) -> Self {
        Value::Bool(b)
    }
}

impl From<&Value> for middleware::Value {
    fn from(v: &Value) -> Self {
        match v {
            Value::String(s) => middleware::Value(hash_str(s).0),
            Value::Int(v) => middleware::Value::from(*v),
            Value::Bool(b) => middleware::Value::from(*b as i64),
            Value::Dictionary(d) => middleware::Value(d.commitment().0),
            Value::Set(s) => middleware::Value(s.commitment().0),
            Value::Array(a) => middleware::Value(a.commitment().0),
            Value::Raw(v) => v.clone(),
        }
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::String(s) => write!(f, "\"{}\"", s),
            Value::Int(v) => write!(f, "{}", v),
            Value::Bool(b) => write!(f, "{}", b),
            Value::Dictionary(d) => write!(f, "dict:{}", d.commitment()),
            Value::Set(s) => write!(f, "set:{}", s.commitment()),
        Value::Array(a) => write!(f, "arr:{}", a.commitment()),
            Value::Raw(v) => write!(f, "{}", v),
        }
    }
}

#[derive(Clone, Debug)]
pub struct SignedPodBuilder {
    pub params: Params,
    pub kvs: HashMap<String, Value>,
}

impl SignedPodBuilder {
    pub fn new(params: &Params) -> Self {
        Self {
            params: params.clone(),
            kvs: HashMap::new(),
        }
    }

    pub fn insert(&mut self, key: impl Into<String>, value: impl Into<Value>) {
        self.kvs.insert(key.into(), value.into());
    }

    pub fn sign<S: PodSigner>(&self, signer: &mut S) -> Result<SignedPod> {
        let mut kvs = HashMap::new();
        let mut key_string_map = HashMap::new();
        for (k, v) in self.kvs.iter() {
            let k_hash = hash_str(k);
            kvs.insert(k_hash, middleware::Value::from(v));
            key_string_map.insert(k_hash, k.clone());
        }
        let pod = signer.sign(&self.params, &kvs)?;
        Ok(SignedPod {
            pod,
            key_string_map,
        })
    }
}

/// SignedPod is a wrapper on top of backend::SignedPod, which additionally stores the
/// string<-->hash relation of the keys.
#[derive(Debug, Clone)]
pub struct SignedPod {
    pub pod: Box<dyn middleware::Pod>,
    /// HashMap to store the reverse relation between key strings and key hashes
    pub key_string_map: HashMap<Hash, String>,
    // TODO: Similar map for hash.
}

impl fmt::Display for SignedPod {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "SignedPod (id:{}):", self.id())?;
        // Note: current version iterates sorting by keys of the kvs, but the merkletree defined at
        // https://0xparc.github.io/pod2/merkletree.html will not need it since it will be
        // deterministic based on the keys values not on the order of the keys when added into the
        // tree.
        for (k, v) in self.kvs().iter().sorted_by_key(|kv| kv.0) {
            writeln!(f, "  - {}: {}", k, v)?;
        }
        Ok(())
    }
}

impl SignedPod {
    pub fn id(&self) -> PodId {
        self.pod.id()
    }
    pub fn origin(&self) -> Origin {
        Origin(PodClass::Signed, self.id())
    }
    pub fn verify(&self) -> bool {
        self.pod.verify()
    }
    pub fn kvs(&self) -> HashMap<Hash, middleware::Value> {
        self.pod
            .kvs()
            .into_iter()
            .map(|(middleware::AnchoredKey(_, k), v)| (k, v))
            .collect()
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct AnchoredKey(pub Origin, pub String);

impl Into<middleware::AnchoredKey> for AnchoredKey {
    fn into(self) -> middleware::AnchoredKey {
        middleware::AnchoredKey(self.0 .1, hash_str(&self.1))
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StatementArg {
    Literal(Value),
    Key(AnchoredKey),
}

impl fmt::Display for StatementArg {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Literal(v) => write!(f, "{}", v),
            Self::Key(r) => write!(f, "{}.{}", r.0 .1, r.1),
        }
    }
}

// TODO: Incorporate custom statements into this enum.
/// Type encapsulating statements with their associated arguments.
#[derive(Clone, Debug, PartialEq, Eq)]
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

impl Into<middleware::Statement> for Statement {
    fn into(self) -> middleware::Statement {
        use middleware::Statement::*;
        match self {
            Self::None => None,
            Self::ValueOf(ak, v) => ValueOf(ak.into(), (&v).into()),
            Self::Equal(ak1, ak2) => Equal(ak1.into(), ak2.into()),
            Self::NotEqual(ak1, ak2) => NotEqual(ak1.into(), ak2.into()),
            Self::Gt(ak1, ak2) => Gt(ak1.into(), ak2.into()),
            Self::Lt(ak1, ak2) => Lt(ak1.into(), ak2.into()),
            Self::Contains(ak1, ak2) => Contains(ak1.into(), ak2.into()),
            Self::NotContains(ak1, ak2) => NotContains(ak1.into(), ak2.into()),
            Self::SumOf(ak1, ak2, ak3) => SumOf(ak1.into(), ak2.into(), ak3.into()),
            Self::ProductOf(ak1, ak2, ak3) => ProductOf(ak1.into(), ak2.into(), ak3.into()),
            Self::MaxOf(ak1, ak2, ak3) => MaxOf(ak1.into(), ak2.into(), ak3.into()),
        }
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

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum OperationArg {
    Entry(String, Value),
    Statement(Statement),
    Literal(Value),
}

impl fmt::Display for OperationArg {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OperationArg::Entry(k, v) => write!(f, "{}: {}", k, v),
            OperationArg::Statement(s) => write!(f, "{}", s),
            OperationArg::Literal(v) => write!(f, "{}", v),
        }
    }
}

impl From<Value> for OperationArg {
    fn from(v: Value) -> Self {
        Self::Literal(v)
    }
}

impl From<&Value> for OperationArg {
    fn from(v: &Value) -> Self {
        Self::Literal(v.clone())
    }
}

impl From<&str> for OperationArg {
    fn from(s: &str) -> Self {
        Self::Literal(Value::from(s))
    }
}

impl From<i64> for OperationArg {
    fn from(v: i64) -> Self {
        Self::Literal(Value::from(v))
    }
}

impl From<bool> for OperationArg {
    fn from(b: bool) -> Self {
        Self::Literal(Value::from(b))
    }
}

impl From<(Origin, &str)> for OperationArg {
    fn from((origin, key): (Origin, &str)) -> Self {
        Self::Key(AnchoredKey(origin, key.to_string()))
    }
}

impl TryFrom<(&SignedPod, &str)> for OperationArg {
    type Error = Error;
    fn try_from((pod, key): (&SignedPod, &str)) -> Result<Self> {
        let value = pod.kvs().get(&hash_str(key)).cloned().ok_or(anyhow!(
            "POD with ID {} does not contain value corresponding to key {}.",
            pod.id(),
            key
        ))?;
        Ok(Self::Statement(Statement::ValueOf(
            AnchoredKey(pod.origin(), key.to_string()),
            Value::Raw(value),
        )))
    }
}

// TODO: Refine this enum.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Operation {
    None,
    NewEntry(String, Value),
    CopyStatement(Statement),
    EqualFromEntries(OperationArg, OperationArg),
    NotEqualFromEntries(OperationArg, OperationArg),
    GtFromEntries(OperationArg, OperationArg),
    LtFromEntries(OperationArg, OperationArg),
    TransitiveEqualFromStatements(Statement, Statement),
    GtToNotEqual(Statement),
    LtToNotEqual(Statement),
    ContainsFromEntries(OperationArg, OperationArg),
    NotContainsFromEntries(OperationArg, OperationArg),
    RenameContainedBy(Statement, Statement),
    SumOf(OperationArg, OperationArg, OperationArg),
    ProductOf(OperationArg, OperationArg, OperationArg),
    MaxOf(OperationArg, OperationArg, OperationArg),
}

impl Operation {
    pub fn code(&self) -> NativeOperation {
        use NativeOperation::*;
        match self {
            Self::None => None,
            Self::NewEntry(_, _) => NewEntry,
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
    pub fn args(&self) -> Vec<OperationArg> {
        use OperationArg::*;
        match self.clone() {
            Self::None => vec![],
            Self::NewEntry(key, value) => vec![Entry(key, value)],
            Self::CopyStatement(s) => vec![Statement(s.into())],
            Self::EqualFromEntries(s1, s2) => vec![s1, s2],
            Self::NotEqualFromEntries(s1, s2) => vec![s1, s2],
            Self::GtFromEntries(s1, s2) => vec![s1, s2],
            Self::LtFromEntries(s1, s2) => vec![s1, s2],
            Self::TransitiveEqualFromStatements(s1, s2) => {
                vec![Statement(s1.into()), Statement(s2.into())]
            }
            Self::GtToNotEqual(s) => vec![Statement(s.into())],
            Self::LtToNotEqual(s) => vec![Statement(s.into())],
            Self::ContainsFromEntries(s1, s2) => vec![s1, s2],
            Self::NotContainsFromEntries(s1, s2) => vec![s1, s2],
            Self::RenameContainedBy(s1, s2) => vec![Statement(s1.into()), Statement(s2.into())],
            Self::SumOf(s1, s2, s3) => vec![s1, s2, s3],
            Self::ProductOf(s1, s2, s3) => vec![s1, s2, s3],
            Self::MaxOf(s1, s2, s3) => vec![s1, s2, s3],
        }
    }
}

impl fmt::Display for Operation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
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

#[derive(Debug)]
pub struct MainPodBuilder {
    pub params: Params,
    pub input_signed_pods: Vec<SignedPod>,
    pub input_main_pods: Vec<MainPod>,
    pub statements: Vec<Statement>,
    pub operations: Vec<Operation>,
    pub public_statements: Vec<Statement>,
    // Internal state
    const_cnt: usize,
}

impl fmt::Display for MainPodBuilder {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "MainPod:")?;
        writeln!(f, "  input_signed_pods:")?;
        for in_pod in &self.input_signed_pods {
            writeln!(f, "    - {}", in_pod.id())?;
        }
        writeln!(f, "  input_main_pods:")?;
        for in_pod in &self.input_main_pods {
            writeln!(f, "    - {}", in_pod.id())?;
        }
        writeln!(f, "  statements:")?;
        for (st, op) in self.statements.iter().zip_eq(self.operations.iter()) {
            write!(f, "    - {} <- ", st)?;
            write!(f, "{}", op)?;
            write!(f, "\n")?;
        }
        Ok(())
    }
}

impl MainPodBuilder {
    pub fn new(params: &Params) -> Self {
        Self {
            params: params.clone(),
            input_signed_pods: Vec::new(),
            input_main_pods: Vec::new(),
            statements: Vec::new(),
            operations: Vec::new(),
            public_statements: Vec::new(),
            const_cnt: 0,
        }
    }
    pub fn add_signed_pod(&mut self, pod: &SignedPod) {
        self.input_signed_pods.push(pod.clone());
    }
    pub fn add_main_pod(&mut self, pod: MainPod) {
        self.input_main_pods.push(pod);
    }
    pub fn insert(&mut self, st_op: (Statement, Operation)) {
        let (st, op) = st_op;
        self.statements.push(st);
        self.operations.push(op);
    }

    pub fn pub_op(&mut self, op: Operation) -> Statement {
        self.op(true, op)
    }

    /// Convert [OperationArg]s to [StatementArg]s for the operations that work with entries
    fn op_args_entries(&mut self, public: bool, args: &mut [OperationArg]) -> Vec<AnchoredKey> {
        let mut st_args = Vec::new();
        for arg in args.iter_mut() {
            match arg {
                OperationArg::Statement(s) => match s {
                    Statement::ValueOf(k, _) => st_args.push(k.clone()),
                    _ => panic!("Invalid statement argument."),
                },
                OperationArg::Literal(v) => {
                    let k = format!("c{}", self.const_cnt);
                    self.const_cnt += 1;
                    let value_of_st = self.op(public, Operation::NewEntry(k.clone(), v.clone()));
                    *arg = OperationArg::Statement(Statement::ValueOf(
                        AnchoredKey(Origin(PodClass::Main, SELF), k.clone()),
                        v.clone(),
                    ));
                    if let StatementArg::Key(k) = value_of_st.args()[0].clone() {
                        st_args.push(k);
                    } else {
                        unreachable!("Unexpected missing anchored key argument!");
                    }
                }
                // TODO: Remove!
                _ => {
                    unreachable!("Argument to operation on entries cannot be of type Entry!")
                }
            };
        }
        st_args
    }

    pub fn op(&mut self, public: bool, op: Operation) -> Statement {
        type O = Operation;
        use Statement::*;
        let mut args = op.args();
        // TODO: argument type checking
        let st = match &op {
            O::None => None,
            O::NewEntry(key, value) => ValueOf(
                AnchoredKey(Origin(PodClass::Main, SELF), key.clone()),
                value.clone(),
            ),
            O::CopyStatement(s) => s.clone(),
            O::EqualFromEntries(_, _) => {
                let args = self.op_args_entries(public, &mut args);
                Equal(args[0].clone(), args[1].clone())
            }
            O::NotEqualFromEntries(_, _) => {
                let args = self.op_args_entries(public, &mut args);
                NotEqual(args[0].clone(), args[1].clone())
            }
            O::GtFromEntries(_, _) => {
                let args = self.op_args_entries(public, &mut args);
                Gt(args[0].clone(), args[1].clone())
            }
            O::LtFromEntries(_, _) => {
                let args = self.op_args_entries(public, &mut args);
                Lt(args[0].clone(), args[1].clone())
            }
            O::TransitiveEqualFromStatements(_, _) => todo!(),
            O::GtToNotEqual(_) => todo!(),
            O::LtToNotEqual(_) => todo!(),
            O::ContainsFromEntries(_, _) => {
                let args = self.op_args_entries(public, &mut args);
                Contains(args[0].clone(), args[1].clone())
            }
            O::NotContainsFromEntries(_, _) => {
                let args = self.op_args_entries(public, &mut args);
                NotContains(args[0].clone(), args[1].clone())
            }
            O::RenameContainedBy(_, _) => todo!(),
            O::SumOf(_, _, _) => todo!(),
            O::ProductOf(_, _, _) => todo!(),
            O::MaxOf(_, _, _) => todo!(),
        };
        self.operations.push(op);
        if public {
            self.public_statements.push(st.clone());
        }
        self.statements.push(st);
        self.statements[self.statements.len() - 1].clone()
    }

    pub fn reveal(&mut self, st: &Statement) {
        self.public_statements.push(st.clone());
    }

    pub fn prove<P: PodProver>(&self, prover: &mut P) -> Result<MainPod> {
        let compiler = MainPodCompiler::new(&self.params);
        let inputs = MainPodCompilerInputs {
            // signed_pods: &self.input_signed_pods,
            // main_pods: &self.input_main_pods,
            statements: &self.statements,
            operations: &self.operations,
            public_statements: &self.public_statements,
        };
        let (statements, operations, public_statements) = compiler.compile(inputs)?;

        let inputs = MainPodInputs {
            signed_pods: &self.input_signed_pods.iter().map(|p| &p.pod).collect_vec(),
            main_pods: &self.input_main_pods.iter().map(|p| &p.pod).collect_vec(),
            statements: &statements,
            operations: &operations,
            public_statements: &public_statements,
        };
        let pod = prover.prove(&self.params, inputs)?;
        Ok(MainPod { pod })
    }
}

#[derive(Debug)]
pub struct MainPod {
    pub pod: Box<dyn middleware::Pod>,
    // TODO: metadata
}

impl MainPod {
    pub fn id(&self) -> PodId {
        self.pod.id()
    }
    pub fn origin(&self) -> Origin {
        Origin(PodClass::Main, self.id())
    }
}

struct MainPodCompilerInputs<'a> {
    // pub signed_pods: &'a [Box<dyn middleware::SignedPod>],
    // pub main_pods: &'a [Box<dyn middleware::MainPod>],
    pub statements: &'a [Statement],
    pub operations: &'a [Operation],
    pub public_statements: &'a [Statement],
}

struct MainPodCompiler {
    params: Params,
    // Output
    statements: Vec<middleware::Statement>,
    operations: Vec<middleware::Operation>,
}

impl MainPodCompiler {
    fn new(params: &Params) -> Self {
        Self {
            params: params.clone(),
            statements: Vec::new(),
            operations: Vec::new(),
        }
    }

    fn push_st_op(&mut self, st: middleware::Statement, op: middleware::Operation) {
        self.statements.push(st);
        self.operations.push(op);
    }

    fn compile_anchored_key(key: &AnchoredKey) -> middleware::AnchoredKey {
        middleware::AnchoredKey(key.0 .1, hash_str(&key.1))
    }

    fn compile_st(&self, st: &Statement) -> middleware::Statement {
        use middleware::Statement::*;
        type S = Statement;
        match st {
            S::None => None,
            S::ValueOf(ak, v) => ValueOf(ak.clone().into(), v.into()),
            S::Equal(ak1, ak2) => Equal(ak1.clone().into(), ak2.clone().into()),
            S::NotEqual(ak1, ak2) => NotEqual(ak1.clone().into(), ak2.clone().into()),
            S::Gt(ak1, ak2) => Gt(ak1.clone().into(), ak2.clone().into()),
            S::Lt(ak1, ak2) => Lt(ak1.clone().into(), ak2.clone().into()),
            S::Contains(ak1, ak2) => Contains(ak1.clone().into(), ak2.clone().into()),
            S::NotContains(ak1, ak2) => NotContains(ak1.clone().into(), ak2.clone().into()),
            S::SumOf(ak1, ak2, ak3) => {
                SumOf(ak1.clone().into(), ak2.clone().into(), ak3.clone().into())
            }
            S::ProductOf(ak1, ak2, ak3) => {
                ProductOf(ak1.clone().into(), ak2.clone().into(), ak3.clone().into())
            }
            S::MaxOf(ak1, ak2, ak3) => {
                MaxOf(ak1.clone().into(), ak2.clone().into(), ak3.clone().into())
            }
        }
    }

    fn compile_op(&self, op: &Operation) -> middleware::Operation {
        use middleware::Operation::*;
        type O = Operation;
        type Arg = OperationArg;
        match op {
            O::None => None,
            // OperationArg::Entry is only used in the frontend.  The (key, value) will only
            // appear in the ValueOf statement in the backend.  This is because a new ValueOf
            // statement doesn't have any requirement on the key and value.
            O::NewEntry(_, _) => NewEntry,
            O::CopyStatement(s) => CopyStatement(self.compile_st(s)),
            O::EqualFromEntries(Arg::Statement(s1), Arg::Statement(s2)) => {
                EqualFromEntries(self.compile_st(s1), self.compile_st(s2))
            }
            O::NotEqualFromEntries(Arg::Statement(s1), Arg::Statement(s2)) => {
                NotEqualFromEntries(self.compile_st(s1), self.compile_st(s2))
            }
            O::GtFromEntries(Arg::Statement(s1), Arg::Statement(s2)) => {
                GtFromEntries(self.compile_st(s1), self.compile_st(s2))
            }
            O::LtFromEntries(Arg::Statement(s1), Arg::Statement(s2)) => {
                LtFromEntries(self.compile_st(s1), self.compile_st(s2))
            }
            O::TransitiveEqualFromStatements(s1, s2) => {
                TransitiveEqualFromStatements(self.compile_st(s1), self.compile_st(s2))
            }
            O::GtToNotEqual(s) => GtToNotEqual(self.compile_st(s)),
            O::LtToNotEqual(s) => LtToNotEqual(self.compile_st(s)),
            O::ContainsFromEntries(Arg::Statement(s1), Arg::Statement(s2)) => {
                ContainsFromEntries(self.compile_st(s1), self.compile_st(s2))
            }
            O::NotContainsFromEntries(Arg::Statement(s1), Arg::Statement(s2)) => {
                NotContainsFromEntries(self.compile_st(s1), self.compile_st(s2))
            }
            O::RenameContainedBy(s1, s2) => {
                RenameContainedBy(self.compile_st(s1), self.compile_st(s2))
            }
            O::SumOf(Arg::Statement(s1), Arg::Statement(s2), Arg::Statement(s3)) => SumOf(
                self.compile_st(s1),
                self.compile_st(s2),
                self.compile_st(s3),
            ),
            O::ProductOf(Arg::Statement(s1), Arg::Statement(s2), Arg::Statement(s3)) => ProductOf(
                self.compile_st(s1),
                self.compile_st(s2),
                self.compile_st(s3),
            ),
            O::MaxOf(Arg::Statement(s1), Arg::Statement(s2), Arg::Statement(s3)) => MaxOf(
                self.compile_st(s1),
                self.compile_st(s2),
                self.compile_st(s3),
            ),
            _ => panic!("Ill-formed operation: {:?}", op),
        }
    }

    fn compile_st_op(&mut self, st: &Statement, op: &Operation) {
        let middle_st = self.compile_st(st);
        let middle_op = self.compile_op(op);
        self.push_st_op(middle_st, middle_op);
    }

    pub fn compile<'a>(
        mut self,
        inputs: MainPodCompilerInputs<'a>,
    ) -> Result<(
        Vec<middleware::Statement>, // input statements
        Vec<middleware::Operation>,
        Vec<middleware::Statement>, // public statements
    )> {
        let MainPodCompilerInputs {
            // signed_pods: _,
            // main_pods: _,
            statements,
            operations,
            public_statements,
        } = inputs;
        for (st, op) in statements.iter().zip_eq(operations.iter()) {
            self.compile_st_op(st, op);
            if self.statements.len() > self.params.max_statements {
                panic!("too many statements");
            }
        }
        let public_statements = public_statements
            .iter()
            .map(|st| self.compile_st(st))
            .collect_vec();
        Ok((self.statements, self.operations, public_statements))
    }
}

// TODO fn fmt_signed_pod_builder
// TODO fn fmt_main_pod

#[macro_use]
pub mod build_utils {
    #[macro_export]
    macro_rules! op_args {
        ($($arg:expr),+) => {vec![$(crate::frontend::OperationArg::from($arg)),*]}
    }

    #[macro_export]
    macro_rules! op {
        (eq, $($arg:expr),+) => { crate::frontend::Operation(
            crate::middleware::NativeOperation::EqualFromEntries,
            crate::op_args!($($arg),*)) };
        (ne, $($arg:expr),+) => { crate::frontend::Operation(
            crate::middleware::NativeOperation::NotEqualFromEntries,
            crate::op_args!($($arg),*)) };
        (gt, $($arg:expr),+) => { crate::frontend::Operation(
            crate::middleware::NativeOperation::GtFromEntries,
            crate::op_args!($($arg),*)) };
        (lt, $($arg:expr),+) => { crate::frontend::Operation(
            crate::middleware::NativeOperation::LtFromEntries,
            crate::op_args!($($arg),*)) };
        (contains, $($arg:expr),+) => { crate::frontend::Operation(
            crate::middleware::NativeOperation::ContainsFromEntries,
            crate::op_args!($($arg),*)) };
        (not_contains, $($arg:expr),+) => { crate::frontend::Operation(
            crate::middleware::NativeOperation::NotContainsFromEntries,
            crate::op_args!($($arg),*)) };
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use crate::backends::mock_main::MockProver;
    use crate::backends::mock_signed::MockSigner;
    use crate::examples::{great_boy_pod_full_flow, tickets_pod_full_flow, zu_kyc_pod_builder, zu_kyc_sign_pod_builders};

    #[test]
    fn test_front_zu_kyc() -> Result<()> {
        let params = Params::default();
        let (gov_id, pay_stub) = zu_kyc_sign_pod_builders(&params);

        // TODO: print pods from the builder

        let mut signer = MockSigner {
            pk: "ZooGov".into(),
        };
        let gov_id = gov_id.sign(&mut signer).unwrap();
        println!("{}", gov_id);

        let mut signer = MockSigner {
            pk: "ZooDeel".into(),
        };
        let pay_stub = pay_stub.sign(&mut signer).unwrap();
        println!("{}", pay_stub);

        let kyc = zu_kyc_pod_builder(&params, &gov_id, &pay_stub);
        println!("{}", kyc);

        // TODO: prove kyc with MockProver and print it

        Ok(())
    }

    #[test]
    fn test_front_great_boy() -> Result<()> {
        let great_boy = great_boy_pod_full_flow();
        println!("{}", great_boy);

        // TODO: prove kyc with MockProver and print it

        Ok(())
    }

    #[test]
    fn test_front_tickets() -> Result<()> {
        let builder = tickets_pod_full_flow();
        println!("{}", builder);

        Ok(())
    }
}
