/// Tree-sitter visitor for calculating complexity metrics across languages
use tree_sitter::Node;
use crate::analysis::metrics::ComplexityMetrics;
use crate::analysis::ast::languages::SupportedLanguage;
use anyhow::Result;

// Methods are included directly in this file for simplicity

/// Tree-sitter visitor for calculating complexity metrics
pub struct ComplexityVisitor<'a> {
    source_code: &'a str,
    language: SupportedLanguage,
    
    // Complexity metrics
    cyclomatic_complexity: u32,
    cognitive_complexity: u32,
    nesting_depth: u32,
    current_depth: u32,
    function_count: u32,
    parameter_count: u32,
    return_points: u32,
    line_count: usize,
}

impl<'a> ComplexityVisitor<'a> {
    pub fn new(source_code: &'a str, language: SupportedLanguage) -> Self {
        Self {
            source_code,
            language,
            cyclomatic_complexity: 1, // Base complexity
            cognitive_complexity: 0,
            nesting_depth: 0,
            current_depth: 0,
            function_count: 0,
            parameter_count: 0,
            return_points: 0,
            line_count: source_code.lines().count(),
        }
    }

    /// Enter a new scope (increase nesting depth)
    pub fn enter_scope(&mut self) {
        self.current_depth += 1;
        if self.current_depth > self.nesting_depth {
            self.nesting_depth = self.current_depth;
        }
    }

    /// Exit current scope (decrease nesting depth)
    pub fn exit_scope(&mut self) {
        if self.current_depth > 0 {
            self.current_depth -= 1;
        }
    }

    /// Count parameters in a function node with error handling
    /// Searches for ALL parameter lists in the node, not just the first one
    pub fn count_parameters(&mut self, node: &Node, parameter_list_type: &str) -> Result<()> {
        // Validate parameter_list_type for current language
        if !self.is_valid_parameter_list_type(parameter_list_type) {
            return Err(anyhow::anyhow!(
                "Invalid parameter list type '{}' for language {}", 
                parameter_list_type, 
                self.language
            ));
        }

        let mut found_parameter_list = false;
        let mut cursor = node.walk();
        
        if cursor.goto_first_child() {
            loop {
                let child = cursor.node();
                if child.kind() == parameter_list_type {
                    // Count parameter nodes within this parameter list
                    let param_count = self.count_parameter_nodes(&child)?;
                    self.parameter_count += param_count;
                    found_parameter_list = true;
                    // Continue searching for additional parameter lists
                }
                if !cursor.goto_next_sibling() {
                    break;
                }
            }
        }

        // Log when no parameter list is found for debugging
        if !found_parameter_list {
            #[cfg(debug_assertions)]
            eprintln!("Warning: No parameter list of type '{}' found in {} node", 
                     parameter_list_type, self.language);
        }

        Ok(())
    }

    /// Validate parameter list type for current language
    fn is_valid_parameter_list_type(&self, param_type: &str) -> bool {
        match self.get_parameter_node_kinds() {
            Ok(valid_types) => valid_types.iter().any(|&valid| valid == param_type),
            Err(e) => {
                // Log error for debugging - this typically happens for Rust which uses syn instead
                #[cfg(debug_assertions)]
                eprintln!("Parameter validation error for {}: {}", self.language, e);
                false
            }
        }
    }

    /// Get language-specific parameter node kinds
    /// Returns the node types that represent parameter lists in function declarations
    fn get_parameter_node_kinds(&self) -> Result<&'static [&'static str]> {
        match self.language {
            SupportedLanguage::Python => Ok(&["parameters", "lambda_parameters"]),
            SupportedLanguage::JavaScript | SupportedLanguage::TypeScript => 
                Ok(&["formal_parameters", "parameters"]),
            SupportedLanguage::Java => Ok(&["formal_parameters", "receiver_parameter"]),
            SupportedLanguage::CSharp => Ok(&["parameter_list", "formal_parameters"]),
            SupportedLanguage::Go => Ok(&["parameter_list", "parameters"]),
            SupportedLanguage::C | SupportedLanguage::Cpp => Ok(&["parameter_list", "formal_parameters"]),
            SupportedLanguage::Php => Ok(&["formal_parameters", "parameters"]),
            SupportedLanguage::Ruby => Ok(&["parameters", "block_parameters"]),
            // Rust should use syn crate, not Tree-sitter
            SupportedLanguage::Rust => Err(anyhow::anyhow!(
                "Rust AST parsing should use syn crate, not Tree-sitter. \
                Tree-sitter cannot properly handle Rust macros and procedural syntax."
            )),
        }
    }

    /// Get language-specific individual parameter node types
    /// Returns the node types that represent individual parameters within parameter lists
    fn get_individual_parameter_kinds(&self) -> Result<&'static [&'static str]> {
        match self.language {
            SupportedLanguage::Python => Ok(&["identifier", "typed_parameter", "default_parameter", "list_splat_pattern"]),
            SupportedLanguage::JavaScript | SupportedLanguage::TypeScript => 
                Ok(&["identifier", "formal_parameter", "rest_parameter", "object_pattern", "array_pattern"]),
            SupportedLanguage::Java => Ok(&["formal_parameter", "receiver_parameter", "spread_parameter"]),
            SupportedLanguage::CSharp => Ok(&["parameter", "parameter_array"]),
            SupportedLanguage::Go => Ok(&["parameter_declaration", "variadic_parameter_declaration"]),
            SupportedLanguage::C | SupportedLanguage::Cpp => Ok(&["parameter_declaration", "abstract_declarator"]),
            SupportedLanguage::Php => Ok(&["formal_parameter", "property_promotion_parameter"]),
            SupportedLanguage::Ruby => Ok(&["identifier", "splat_parameter", "hash_splat_parameter", "block_parameter"]),
            // Rust should use syn crate, not Tree-sitter
            SupportedLanguage::Rust => Err(anyhow::anyhow!(
                "Rust AST parsing should use syn crate, not Tree-sitter. \
                Tree-sitter cannot properly handle Rust macros and procedural syntax."
            )),
        }
    }

    /// Helper to count individual parameter nodes with error handling
    fn count_parameter_nodes(&self, param_list_node: &Node) -> Result<u32> {
        let mut count = 0;
        let valid_param_kinds = self.get_individual_parameter_kinds()?;
        
        let mut cursor = param_list_node.walk();
        if cursor.goto_first_child() {
            loop {
                let child = cursor.node();
                let node_kind = child.kind();
                
                // Use language-specific parameter kinds for better accuracy
                if valid_param_kinds.iter().any(|&kind| kind == node_kind) {
                    count += 1;
                }
                
                if !cursor.goto_next_sibling() {
                    break;
                }
            }
        }
        Ok(count)
    }

    /// Build complexity metrics from current state
    pub fn build_metrics(&self) -> ComplexityMetrics {
        ComplexityMetrics {
            cyclomatic_complexity: self.cyclomatic_complexity,
            cognitive_complexity: self.cognitive_complexity,
            nesting_depth: self.nesting_depth,
            function_count: self.function_count,
            parameter_count: self.parameter_count,
            return_points: self.return_points,
            line_count: self.line_count,
        }
    }

    /// Visit AST nodes iteratively to prevent stack overflow on deep trees
    pub fn visit_node(&mut self, root: &Node) -> Result<()> {
        // Use iterative traversal with explicit stack to prevent stack overflow
        // Each stack entry contains (node, initial_depth) to track scope changes
        let mut stack: Vec<(tree_sitter::Node, u32)> = vec![(root.clone(), self.current_depth)];

        while let Some((node, initial_depth)) = stack.pop() {
            // Process current node and track depth changes
            self.process_node(&node)?;

            // Add children to stack in reverse order for left-to-right processing
            let mut cursor = node.walk();
            if cursor.goto_first_child() {
                let mut children = Vec::new();
                loop {
                    children.push((cursor.node(), self.current_depth));
                    if !cursor.goto_next_sibling() {
                        break;
                    }
                }
                
                // Reverse to maintain left-to-right traversal order
                for child in children.into_iter().rev() {
                    stack.push(child);
                }
            }

            // Restore depth after processing this subtree (auto-exit scope if entered)
            // This ensures proper scope management for cognitive complexity calculation
            if self.current_depth > initial_depth {
                self.exit_scope();
            }
        }

        Ok(())
    }

    fn process_node(&mut self, node: &Node) -> Result<()> {
        let node_type = node.kind();
        
        match self.language {
            SupportedLanguage::Python => self.process_python_node(node_type, node),
            SupportedLanguage::JavaScript | SupportedLanguage::TypeScript => {
                self.process_js_ts_node(node_type, node)
            }
            SupportedLanguage::Java => self.process_java_node(node_type, node),
            SupportedLanguage::Rust => {
                // Rust should use syn crate, not Tree-sitter - warn and fall back to generic
                #[cfg(debug_assertions)]
                eprintln!("Warning: Rust should use syn crate for AST analysis, not Tree-sitter");
                self.process_generic_node(node_type, node)
            }
            SupportedLanguage::CSharp | SupportedLanguage::Go | 
            SupportedLanguage::C | SupportedLanguage::Cpp |
            SupportedLanguage::Php | SupportedLanguage::Ruby => {
                // Use generic processing for languages without specific handlers yet
                self.process_generic_node(node_type, node)
            }
        }
    }

    fn process_python_node(&mut self, node_type: &str, node: &Node) -> Result<()> {
        match node_type {
            "function_definition" | "async_function_definition" => {
                self.function_count += 1;
                self.enter_scope();
                self.count_parameters(node, "parameters")?;
            }
            "class_definition" => {
                self.enter_scope();
            }
            "if_statement" | "elif_clause" => {
                self.cyclomatic_complexity += 1;
                self.cognitive_complexity += 1 + self.current_depth;
                self.enter_scope();
            }
            "while_statement" | "for_statement" => {
                self.cyclomatic_complexity += 1;
                self.cognitive_complexity += 1 + self.current_depth;
                self.enter_scope();
            }
            "try_statement" => {
                self.cyclomatic_complexity += 1;
                self.enter_scope();
            }
            "except_clause" => {
                self.cyclomatic_complexity += 1;
                self.cognitive_complexity += 1;
            }
            "return_statement" => {
                self.return_points += 1;
            }
            "and" | "or" => {
                self.cyclomatic_complexity += 1;
            }
            _ => {
                // Unrecognized node type - log for debugging in development
                #[cfg(debug_assertions)]
                eprintln!("Unrecognized Python node type: {}", node_type);
            }
        }
        Ok(())
    }

    fn process_js_ts_node(&mut self, node_type: &str, node: &Node) -> Result<()> {
        match node_type {
            "function_declaration" | "function_expression" | "arrow_function" | "method_definition" => {
                self.function_count += 1;
                self.enter_scope();
                self.count_parameters(node, "formal_parameters")?;
            }
            "class_declaration" => {
                self.enter_scope();
            }
            "if_statement" => {
                self.cyclomatic_complexity += 1;
                self.cognitive_complexity += 1 + self.current_depth;
                self.enter_scope();
            }
            "while_statement" | "for_statement" | "for_in_statement" | "for_of_statement" => {
                self.cyclomatic_complexity += 1;
                self.cognitive_complexity += 1 + self.current_depth;
                self.enter_scope();
            }
            "switch_statement" => {
                self.cyclomatic_complexity += 1;
                self.enter_scope();
            }
            "case_clause" => {
                self.cyclomatic_complexity += 1;
            }
            "try_statement" => {
                self.enter_scope();
            }
            "catch_clause" => {
                self.cyclomatic_complexity += 1;
                self.cognitive_complexity += 1;
            }
            "return_statement" => {
                self.return_points += 1;
            }
            "logical_expression" => {
                // && or || operators
                self.cyclomatic_complexity += 1;
            }
            _ => {
                // Unrecognized node type - log for debugging in development
                #[cfg(debug_assertions)]
                eprintln!("Unrecognized JavaScript/TypeScript node type: {}", node_type);
            }
        }
        Ok(())
    }

    /// Process Java-specific AST nodes
    pub fn process_java_node(&mut self, node_type: &str, node: &Node) -> Result<()> {
        match node_type {
            "method_declaration" | "constructor_declaration" => {
                self.function_count += 1;
                self.enter_scope();
                self.count_parameters(node, "formal_parameters")?;
            }
            "class_declaration" | "interface_declaration" => {
                self.enter_scope();
            }
            "if_statement" => {
                self.cyclomatic_complexity += 1;
                self.cognitive_complexity += 1 + self.current_depth;
                self.enter_scope();
            }
            _ => {
                #[cfg(debug_assertions)]
                eprintln!("Unhandled Java node type: {}", node_type);
            }
        }
        Ok(())
    }

    /// Process generic AST nodes for languages without specific handlers
    pub fn process_generic_node(&mut self, node_type: &str, _node: &Node) -> Result<()> {
        match node_type {
            "function_definition" | "function_declaration" | "function_expression" |
            "method_definition" | "method_declaration" | "arrow_function" |
            "lambda" | "function" | "def" => {
                self.function_count += 1;
                self.enter_scope();
            }
            "if_statement" | "if_expression" | "conditional" | "ternary_expression" => {
                self.cyclomatic_complexity += 1;
                self.cognitive_complexity += 1 + self.current_depth;
                self.enter_scope();
            }
            "while_statement" | "while_loop" | "for_statement" | "for_loop" |
            "for_in_statement" | "for_of_statement" | "foreach_statement" |
            "do_while_statement" | "repeat_statement" => {
                self.cyclomatic_complexity += 1;
                self.cognitive_complexity += 1 + self.current_depth;
                self.enter_scope();
            }
            "switch_statement" | "switch_expression" | "case_statement" |
            "match_statement" | "pattern_match" => {
                self.cyclomatic_complexity += 1;
                self.enter_scope();
            }
            "case_clause" | "case_label" | "switch_label" | "match_arm" => {
                self.cyclomatic_complexity += 1;
            }
            "return_statement" | "return_expression" | "yield_statement" => {
                self.return_points += 1;
            }
            "logical_and" | "logical_or" | "boolean_and" | "boolean_or" => {
                self.cyclomatic_complexity += 1;
            }
            _ => {
                #[cfg(debug_assertions)]
                eprintln!("Unrecognized node type for {}: {}", self.language, node_type);
            }
        }
        Ok(())
    }
}