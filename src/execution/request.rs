#[derive(Debug, Clone)]
pub enum ExecutionRequest {
    Command {
        server_id: String,
        command: String,
        confirm: bool,
    },

    Script {
        server_id: String,
        interpreter: String,
        content: String,
        confirm: bool,
    },
}