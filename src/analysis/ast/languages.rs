/// Multi-language AST analysis using Tree-sitter and specialized parsers
use anyhow::Result;
use tree_sitter::{Language, Parser};

use crate::analysis::ast::visitor::ComplexityVisitor;
use crate::analysis::metrics::ComplexityMetrics;

/// Supported languages for AST analysis
/// Note: Rust uses syn crate for superior macro and procedural parsing,
/// while other languages use Tree-sitter for consistent cross-language support
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SupportedLanguage {
    /// Rust - handled by syn crate (not Tree-sitter)
    Rust,
    /// Python - Tree-sitter based
    Python,
    /// JavaScript - Tree-sitter based  
    JavaScript,
    /// TypeScript - Tree-sitter based
    TypeScript,
    /// Java - Tree-sitter based
    Java,
    /// C# - Tree-sitter based
    CSharp,
    /// Go - Tree-sitter based
    Go,
    /// C - Tree-sitter based
    C,
    /// C++ - Tree-sitter based
    Cpp,
    /// PHP - Tree-sitter based
    Php,
    /// Ruby - Tree-sitter based
    Ruby,
}

impl SupportedLanguage {
    pub fn from_extension(ext: &str) -> Option<Self> {
        match ext {
            "rs" => Some(Self::Rust),
            "py" => Some(Self::Python),
            "js" | "mjs" => Some(Self::JavaScript),
            "ts" | "tsx" => Some(Self::TypeScript),
            "jsx" => Some(Self::JavaScript),
            "java" => Some(Self::Java),
            "cs" => Some(Self::CSharp),
            "go" => Some(Self::Go),
            "c" | "h" => Some(Self::C),
            "cpp" | "cc" | "cxx" | "hpp" | "hxx" => Some(Self::Cpp),
            "php" => Some(Self::Php),
            "rb" => Some(Self::Ruby),
            _ => None,
        }
    }

    pub fn get_tree_sitter_language(self) -> Result<Language> {
        match self {
            Self::Rust => anyhow::bail!("Rust uses syn crate, not tree-sitter"),
            Self::Python => Ok(tree_sitter_python::language()),
            Self::JavaScript => Ok(tree_sitter_javascript::language()),
            Self::TypeScript => Ok(tree_sitter_typescript::language_typescript()),
            Self::Java => Ok(tree_sitter_java::language()),
            Self::CSharp => Ok(tree_sitter_c_sharp::language()),
            Self::Go => Ok(tree_sitter_go::language()),
            Self::C => Ok(tree_sitter_c::language()),
            Self::Cpp => Ok(tree_sitter_cpp::language()),
            Self::Php => Ok(tree_sitter_php::language_php()),
            Self::Ruby => Ok(tree_sitter_ruby::language()),
        }
    }
}

impl std::fmt::Display for SupportedLanguage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Rust => write!(f, "Rust"),
            Self::Python => write!(f, "Python"),
            Self::JavaScript => write!(f, "JavaScript"),
            Self::TypeScript => write!(f, "TypeScript"),
            Self::Java => write!(f, "Java"),
            Self::CSharp => write!(f, "C#"),
            Self::Go => write!(f, "Go"),
            Self::C => write!(f, "C"),
            Self::Cpp => write!(f, "C++"),
            Self::Php => write!(f, "PHP"),
            Self::Ruby => write!(f, "Ruby"),
        }
    }
}

/// Multi-language AST analyzer using Tree-sitter
pub struct MultiLanguageAnalyzer;

impl MultiLanguageAnalyzer {
    /// Analyze source code with Tree-sitter and return complexity metrics
    pub fn analyze_with_tree_sitter(
        source_code: &str,
        language: SupportedLanguage,
    ) -> Result<ComplexityMetrics> {
        // Input validation
        if source_code.is_empty() {
            return Err(anyhow::anyhow!("Source code cannot be empty"));
        }

        // Additional validation for extremely long input to prevent resource exhaustion
        if source_code.len() > 10_000_000 {
            // 10MB limit
            return Err(anyhow::anyhow!(
                "Source code too large (>10MB), potential DoS risk"
            ));
        }

        // Rust should use syn crate, not Tree-sitter
        if language == SupportedLanguage::Rust {
            return Err(anyhow::anyhow!(
                "Rust analysis should use syn crate, not Tree-sitter"
            ));
        }

        // Get Tree-sitter language
        let ts_language = language.get_tree_sitter_language()?;

        // Create parser
        let mut parser = Parser::new();
        parser.set_language(&ts_language)?;

        // Parse source code with error handling for malformed syntax
        let tree = parser.parse(source_code, None).ok_or_else(|| {
            anyhow::anyhow!(
                "Failed to parse {} source code - syntax may be invalid",
                language
            )
        })?;

        // Validate tree structure to prevent potential crashes
        let root_node = tree.root_node();
        if root_node.has_error() {
            return Err(anyhow::anyhow!(
                "Source code contains syntax errors that prevent analysis"
            ));
        }

        // Create visitor and analyze AST
        let mut visitor = ComplexityVisitor::new(source_code, language);
        visitor.visit_node(&root_node)?;

        Ok(visitor.build_metrics())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_supported_language_from_extension() {
        assert_eq!(
            SupportedLanguage::from_extension("py"),
            Some(SupportedLanguage::Python)
        );
        assert_eq!(
            SupportedLanguage::from_extension("js"),
            Some(SupportedLanguage::JavaScript)
        );
        assert_eq!(
            SupportedLanguage::from_extension("ts"),
            Some(SupportedLanguage::TypeScript)
        );
        assert_eq!(
            SupportedLanguage::from_extension("java"),
            Some(SupportedLanguage::Java)
        );
        assert_eq!(
            SupportedLanguage::from_extension("rs"),
            Some(SupportedLanguage::Rust)
        );
        assert_eq!(SupportedLanguage::from_extension("unknown"), None);
    }

    #[test]
    fn test_tree_sitter_language_creation() {
        // Test that we can create Tree-sitter languages for supported languages
        assert!(SupportedLanguage::Python.get_tree_sitter_language().is_ok());
        assert!(SupportedLanguage::JavaScript
            .get_tree_sitter_language()
            .is_ok());
        assert!(SupportedLanguage::Java.get_tree_sitter_language().is_ok());

        // Rust should fail as it uses syn crate
        assert!(SupportedLanguage::Rust.get_tree_sitter_language().is_err());
    }

    #[test]
    fn test_analyze_empty_code() {
        let result = MultiLanguageAnalyzer::analyze_with_tree_sitter("", SupportedLanguage::Python);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("empty"));
    }

    #[test]
    fn test_analyze_rust_rejection() {
        let result = MultiLanguageAnalyzer::analyze_with_tree_sitter(
            "fn main() {}",
            SupportedLanguage::Rust,
        );
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("syn crate"));
    }

    #[test]
    fn test_analyze_simple_python() {
        let python_code = "def hello():\n    return 'world'";
        let result =
            MultiLanguageAnalyzer::analyze_with_tree_sitter(python_code, SupportedLanguage::Python);
        assert!(result.is_ok());

        let metrics = result.unwrap();
        assert!(metrics.function_count >= 1);
        assert!(metrics.line_count >= 2);
    }

    #[test]
    fn test_analyze_simple_javascript() {
        let js_code = "function hello() { return 'world'; }";
        let result =
            MultiLanguageAnalyzer::analyze_with_tree_sitter(js_code, SupportedLanguage::JavaScript);
        assert!(result.is_ok());

        let metrics = result.unwrap();
        assert!(metrics.function_count >= 1);
    }
}
