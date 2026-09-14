pub mod detector;
pub mod models;
pub mod manager;

pub use detector::detect;
pub use models::{
    InitSystem,
    PackageManager,
    ServerCapabilities,
};