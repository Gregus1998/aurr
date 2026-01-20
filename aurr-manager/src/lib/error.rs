use std::fmt;
use std::error::Error;
use azure_core::Error as Azure_Core_Error;
use azure_storage::Error as Azure_storage_Error;

#[derive(Debug)]
pub enum CustomError {

    IoError(std::io::Error),
    ParseError(String),
    AzureCoreError(Azure_Core_Error),
    AzureStorageError(Azure_storage_Error),
    ConnectionError(String),
    GenericError(String),
}

/// Implementation of Display for each custom error -> each new error should be included in the match for a special formatting.
impl fmt::Display for CustomError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {

            CustomError::AzureStorageError(e) => {
                write!(f, "AzureStorageError: {}", e)},
            
            CustomError::AzureCoreError(e) => {
                write!(f, "AzureCoreError: {}",e)},

            CustomError::IoError(e) => {
                write!(f,"IO-ERROR -> Probably some bad input: {}",e)},

            CustomError::ParseError(s) => {
                todo!()},

            CustomError::ConnectionError(s) => {
                write!(f, "Could not connect to azure_storage_account due to: {}",s)},

            CustomError::GenericError(s) => {
                write!(f, "Generic error: {:?}",s)},
            }


        }
    
}

impl Error for CustomError{}

///Implementing a casting of std::io::Error to CustomError -> good example on how to do it.
impl From<std::io::Error> for CustomError {

    fn from(value: std::io::Error) -> Self {
        CustomError::IoError(value)
    }
    
}
