use std::fmt;

use anyhow::Result;

use crate::middleware::{self, NativeOperation};

use super::Statement;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum OperationArg {
    None,
    Index(usize),
}

impl OperationArg {
    pub fn is_none(&self) -> bool {
        matches!(self, OperationArg::None)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum OperationArgError {
    KeyNotFound,
    StatementNotFound,
}

impl std::fmt::Display for OperationArgError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OperationArgError::KeyNotFound => write!(f, "Key not found"),
            OperationArgError::StatementNotFound => write!(f, "Statement not found"),
        }
    }
}

impl std::error::Error for OperationArgError {}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Operation(pub NativeOperation, pub Vec<OperationArg>);

impl Operation {
    pub fn deref(&self, statements: &[Statement]) -> Result<crate::middleware::Operation> {
        let deref_args = self
            .1
            .iter()
            .flat_map(|arg| match arg {
                OperationArg::None => None,
                OperationArg::Index(i) => Some(statements[*i].clone().try_into()),
            })
            .collect::<Result<Vec<crate::middleware::Statement>>>()?;
        middleware::Operation::op(self.0, &deref_args)
    }
}

impl fmt::Display for Operation {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?} ", self.0)?;
        for (i, arg) in self.1.iter().enumerate() {
            if !(!f.alternate() && arg.is_none()) {
                if i != 0 {
                    write!(f, " ")?;
                }
                match arg {
                    OperationArg::None => write!(f, "none")?,
                    OperationArg::Index(i) => write!(f, "{:02}", i)?,
                }
            }
        }
        Ok(())
    }
}
