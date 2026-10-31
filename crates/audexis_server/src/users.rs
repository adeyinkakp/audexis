mod commands;
mod quota;

pub use commands::{Command, parse, run};
pub(crate) use quota::default_quota;

pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

pub fn help() {
    println!(
        "Audexis server

Usage:
  audexis_server                             Start the server
  audexis_server users create <name> [--quota <size>]
  audexis_server users list
  audexis_server users reset-key <name>
  audexis_server users quota <name> <size>
  audexis_server users disable <name>
  audexis_server users enable <name>

Sizes: 100GB, 1TB, or a bytes. New users default to
AUDEXIS_QUOTA_BYTES (100GB if no var)."
    );
}
