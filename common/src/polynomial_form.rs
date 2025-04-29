use serde::{Deserialize, Serialize};
use clap::ValueEnum;

#[derive(Debug, Clone, ValueEnum, Serialize, Deserialize)]
pub enum PolynomialForm {
    Coeff,
    Eval
}
