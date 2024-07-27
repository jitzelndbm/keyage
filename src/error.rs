use owo_colors::OwoColorize;
use std::fmt;
use thiserror::Error as ThisError;

const ERROR_PREFIX: &str = "[keyage error]";

#[derive(ThisError)]
pub enum Error {
    #[error("TODO: error message")]
    Todo,
}

impl fmt::Debug for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // NOTE: \r, because there is a default prefix "Error: ", don't know how to remove it...
        // Not the best approach, there might be another solution
        writeln!(
            f,
            "\r{}{}{}",
            ERROR_PREFIX.red().bold(),
            ": ".bold(),
            self.to_string().bold()
        )
    }
}
