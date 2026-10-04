pub mod connection;
pub mod manager;
mod sudo;

pub use connection::{CommandResult, SshConnection};
pub use manager::{ServerInfo, SshManager};
