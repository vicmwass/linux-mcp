pub mod connection;
pub mod manager;

pub use connection::{CommandResult, SshConnection};
pub use manager::{ServerInfo, SshManager};