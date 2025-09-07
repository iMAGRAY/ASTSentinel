/// Code analysis modules for project inspection and metrics
pub mod ast;
pub mod metrics;
pub mod project;

// Re-export commonly used types
pub use metrics::ComplexityMetrics;
pub use project::{
    ProjectStructure,
    scan_project_structure,
    format_project_structure_for_ai,
    ScanConfig,
};