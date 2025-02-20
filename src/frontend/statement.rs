use std::fmt;

use anyhow::{anyhow, Result};

use crate::middleware::{self, NativeStatement};

use super::{AnchoredKey, Value};

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

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Statement(pub NativeStatement, pub Vec<StatementArg>);

impl TryFrom<Statement> for middleware::Statement {
    type Error = anyhow::Error;
    fn try_from(s: Statement) -> Result<Self> {
        type MS = middleware::Statement;
        type NS = NativeStatement;
        type SA = StatementArg;
        let args = (
            s.1.get(0).cloned(),
            s.1.get(1).cloned(),
            s.1.get(2).cloned(),
        );
        Ok(match (s.0, args) {
            (NS::None, (None, None, None)) => MS::None,
            (NS::ValueOf, (Some(SA::Key(ak)), Some(StatementArg::Literal(v)), None)) => {
                MS::ValueOf(ak.into(), (&v).into())
            }
            (NS::Equal, (Some(SA::Key(ak1)), Some(SA::Key(ak2)), None)) => {
                MS::Equal(ak1.into(), ak2.into())
            }
            (NS::NotEqual, (Some(SA::Key(ak1)), Some(SA::Key(ak2)), None)) => {
                MS::NotEqual(ak1.into(), ak2.into())
            }
            (NS::Gt, (Some(SA::Key(ak1)), Some(SA::Key(ak2)), None)) => {
                MS::Gt(ak1.into(), ak2.into())
            }
            (NS::Lt, (Some(SA::Key(ak1)), Some(SA::Key(ak2)), None)) => {
                MS::Lt(ak1.into(), ak2.into())
            }
            (NS::Contains, (Some(SA::Key(ak1)), Some(SA::Key(ak2)), None)) => {
                MS::Contains(ak1.into(), ak2.into())
            }
            (NS::NotContains, (Some(SA::Key(ak1)), Some(SA::Key(ak2)), None)) => {
                MS::NotContains(ak1.into(), ak2.into())
            }
            (NS::SumOf, (Some(SA::Key(ak1)), Some(SA::Key(ak2)), Some(SA::Key(ak3)))) => {
                MS::SumOf(ak1.into(), ak2.into(), ak3.into())
            }
            (NS::ProductOf, (Some(SA::Key(ak1)), Some(SA::Key(ak2)), Some(SA::Key(ak3)))) => {
                MS::ProductOf(ak1.into(), ak2.into(), ak3.into())
            }
            (NS::MaxOf, (Some(SA::Key(ak1)), Some(SA::Key(ak2)), Some(SA::Key(ak3)))) => {
                MS::MaxOf(ak1.into(), ak2.into(), ak3.into())
            }
            _ => Err(anyhow!("Ill-formed statement: {}", s))?,
        })
    }
}

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
