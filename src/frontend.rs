use anyhow::Result;
use itertools::Itertools;
use plonky2::field::types::Field;
use std::collections::HashMap;
use std::convert::From;
use std::fmt;
use std::io::{self, Write};

use crate::backend;
use crate::{hash_str, Params, PodId, F, SELF};

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

impl From<&Value> for backend::Value {
    fn from(v: &Value) -> Self {
        match v {
            Value::String(s) => backend::Value(hash_str(s).0),
            Value::Int(v) => {
                backend::Value([F::from_canonical_u64(*v as u64), F::ZERO, F::ZERO, F::ZERO])
            }
            // TODO
            Value::MerkleTree(mt) => backend::Value([
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

/// SignedPod is a wrapper on top of backend::SignedPod, which additionally stores the
/// string<-->hash relation of the keys.
#[derive(Clone, Debug)]
pub struct SignedPod {
    pub pod: backend::SignedPod,
    // `string_key_map` is a hashmap to store the relation between key strings and key hashes
    // TODO review if maybe store it as <Hash, String>, so that when iterating we can get each
    // hash respective string
    string_key_map: HashMap<String, crate::Hash>,
}

impl SignedPod {
    pub fn new(params: &Params, kvs: HashMap<String, Value>) -> Result<Self> {
        let (hashed_kvs, string_key_map): (
            HashMap<crate::Hash, backend::Value>,
            HashMap<String, crate::Hash>,
        ) = kvs.iter().fold(
            (HashMap::new(), HashMap::new()),
            |(mut hashed_kvs, mut key_to_hash), (k, v)| {
                let h = hash_str(k);
                hashed_kvs.insert(h, backend::Value::from(v));
                key_to_hash.insert(k.clone(), h);
                (hashed_kvs, key_to_hash)
            },
        );
        let pod = backend::SignedPod::new(params, hashed_kvs)?;
        Ok(Self {
            pod,
            string_key_map,
        })
    }
    pub fn id(&self) -> PodId {
        self.pod.id
    }
    pub fn origin(&self) -> Origin {
        Origin(PodType::Signed, self.pod.id)
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

impl From<NativeStatement> for backend::NativeStatement {
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

#[derive(Clone, Debug)]
pub struct MainPodBuilder {
    pub params: Params,
    pub statements: Vec<Statement>,
    pub operations: Vec<Operation>,
}

impl MainPodBuilder {
    pub fn push_statement(&mut self, st: Statement, op: Operation) {
        self.statements.push(st);
        self.operations.push(op);
    }
    pub fn build(self) -> MainPod {
        MainPod {
            params: self.params,
            id: PodId::default(),      // TODO
            input_signed_pods: vec![], // TODO
            input_main_pods: vec![],   // TODO
            statements: self
                .statements
                .into_iter()
                .zip(self.operations.into_iter())
                .collect(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct MainPod {
    pub params: Params,
    pub id: PodId,
    pub input_signed_pods: Vec<SignedPod>,
    pub input_main_pods: Vec<MainPod>,
    pub statements: Vec<(Statement, Operation)>,
}

impl MainPod {
    pub fn origin(&self) -> Origin {
        Origin(PodType::Main, self.id)
    }

    pub fn compile(&self) -> Result<backend::MainPod> {
        MainPodCompiler::new(self).compile()
    }

    pub fn max_priv_statements(&self) -> usize {
        self.params.max_statements - self.params.max_public_statements
    }
}

struct MainPodCompiler<'a> {
    // Input
    pod: &'a MainPod,
    // Output
    statements: Vec<backend::Statement>,
    // Internal state
    const_cnt: usize,
}

impl<'a> MainPodCompiler<'a> {
    fn new(pod: &'a MainPod) -> Self {
        Self {
            pod,
            statements: Vec::new(),
            const_cnt: 0,
        }
    }

    fn compile_st(&mut self, st: &Statement) {
        let mut args = Vec::new();
        let Statement(front_typ, front_args) = st;
        for front_arg in front_args {
            let key = match front_arg {
                StatementArg::Literal(v) => {
                    let key = format!("_c{}", self.const_cnt);
                    let key_hash = hash_str(&key);
                    self.const_cnt += 1;
                    let value_of_args = vec![
                        backend::StatementArg::Ref(backend::AnchoredKey(SELF, key_hash)),
                        backend::StatementArg::Literal(backend::Value::from(v)),
                    ];
                    self.statements.push(backend::Statement(
                        backend::NativeStatement::ValueOf,
                        value_of_args,
                    ));
                    backend::AnchoredKey(SELF, key_hash)
                }
                StatementArg::Ref(k) => backend::AnchoredKey(k.0 .1, hash_str(&k.1)),
            };
            args.push(backend::StatementArg::Ref(key));
            if args.len() > self.pod.params.max_statement_args {
                panic!("too many statement args");
            }
        }
        self.statements.push(backend::Statement(
            backend::NativeStatement::from(*front_typ),
            args,
        ));
    }

    pub fn compile(mut self) -> Result<backend::MainPod> {
        let MainPod {
            statements,
            params,
            input_signed_pods,
            ..
        } = self.pod;
        for st in statements {
            self.compile_st(&st.0);
            if self.statements.len() > params.max_statements {
                panic!("too many statements");
            }
        }
        let input_signed_pods: Vec<backend::SignedPod> =
            input_signed_pods.iter().map(|p| p.pod.clone()).collect();
        backend::MainPod::new(params.clone(), input_signed_pods, vec![], self.statements)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NativeOperation {
    TransitiveEqualityFromStatements = 1,
    GtToNonequality = 2,
    LtToNonequality = 3,
    Auto = 1024,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum OperationArg {
    Statement(Statement),
    Key(AnchoredKey),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Operation(pub NativeOperation, pub Vec<OperationArg>);

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
        for (k, v) in pod.pod.kvs.iter().sorted_by_key(|kv| kv.0) {
            writeln!(w, "  - {}: {}", k, v)?;
        }
        Ok(())
    }

    pub fn fmt_main_pod(&self, w: &mut dyn Write, pod: &MainPod) -> io::Result<()> {
        writeln!(w, "MainPod (id:{}):", pod.id)?;
        writeln!(w, "  input_signed_pods:")?;
        for in_pod in &pod.input_signed_pods {
            writeln!(w, "    - {}", in_pod.id())?;
        }
        writeln!(w, "  input_main_pods:")?;
        for in_pod in &pod.input_main_pods {
            writeln!(w, "    - {}", in_pod.id)?;
        }
        writeln!(w, "  statements:")?;
        for st in &pod.statements {
            let (st, op) = st;
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
    use crate::Hash;
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
