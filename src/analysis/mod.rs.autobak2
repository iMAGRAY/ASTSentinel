/// Code analysis modules for project inspection and metrics
pub mod ast;
pub mod dependencies;
pub mod duplicate_detector;
pub mod metrics;
pub mod project;
pub mod timings;

// Re-export commonly used types
pub use dependencies::{analyze_project_dependencies, DependencyInfo, PackageManager, ProjectDependencies};
pub use metrics::ComplexityMetrics;
pub use project::{format_project_structure_for_ai, scan_project_structure, ProjectStructure, ScanConfig};
