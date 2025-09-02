/// External service providers for AI and other integrations

pub mod ai;

// Re-export AI provider types and client
pub use ai::{
    UniversalAIClient,
    AIProvider,
};