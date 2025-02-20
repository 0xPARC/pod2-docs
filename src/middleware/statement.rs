use anyhow::{anyhow, Result};
use plonky2::field::types::Field;
use std::fmt;
use strum_macros::FromRepr;

use super::{hash_str, AnchoredKey, Hash, ToFields, Value, F};

pub const KEY_SIGNER: &str = "_signer";
pub const KEY_TYPE: &str = "_type";
pub const STATEMENT_ARG_F_LEN: usize = 8;

#[derive(Clone, Copy, Debug, FromRepr, PartialEq, Eq)]
pub enum NativePredicate {
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

impl ToFields for NativePredicate {
    fn to_fields(self) -> (Vec<F>, usize) {
        (vec![F::from_canonical_u64(self as u64)], 1)
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
    pub fn code(&self) -> NativePredicate {
        match self {
            Self::None => NativePredicate::None,
            Self::ValueOf(_, _) => NativePredicate::ValueOf,
            Self::Equal(_, _) => NativePredicate::Equal,
            Self::NotEqual(_, _) => NativePredicate::NotEqual,
            Self::Gt(_, _) => NativePredicate::Gt,
            Self::Lt(_, _) => NativePredicate::Lt,
            Self::Contains(_, _) => NativePredicate::Contains,
            Self::NotContains(_, _) => NativePredicate::NotContains,
            Self::SumOf(_, _, _) => NativePredicate::SumOf,
            Self::ProductOf(_, _, _) => NativePredicate::ProductOf,
            Self::MaxOf(_, _, _) => NativePredicate::MaxOf,
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

use std::sync::Arc;

// BEGIN Custom 1b

#[derive(Debug)]
pub enum HashOrWildcard {
    Hash(Hash),
    Wildcard(usize),
}

impl fmt::Display for HashOrWildcard {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Hash(h) => write!(f, "{}", h),
            Self::Wildcard(n) => write!(f, "*{}", n),
        }
    }
}

#[derive(Debug)]
pub enum StatementTmplArg {
    None,
    Literal(Value),
    Key(HashOrWildcard, HashOrWildcard),
}

impl fmt::Display for StatementTmplArg {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::None => write!(f, "none"),
            Self::Literal(v) => write!(f, "{}", v),
            Self::Key(pod_id, key) => write!(f, "({}, {})", pod_id, key),
        }
    }
}

// END

// BEGIN Custom 2

// pub enum StatementTmplArg {
//     None,
//     Literal(Value),
//     Wildcard(usize),
// }

// END

/// Statement Template for a Custom Predicate
#[derive(Debug)]
pub struct StatementTmpl(Predicate, Vec<StatementTmplArg>);

#[derive(Debug)]
pub struct CustomPredicate {
    /// true for "and", false for "or"
    pub conjunction: bool,
    pub statements: Vec<StatementTmpl>,
    pub args_len: usize,
    // TODO: Add private args length?
    // TODO: Add args type information?
}

impl fmt::Display for CustomPredicate {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "{}<", if self.conjunction { "and" } else { "or" })?;
        for st in &self.statements {
            write!(f, "  {}", st.0)?;
            for (i, arg) in st.1.iter().enumerate() {
                if i != 0 {
                    write!(f, ", ")?;
                }
                write!(f, "{}", arg)?;
            }
            writeln!(f, "),")?;
        }
        write!(f, ">(")?;
        for i in 0..self.args_len {
            if i != 0 {
                write!(f, ", ")?;
            }
            write!(f, "*{}", i)?;
        }
        writeln!(f, ")")?;
        Ok(())
    }
}

#[derive(Debug)]
pub struct CustomPredicateBatch {
    predicates: Vec<CustomPredicate>,
}

impl CustomPredicateBatch {
    pub fn hash(&self) -> Hash {
        // TODO
        hash_str(&format!("{:?}", self))
    }
}

#[derive(Clone, Debug)]
pub enum Predicate {
    Native(NativePredicate),
    BatchSelf(usize),
    Custom(Arc<CustomPredicateBatch>, usize),
}

impl From<NativePredicate> for Predicate {
    fn from(v: NativePredicate) -> Self {
        Self::Native(v)
    }
}

impl ToFields for Predicate {
    fn to_fields(self) -> (Vec<F>, usize) {
        todo!()
    }
}

impl fmt::Display for Predicate {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Native(p) => write!(f, "{:?}", p),
            Self::BatchSelf(i) => write!(f, "self.{}", i),
            Self::Custom(pb, i) => write!(f, "{}.{}", pb.hash(), i),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::middleware::PodType;

    enum HashOrWildcardStr {
        Hash(Hash),
        Wildcard(String),
    }

    fn l(s: &str) -> HashOrWildcardStr {
        HashOrWildcardStr::Hash(hash_str(s))
    }

    fn w(s: &str) -> HashOrWildcardStr {
        HashOrWildcardStr::Wildcard(s.to_string())
    }

    enum BuilderArg {
        Literal(Value),
        Key(HashOrWildcardStr, HashOrWildcardStr),
    }

    impl From<(HashOrWildcardStr, HashOrWildcardStr)> for BuilderArg {
        fn from((pod_id, key): (HashOrWildcardStr, HashOrWildcardStr)) -> Self {
            Self::Key(pod_id, key)
        }
    }

    impl<V> From<V> for BuilderArg
    where
        V: Into<Value>,
    {
        fn from(v: V) -> Self {
            Self::Literal(v.into())
        }
    }

    struct StatementTmplBuilder {
        predicate: Predicate,
        args: Vec<BuilderArg>,
    }

    fn st_tmpl(p: impl Into<Predicate>) -> StatementTmplBuilder {
        StatementTmplBuilder {
            predicate: p.into(),
            args: Vec::new(),
        }
    }

    impl StatementTmplBuilder {
        fn arg(mut self, a: impl Into<BuilderArg>) -> Self {
            self.args.push(a.into());
            self
        }
    }

    struct CustomPredicateBatchBuilder {
        predicates: Vec<CustomPredicate>,
    }

    impl CustomPredicateBatchBuilder {
        fn new() -> Self {
            Self {
                predicates: Vec::new(),
            }
        }

        fn predicate_and(
            &mut self,
            args: &[&str],
            priv_args: &[&str],
            sts: &[StatementTmplBuilder],
        ) -> Predicate {
            self.predicate(true, args, priv_args, sts)
        }

        fn predicate_or(
            &mut self,
            args: &[&str],
            priv_args: &[&str],
            sts: &[StatementTmplBuilder],
        ) -> Predicate {
            self.predicate(false, args, priv_args, sts)
        }

        fn predicate(
            &mut self,
            conjunction: bool,
            args: &[&str],
            priv_args: &[&str],
            sts: &[StatementTmplBuilder],
        ) -> Predicate {
            use BuilderArg as BA;
            let statements = sts
                .iter()
                .map(|sb| {
                    let args = sb
                        .args
                        .iter()
                        .map(|a| match a {
                            BA::Literal(v) => StatementTmplArg::Literal(*v),
                            BA::Key(pod_id, key) => StatementTmplArg::Key(
                                resolve_wildcard(args, priv_args, pod_id),
                                resolve_wildcard(args, priv_args, key),
                            ),
                        })
                        .collect();
                    StatementTmpl(sb.predicate.clone(), args)
                })
                .collect();
            let custom_predicate = CustomPredicate {
                conjunction,
                statements,
                args_len: args.len(),
            };
            self.predicates.push(custom_predicate);
            Predicate::BatchSelf(self.predicates.len() - 1)
        }

        fn finish(self) -> Arc<CustomPredicateBatch> {
            Arc::new(CustomPredicateBatch {
                predicates: self.predicates,
            })
        }
    }

    fn resolve_wildcard(
        args: &[&str],
        priv_args: &[&str],
        v: &HashOrWildcardStr,
    ) -> HashOrWildcard {
        match v {
            HashOrWildcardStr::Hash(h) => HashOrWildcard::Hash(*h),
            HashOrWildcardStr::Wildcard(s) => HashOrWildcard::Wildcard(
                args.iter()
                    .chain(priv_args.iter())
                    .enumerate()
                    .find_map(|(i, name)| (&s == name).then_some(i))
                    .unwrap(),
            ),
        }
    }

    #[test]
    fn test_custom_pred() {
        use NativePredicate as NP;

        let mut builder = CustomPredicateBatchBuilder::new();
        let eth_friend = builder.predicate_and(
            &["src_or", "src_key", "dst_or", "dst_key"],
            &["attestation_pod"],
            &[
                st_tmpl(NP::ValueOf)
                    .arg((w("attestation_pod"), l("type")))
                    .arg(PodType::Signed),
                st_tmpl(NP::Equal)
                    .arg((w("attestation_pod"), l("signer")))
                    .arg((w("src_or"), w("src_key"))),
                st_tmpl(NP::Equal)
                    .arg((w("attestation_pod"), l("attestation")))
                    .arg((w("dst_or"), w("dst_key"))),
            ],
        );

        println!("a.0. eth_friend = {}", builder.predicates.last().unwrap());
        let eth_friend = builder.finish();
        // This batch only has 1 predicate, so we pick it already for convenience
        let eth_friend = Predicate::Custom(eth_friend, 0);

        let mut builder = CustomPredicateBatchBuilder::new();
        let eth_dos_distance_base = builder.predicate_and(
            &[
                "src_or",
                "src_key",
                "dst_or",
                "dst_key",
                "distance_or",
                "distance_key",
            ],
            &[],
            &[
                st_tmpl(NP::Equal)
                    .arg((w("src_or"), l("src_key")))
                    .arg((w("dst_or"), w("dst_key"))),
                st_tmpl(NP::ValueOf)
                    .arg((w("distance_or"), w("distance_key")))
                    .arg(0),
            ],
        );

        println!(
            "b.0. eth_dos_distance_base = {}",
            builder.predicates.last().unwrap()
        );

        let eth_dos_distance = Predicate::BatchSelf(3);

        let eth_dos_distance_ind = builder.predicate_and(
            &[
                "src_or",
                "src_key",
                "dst_or",
                "dst_key",
                "distance_or",
                "distance_key",
            ],
            &[
                "one_or",
                "one_key",
                "shorter_distance_or",
                "shorter_distance_key",
                "intermed_or",
                "intermed_key",
            ],
            &[
                st_tmpl(eth_dos_distance)
                    .arg((w("src_or"), w("src_key")))
                    .arg((w("intermed_or"), w("intermed_key")))
                    .arg((w("shorter_distance_or"), w("shorter_distance_key"))),
                // distance == shorter_distance + 1
                st_tmpl(NP::ValueOf).arg((w("one_or"), w("one_key"))).arg(1),
                st_tmpl(NP::SumOf)
                    .arg((w("distance_or"), w("distance_key")))
                    .arg((w("shorter_distance_or"), w("shorter_distance_key")))
                    .arg((w("one_or"), w("one_key"))),
                // intermed is a friend of dst
                st_tmpl(eth_friend)
                    .arg((w("intermed_or"), w("intermed_key")))
                    .arg((w("dst_or"), w("dst_key"))),
            ],
        );

        println!(
            "b.1. eth_dos_distance_ind = {}",
            builder.predicates.last().unwrap()
        );

        let eth_dos_distance = builder.predicate_or(
            &[
                "src_or",
                "src_key",
                "dst_or",
                "dst_key",
                "distance_or",
                "distance_key",
            ],
            &[],
            &[
                st_tmpl(eth_dos_distance_base)
                    .arg((w("src_or"), w("src_key")))
                    .arg((w("dst_or"), w("dst_key")))
                    .arg((w("distance_or"), w("distance_key"))),
                st_tmpl(eth_dos_distance_ind)
                    .arg((w("src_or"), w("src_key")))
                    .arg((w("dst_or"), w("dst_key")))
                    .arg((w("distance_or"), w("distance_key"))),
            ],
        );

        println!(
            "b.2. eth_dos_distance = {}",
            builder.predicates.last().unwrap()
        );
    }
}
