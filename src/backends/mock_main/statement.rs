use anyhow::{anyhow, Result};
use std::fmt;

use crate::middleware::{self, NativePredicate, StatementArg, ToFields};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Statement(pub NativePredicate, pub Vec<StatementArg>);

impl Statement {
    pub fn is_none(&self) -> bool {
        self.0 == NativePredicate::None
    }
    /// Argument method. Trailing Nones are filtered out.
    pub fn args(&self) -> Vec<StatementArg> {
        let maybe_last_arg_index = (0..self.1.len()).rev().find(|i| !self.1[*i].is_none());
        match maybe_last_arg_index {
            None => vec![],
            Some(i) => self.1[0..i + 1].to_vec(),
        }
    }
}

impl ToFields for Statement {
    fn to_fields(self) -> (Vec<middleware::F>, usize) {
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

impl TryFrom<Statement> for middleware::Statement {
    type Error = anyhow::Error;
    fn try_from(s: Statement) -> Result<Self> {
        type S = middleware::Statement;
        type NP = NativePredicate;
        type SA = StatementArg;
        let proper_args = s.args();
        let args = (
            proper_args.get(0).cloned(),
            proper_args.get(1).cloned(),
            proper_args.get(2).cloned(),
        );
        Ok(match (s.0, args, proper_args.len()) {
            (NP::None, _, 0) => S::None,
            (NP::ValueOf, (Some(SA::Key(ak)), Some(SA::Literal(v)), None), 2) => S::ValueOf(ak, v),
            (NP::Equal, (Some(SA::Key(ak1)), Some(SA::Key(ak2)), None), 2) => S::Equal(ak1, ak2),
            (NP::NotEqual, (Some(SA::Key(ak1)), Some(SA::Key(ak2)), None), 2) => {
                S::NotEqual(ak1, ak2)
            }
            (NP::Gt, (Some(SA::Key(ak1)), Some(SA::Key(ak2)), None), 2) => S::Gt(ak1, ak2),
            (NP::Lt, (Some(SA::Key(ak1)), Some(SA::Key(ak2)), None), 2) => S::Lt(ak1, ak2),
            (NP::Contains, (Some(SA::Key(ak1)), Some(SA::Key(ak2)), None), 2) => {
                S::Contains(ak1, ak2)
            }
            (NP::NotContains, (Some(SA::Key(ak1)), Some(SA::Key(ak2)), None), 2) => {
                S::NotContains(ak1, ak2)
            }
            (NP::SumOf, (Some(SA::Key(ak1)), Some(SA::Key(ak2)), Some(SA::Key(ak3))), 3) => {
                S::SumOf(ak1, ak2, ak3)
            }
            (NP::ProductOf, (Some(SA::Key(ak1)), Some(SA::Key(ak2)), Some(SA::Key(ak3))), 3) => {
                S::ProductOf(ak1, ak2, ak3)
            }
            (NP::MaxOf, (Some(SA::Key(ak1)), Some(SA::Key(ak2)), Some(SA::Key(ak3))), 3) => {
                S::MaxOf(ak1, ak2, ak3)
            }
            _ => Err(anyhow!("Ill-formed statement expression {:?}", s))?,
        })
    }
}

impl From<middleware::Statement> for Statement {
    fn from(s: middleware::Statement) -> Self {
        Statement(s.code(), s.args().into_iter().map(|arg| arg).collect())
    }
}

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
