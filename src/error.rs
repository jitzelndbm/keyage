use std::io::Write;

use thiserror::Error as ThisError;

/// This result type makes sure that appropriate error messages are printed, in the right format.
pub type Result<T> = core::result::Result<T, Error>;

#[derive(Debug, ThisError)]
pub enum Error {
    #[error("Failed to load configuration: {0}")]
    ConfigLoad(String),
    #[error("Could not construct recipient, unexpected format found, expected age or ssh")]
    InvalidRecipientFormat,
    #[error("An unexpected error occured while encrypting with age")]
    AgeEncryption,
    #[error("An unexpected error occured while decrypting with age")]
    AgeDecryption,
    #[error("The path of the store could not be found: {0}")]
    StoreNotFound(String),
    #[error("The provided path could not be found in the store")]
    PasswordNotFound,
    #[error("An error occured while trying to read the store directory: {0}")]
    StoreRead(String),
    #[error("An error occured while trying to write to the store directory: {0}")]
    StoreWrite(String),
    #[error(
        "The path is not a valid path in the store (i.e. age extension and in store directory): {0}"
    )]
    InvalidPath(String),
    #[error("An OTP error occured: {0}")]
    Totp(String),
    #[error("An error occured while trying to display a qr code: {0}")]
    Qr(String),
    #[error("A conversion/construction error occured: {0}")]
    StringConversion(String),
    #[error("An error occured while tyring to generate a password: {0}")]
    PasswordGeneration(String),
    #[error("Could not display prompt: {0}")]
    Prompt(String),
}

pub fn default_error_handler(error: &Error, output: &mut dyn Write) {
    writeln!(output, "[error]: {:?}", error).ok();
}
