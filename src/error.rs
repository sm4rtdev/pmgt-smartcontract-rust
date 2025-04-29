use std::fmt;
use std::error::Error;

/// Custom error types for the CLI application
#[derive(Debug)]
pub enum CliError {
    /// Invalid address format
    InvalidAddress,
    /// Failed to parse response
    ParseError,
    /// No result returned from contract call
    NoResult,
    /// Contract call failed
    ContractCallFailed,
    /// Invalid contract method
    InvalidMethod,
    /// Conversion error
    ConversionError,
}

impl fmt::Display for CliError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CliError::InvalidAddress => write!(f, "Invalid address format"),
            CliError::ParseError => write!(f, "Failed to parse response"),
            CliError::NoResult => write!(f, "No result returned from contract call"),
            CliError::ContractCallFailed => write!(f, "Contract call failed"),
            CliError::InvalidMethod => write!(f, "Invalid contract method"),
            CliError::ConversionError => write!(f, "Failed to convert value"),
        }
    }
}

impl Error for CliError {} 