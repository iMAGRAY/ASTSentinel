use crate::analysis::ast::languages::SupportedLanguage;
use crate::analysis::ast::kind_ids::{self, KindIds};
use crate::analysis::metrics::ComplexityMetrics;
use anyhow::Result;
/// Tree-sitter visitor for calculating complexity metrics across languages
use tree_sitter::Node;

// Methods are included directly in this file for simplicity

/// Tree-sitter visitor for calculating complexity metrics
pub struct ComplexityVisitor<'a> {
    #[allow(dead_code)]
    source_code: &'a str,
    language: SupportedLanguage,
    // Optional fast-path: precomputed kind ids for hot nodes (per language)
    kind_ids: Option<KindIds>,

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
        // Use precomputed kind ids when available (constant-time)
        let kind_ids = kind_ids::get_for_language(language);
        Self {
            source_code,
            language,
            kind_ids,
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
            eprintln!(
                "Warning: No parameter list of type '{}' found in {} node",
                parameter_list_type, self.language
            );
        }

        Ok(())
    }

    /// Validate parameter list type for current language
    fn is_valid_parameter_list_type(&self, param_type: &str) -> bool {
        match self.get_parameter_node_kinds() {
            Ok(valid_types) => valid_types.contains(&param_type),
            Err(_e) => {
                // Log error for debugging - this typically happens for Rust which uses syn instead
                #[cfg(debug_assertions)]
                eprintln!("Parameter validation error for {}: {}", self.language, _e);
                false
            }
        }
    }

    /// Get language-specific parameter node kinds
    /// Returns the node types that represent parameter lists in function declarations

    /// Process C#-specific AST nodes
    pub fn process_csharp_node(&mut self, node_type: &str, node: &Node) -> Result<()> {
        if let Some(ref ids) = self.kind_ids {
            let k = node.kind_id();
            if k == ids.cs_method_declaration || k == ids.cs_constructor_declaration {
                self.function_count += 1;
                self.enter_scope();
                let _ = self.count_parameters(node, "parameter_list");
                return Ok(());
            }
            if k == ids.cs_if_statement
                || k == ids.cs_for_statement
                || k == ids.cs_while_statement
                || k == ids.cs_foreach_statement
                || k == ids.cs_switch_statement
            {
                self.cyclomatic_complexity += 1;
                if k != ids.cs_switch_statement {
                    self.cognitive_complexity += 1 + self.current_depth;
                }
                self.enter_scope();
                return Ok(());
            }
            if k == ids.cs_return_statement {
                self.return_points += 1;
                return Ok(());
            }
            if k == ids.cs_block {
                return Ok(());
            }
        }
        match node_type {
            "method_declaration" | "constructor_declaration" => {
                self.function_count += 1;
                self.enter_scope();
                self.count_parameters(node, "parameter_list")?;
            }
            "if_statement" | "for_statement" | "while_statement" | "foreach_statement" | "switch_statement" => {
                self.cyclomatic_complexity += 1;
                if node_type != "switch_statement" {
                    self.cognitive_complexity += 1 + self.current_depth;
                }
                self.enter_scope();
            }
            "return_statement" => self.return_points += 1,
            _ => {}
        }
        Ok(())
    }

    /// Process Go-specific AST nodes
    pub fn process_go_node(&mut self, node_type: &str, node: &Node) -> Result<()> {
        if let Some(ref ids) = self.kind_ids {
            let k = node.kind_id();
            if k == ids.go_function_declaration || k == ids.go_method_declaration {
                self.function_count += 1;
                self.enter_scope();
                let _ = self.count_parameters(node, "parameter_list");
                return Ok(());
            }
            if k == ids.go_if_statement
                || k == ids.go_for_statement
                || k == ids.go_switch_statement
                || k == ids.go_select_statement
            {
                self.cyclomatic_complexity += 1;
                if k != ids.go_switch_statement {
                    self.cognitive_complexity += 1 + self.current_depth;
                }
                self.enter_scope();
                return Ok(());
            }
            if k == ids.go_return_statement {
                self.return_points += 1;
                return Ok(());
            }
            if k == ids.go_block {
                return Ok(());
            }
        }
        match node_type {
            "function_declaration" | "method_declaration" => {
                self.function_count += 1;
                self.enter_scope();
                self.count_parameters(node, "parameter_list")?;
            }
            "if_statement" | "for_statement" | "switch_statement" | "select_statement" => {
                self.cyclomatic_complexity += 1;
                if node_type != "switch_statement" {
                    self.cognitive_complexity += 1 + self.current_depth;
                }
                self.enter_scope();
            }
            "return_statement" => self.return_points += 1,
            _ => {}
        }
        Ok(())
    }
    fn get_parameter_node_kinds(&self) -> Result<&'static [&'static str]> {
        match self.language {
            SupportedLanguage::Python => Ok(&["parameters", "lambda_parameters"]),
            SupportedLanguage::JavaScript | SupportedLanguage::TypeScript => {
                Ok(&["formal_parameters", "parameters"])
            }
            SupportedLanguage::Java => Ok(&["formal_parameters", "receiver_parameter"]),
            SupportedLanguage::CSharp => Ok(&["parameter_list", "formal_parameters"]),
            SupportedLanguage::Go => Ok(&["parameter_list", "parameters"]),
            SupportedLanguage::C | SupportedLanguage::Cpp => {
                Ok(&["parameter_list", "formal_parameters"])
            }
            SupportedLanguage::Php => Ok(&["formal_parameters", "parameters"]),
            SupportedLanguage::Ruby => Ok(&["parameters", "block_parameters"]),
            // Rust should use syn crate, not Tree-sitter
            SupportedLanguage::Rust => Err(anyhow::anyhow!(
                "Rust AST parsing should use syn crate, not Tree-sitter. \
                Tree-sitter cannot properly handle Rust macros and procedural syntax."
            )),
            // New languages with basic parameter support
            SupportedLanguage::Zig => Ok(&["parameter_list", "parameters"]),
            SupportedLanguage::V => Ok(&["parameter_list", "parameters"]),
            SupportedLanguage::Gleam => Ok(&["parameters", "parameter_list"]),
            // Config languages don't have function parameters
            SupportedLanguage::Json | SupportedLanguage::Yaml | SupportedLanguage::Toml => Ok(&[]),
        }
    }

    /// Get language-specific individual parameter node types
    /// Returns the node types that represent individual parameters within parameter lists
    fn get_individual_parameter_kinds(&self) -> Result<&'static [&'static str]> {
        match self.language {
            SupportedLanguage::Python => Ok(&[
                "identifier",
                "typed_parameter",
                "default_parameter",
                "list_splat_pattern",
            ]),
            SupportedLanguage::JavaScript | SupportedLanguage::TypeScript => Ok(&[
                "identifier",
                "formal_parameter",
                "rest_parameter",
                "object_pattern",
                "array_pattern",
            ]),
            SupportedLanguage::Java => {
                Ok(&["formal_parameter", "receiver_parameter", "spread_parameter"])
            }
            SupportedLanguage::CSharp => Ok(&["parameter", "parameter_array"]),
            SupportedLanguage::Go => {
                Ok(&["parameter_declaration", "variadic_parameter_declaration"])
            }
            SupportedLanguage::C | SupportedLanguage::Cpp => {
                Ok(&["parameter_declaration", "abstract_declarator"])
            }
            SupportedLanguage::Php => Ok(&["formal_parameter", "property_promotion_parameter"]),
            SupportedLanguage::Ruby => Ok(&[
                "identifier",
                "splat_parameter",
                "hash_splat_parameter",
                "block_parameter",
            ]),
            // Rust should use syn crate, not Tree-sitter
            SupportedLanguage::Rust => Err(anyhow::anyhow!(
                "Rust AST parsing should use syn crate, not Tree-sitter. \
                Tree-sitter cannot properly handle Rust macros and procedural syntax."
            )),
            // New languages with basic parameter support
            SupportedLanguage::Zig => Ok(&["identifier", "parameter_declaration"]),
            SupportedLanguage::V => Ok(&["identifier", "parameter"]),
            SupportedLanguage::Gleam => Ok(&["identifier", "parameter"]),
            // Config languages don't have function parameters
            SupportedLanguage::Json | SupportedLanguage::Yaml | SupportedLanguage::Toml => Ok(&[]),
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
                if valid_param_kinds.contains(&node_kind) {
                    count += 1;
                }

                if !cursor.goto_next_sibling() {
                    break;
                }
            }
        }
        Ok(count)
    }

    /// Find parameter node for arrow functions with different parameter structures
    /// Arrow functions can have: (a, b) => {}, a => {}, ({x, y}) => {}
    fn find_arrow_function_parameter<'b>(
        &self,
        arrow_function_node: &Node<'b>,
    ) -> Option<Node<'b>> {
        let mut cursor = arrow_function_node.walk();

        if cursor.goto_first_child() {
            loop {
                let child = cursor.node();
                match child.kind() {
                    // Direct identifier parameter: x => {}
                    "identifier" => return Some(child),
                    // Formal parameter list: (a, b) => {}
                    "formal_parameters" => return Some(child),
                    // Parameters without parentheses
                    "parameter" => return Some(child),
                    _ => {}
                }

                if !cursor.goto_next_sibling() {
                    break;
                }
            }
        }
        None
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
        let mut stack: Vec<(tree_sitter::Node, u32)> = vec![(*root, self.current_depth)];

        while let Some((node, initial_depth)) = stack.pop() {
            // Process current node and track depth changes
            self.process_node(&node)?;

            // Push children right-to-left without per-node heap allocations
            // to keep traversal order left-to-right on pop
            let child_count = node.child_count();
            if child_count > 0 {
                for i in (0..child_count).rev() {
                    if let Some(child) = node.child(i) {
                        stack.push((child, self.current_depth));
                    }
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
            SupportedLanguage::CSharp => self.process_csharp_node(node_type, node),
            SupportedLanguage::Go => self.process_go_node(node_type, node),
            SupportedLanguage::C => self.process_c_node(node_type, node),
            SupportedLanguage::Cpp => self.process_cpp_node(node_type, node),
            SupportedLanguage::Php => self.process_php_node(node_type, node),
            SupportedLanguage::Ruby => self.process_ruby_node(node_type, node),
            // New languages: Zig, V, and Gleam with basic AST support
            SupportedLanguage::Zig => self.process_zig_node(node_type, node),
            SupportedLanguage::V => self.process_v_node(node_type, node),
            SupportedLanguage::Gleam => self.process_gleam_node(node_type, node),
            // Config languages shouldn't use tree-sitter analysis
            SupportedLanguage::Json | SupportedLanguage::Yaml | SupportedLanguage::Toml => {
                // These languages should use regex-based analysis instead of AST
                Ok(())
            }
        }
    }

    fn process_python_node(&mut self, node_type: &str, node: &Node) -> Result<()> {
        // Fast path via kind ids when available
        if let Some(ref ids) = self.kind_ids {
            let k = node.kind_id();
            if k == ids.py_function_definition || k == ids.py_async_function_definition {
                let has_params = ids.py_parameters_id.is_some();
                self.function_count += 1;
                self.enter_scope();
                // count params
                if has_params {
                    if let Err(_) = self.count_parameters(node, "parameters") {
                        // fallback if tree version differs
                        let _ = self.count_parameters(node, "parameter_list");
                    }
                } else {
                    // fallback to string-based
                    let _ = self.count_parameters(node, "parameters");
                }
                return Ok(());
            }
            if k == ids.py_class_definition {
                self.enter_scope();
                return Ok(());
            }
            if k == ids.py_if_statement || k == ids.py_elif_clause {
                self.cyclomatic_complexity += 1;
                self.cognitive_complexity += 1 + self.current_depth;
                self.enter_scope();
                return Ok(());
            }
            if k == ids.py_while_statement || k == ids.py_for_statement {
                self.cyclomatic_complexity += 1;
                self.cognitive_complexity += 1 + self.current_depth;
                self.enter_scope();
                return Ok(());
            }
            if k == ids.py_try_statement {
                self.cyclomatic_complexity += 1;
                self.enter_scope();
                return Ok(());
            }
            if k == ids.py_except_clause {
                self.cyclomatic_complexity += 1;
                self.cognitive_complexity += 1;
                return Ok(());
            }
            if k == ids.py_return_statement {
                self.return_points += 1;
                return Ok(());
            }
        }
        match node_type {
            // Tree-sitter Python actual node types
            "function_definition" | "async_function_definition" | "def" => {
                self.function_count += 1;
                self.enter_scope();
                // Try both parameter list types for Python AST compatibility
                // Tree-sitter Python may use "parameters" or "parameter_list" depending on version
                if let Err(_) = self.count_parameters(node, "parameters") {
                    if let Err(_e) = self.count_parameters(node, "parameter_list") {
                        #[cfg(debug_assertions)]
                        eprintln!(
                            "Warning: Failed to find parameter list in Python function: {}",
                            e
                        );
                    }
                }
            }
            "class_definition" | "class" => {
                self.enter_scope();
            }
            "if_statement" | "elif_clause" | "if" | "elif" => {
                self.cyclomatic_complexity += 1;
                self.cognitive_complexity += 1 + self.current_depth;
                self.enter_scope();
            }
            "while_statement" | "for_statement" | "while" | "for" => {
                self.cyclomatic_complexity += 1;
                self.cognitive_complexity += 1 + self.current_depth;
                self.enter_scope();
            }
            "try_statement" | "try" => {
                self.cyclomatic_complexity += 1;
                self.enter_scope();
            }
            "except_clause" | "except" => {
                self.cyclomatic_complexity += 1;
                self.cognitive_complexity += 1;
            }
            "return_statement" | "return" => {
                self.return_points += 1;
            }
            "and" | "or" | "boolean_operator" => {
                self.cyclomatic_complexity += 1;
            }
            // Common structural nodes that don't affect complexity
            "module"
            | "block"
            | "identifier"
            | "parameters"
            | "("
            | ")"
            | ":"
            | "comment"
            | "string"
            | "integer"
            | "pass_statement"
            | "pass"
            | "expression_statement"
            | "assignment"
            | "="
            | "call"
            | "attribute"
            | "."
            | "argument_list"
            | ","
            | "import_statement"
            | "import"
            | "dotted_name"
            | "string_start"
            | "string_content"
            | "string_end"
            | "interpolation"
            | "{"
            | "}"
            | "as_pattern"
            | "as"
            | "as_pattern_target"
            | "dictionary"
            | "pair"
            | "list"
            | "["
            | "]"
            | "comparison_operator"
            | "=="
            | "!="
            | "<"
            | ">"
            | "<="
            | ">="
            | "not_operator"
            | "not"
            | "keyword_argument"
            | "subscript"
            | "slice"
            | "list_comprehension"
            | "for_in_clause"
            | "if_clause"
            | "augmented_assignment"
            | "+="
            | "-="
            | "pattern_list"
            | "in"
            | "else_clause"
            | "else"
            | "expression_list"
            | "binary_operator"
            | "+"
            | "-"
            | "*"
            | "/"
            | "%"
            | "**"
            | "//"
            | "none"
            | "true"
            | "false"
            | "float"
            | "parenthesized_expression"
            | "default_parameter"
            | "list_splat_pattern"
            | "dictionary_splat_pattern"
            | "typed_parameter"
            | "typed_default_parameter" => {
                // These are structural nodes, no complexity impact
            }
            _ => {
                // Only log truly unrecognized node types
                #[cfg(debug_assertions)]
                eprintln!("Unrecognized Python node type: {node_type}");
            }
        }
        Ok(())
    }

    /// Process C-specific AST nodes
    pub fn process_c_node(&mut self, node_type: &str, node: &Node) -> Result<()> {
        if let Some(ref ids) = self.kind_ids {
            let k = node.kind_id();
            if k == ids.c_function_definition {
                self.function_count += 1;
                self.enter_scope();
                let _ = self.count_parameters(node, "parameter_list");
                return Ok(());
            }
            if k == ids.c_if_statement
                || k == ids.c_for_statement
                || k == ids.c_while_statement
                || k == ids.c_switch_statement
            {
                self.cyclomatic_complexity += 1;
                if k != ids.c_switch_statement {
                    self.cognitive_complexity += 1 + self.current_depth;
                }
                self.enter_scope();
                return Ok(());
            }
            if k == ids.c_return_statement {
                self.return_points += 1;
                return Ok(());
            }
            if k == ids.c_compound_statement {
                return Ok(());
            }
        }
        match node_type {
            "function_definition" => {
                self.function_count += 1;
                self.enter_scope();
                let _ = self.count_parameters(node, "parameter_list");
            }
            "if_statement" | "for_statement" | "while_statement" | "switch_statement" => {
                self.cyclomatic_complexity += 1;
                if node_type != "switch_statement" {
                    self.cognitive_complexity += 1 + self.current_depth;
                }
                self.enter_scope();
            }
            "return_statement" => self.return_points += 1,
            _ => {}
        }
        Ok(())
    }

    /// Process C++-specific AST nodes (shares many node kinds with C)
    pub fn process_cpp_node(&mut self, node_type: &str, node: &Node) -> Result<()> {
        if let Some(ref ids) = self.kind_ids {
            let k = node.kind_id();
            if k == ids.cpp_function_definition {
                self.function_count += 1;
                self.enter_scope();
                let _ = self.count_parameters(node, "parameter_list");
                return Ok(());
            }
            if k == ids.cpp_if_statement
                || k == ids.cpp_for_statement
                || k == ids.cpp_while_statement
                || k == ids.cpp_switch_statement
            {
                self.cyclomatic_complexity += 1;
                if k != ids.cpp_switch_statement {
                    self.cognitive_complexity += 1 + self.current_depth;
                }
                self.enter_scope();
                return Ok(());
            }
            if k == ids.cpp_return_statement {
                self.return_points += 1;
                return Ok(());
            }
            if k == ids.cpp_compound_statement {
                return Ok(());
            }
        }
        match node_type {
            "function_definition" => {
                self.function_count += 1;
                self.enter_scope();
                let _ = self.count_parameters(node, "parameter_list");
            }
            "if_statement" | "for_statement" | "while_statement" | "switch_statement" => {
                self.cyclomatic_complexity += 1;
                if node_type != "switch_statement" {
                    self.cognitive_complexity += 1 + self.current_depth;
                }
                self.enter_scope();
            }
            "return_statement" => self.return_points += 1,
            _ => {}
        }
        Ok(())
    }

    /// Process PHP-specific AST nodes
    pub fn process_php_node(&mut self, node_type: &str, node: &Node) -> Result<()> {
        if let Some(ref ids) = self.kind_ids {
            let k = node.kind_id();
            if k == ids.php_function_definition || k == ids.php_method_declaration {
                self.function_count += 1;
                self.enter_scope();
                // PHP parameters
                let _ = self.count_parameters(node, "formal_parameters");
                return Ok(());
            }
            if k == ids.php_if_statement
                || k == ids.php_for_statement
                || k == ids.php_while_statement
                || k == ids.php_switch_statement
            {
                self.cyclomatic_complexity += 1;
                if k != ids.php_switch_statement {
                    self.cognitive_complexity += 1 + self.current_depth;
                }
                self.enter_scope();
                return Ok(());
            }
            if k == ids.php_return_statement {
                self.return_points += 1;
                return Ok(());
            }
            if k == ids.php_compound_statement {
                return Ok(());
            }
        }
        match node_type {
            "function_definition" | "method_declaration" => {
                self.function_count += 1;
                self.enter_scope();
                let _ = self.count_parameters(node, "formal_parameters");
            }
            "if_statement" | "for_statement" | "while_statement" | "switch_statement" => {
                self.cyclomatic_complexity += 1;
                if node_type != "switch_statement" {
                    self.cognitive_complexity += 1 + self.current_depth;
                }
                self.enter_scope();
            }
            "return_statement" => self.return_points += 1,
            _ => {}
        }
        Ok(())
    }

    /// Process Ruby-specific AST nodes
    pub fn process_ruby_node(&mut self, node_type: &str, node: &Node) -> Result<()> {
        if let Some(ref ids) = self.kind_ids {
            let k = node.kind_id();
            if k == ids.ruby_method || k == ids.ruby_def {
                self.function_count += 1;
                self.enter_scope();
                // Ruby parameters
                let _ = self.count_parameters(node, "parameters");
                return Ok(());
            }
            if k == ids.ruby_if || k == ids.ruby_elsif || k == ids.ruby_while || k == ids.ruby_for || k == ids.ruby_case {
                self.cyclomatic_complexity += 1;
                if k != ids.ruby_case {
                    self.cognitive_complexity += 1 + self.current_depth;
                }
                self.enter_scope();
                return Ok(());
            }
            if k == ids.ruby_when {
                self.cyclomatic_complexity += 1;
                return Ok(());
            }
            if k == ids.ruby_return {
                self.return_points += 1;
                return Ok(());
            }
        }
        match node_type {
            // Tree-sitter-ruby uses 'method' and also 'def' tokens
            "method" | "def" => {
                self.function_count += 1;
                self.enter_scope();
                let _ = self.count_parameters(node, "parameters");
            }
            "if" | "elsif" | "while" | "for" | "case" => {
                self.cyclomatic_complexity += 1;
                if node_type != "case" {
                    self.cognitive_complexity += 1 + self.current_depth;
                }
                self.enter_scope();
            }
            "when" => {
                self.cyclomatic_complexity += 1;
            }
            "return" => self.return_points += 1,
            _ => {}
        }
        Ok(())
    }

    fn process_js_ts_node(&mut self, node_type: &str, node: &Node) -> Result<()> {
        if let Some(ref ids) = self.kind_ids {
            let k = node.kind_id();
            if k == ids.js_function_declaration
                || k == ids.js_function_expression
                || k == ids.js_method_definition
                || k == ids.ts_function_declaration
            {
                self.function_count += 1;
                self.enter_scope();
                // Try both param kinds
                if let Err(_) = self.count_parameters(node, "formal_parameters") {
                    let _ = self.count_parameters(node, "parameters");
                }
                return Ok(());
            }
            if k == ids.js_if_statement {
                self.cyclomatic_complexity += 1;
                self.cognitive_complexity += 1 + self.current_depth;
                self.enter_scope();
                return Ok(());
            }
            if k == ids.js_for_statement || k == ids.js_while_statement {
                self.cyclomatic_complexity += 1;
                self.cognitive_complexity += 1 + self.current_depth;
                self.enter_scope();
                return Ok(());
            }
            if k == ids.js_return_statement {
                self.return_points += 1;
                return Ok(());
            }
        }
        match node_type {
            // Tree-sitter JavaScript actual node types
            "function_declaration"
            | "function_expression"
            | "method_definition"
            | "function"
            | "async" => {
                self.function_count += 1;
                self.enter_scope();
                // Try both parameter list types for JavaScript
                if let Err(_) = self.count_parameters(node, "formal_parameters") {
                    if let Err(_e) = self.count_parameters(node, "parameters") {
                        #[cfg(debug_assertions)]
                        eprintln!(
                            "Warning: Failed to find parameter list in JavaScript function: {}",
                            e
                        );
                    }
                }
            }
            "arrow_function" => {
                self.function_count += 1;
                self.enter_scope();
                // Arrow functions have different parameter structures
                // They can have single identifier as parameter or formal_parameters
                if let Err(_) = self.count_parameters(node, "formal_parameters") {
                    if let Err(_) = self.count_parameters(node, "parameters") {
                        // For arrow functions, check if there's a direct identifier parameter
                        if let Some(param_node) = self.find_arrow_function_parameter(node) {
                            if param_node.kind() == "identifier" {
                                self.parameter_count += 1;
                            }
                        }
                    }
                }
            }
            "class_declaration" | "class" | "class_body" => {
                self.enter_scope();
            }
            "if_statement" | "if" => {
                self.cyclomatic_complexity += 1;
                self.cognitive_complexity += 1 + self.current_depth;
                self.enter_scope();
            }
            "while_statement" | "for_statement" | "for_in_statement" | "for_of_statement"
            | "while" | "for" => {
                self.cyclomatic_complexity += 1;
                self.cognitive_complexity += 1 + self.current_depth;
                self.enter_scope();
            }
            "switch_statement" | "switch" => {
                self.cyclomatic_complexity += 1;
                self.enter_scope();
            }
            "case_clause" | "case" => {
                self.cyclomatic_complexity += 1;
            }
            "try_statement" | "try" => {
                self.enter_scope();
            }
            "catch_clause" | "catch" => {
                self.cyclomatic_complexity += 1;
                self.cognitive_complexity += 1;
            }
            "return_statement" | "return" => {
                self.return_points += 1;
            }
            "logical_expression" | "binary_expression" => {
                // && or || operators, +, -, *, / etc
                self.cyclomatic_complexity += 1;
            }
            // Common structural nodes that don't affect complexity
            "program"
            | "statement_block"
            | "identifier"
            | "formal_parameters"
            | "("
            | ")"
            | "{"
            | "comment"
            | "string"
            | "number"
            | ";"
            | "property_identifier"
            | "."
            | ","
            | "variable_declaration"
            | "variable_declarator"
            | "var"
            | "let"
            | "const"
            | "="
            | "call_expression"
            | "member_expression"
            | "arguments"
            | "string_fragment"
            | "lexical_declaration"
            | "expression_statement"
            | "assignment_expression"
            | "parenthesized_expression"
            | "object"
            | "pair"
            | ":"
            | "true"
            | "false"
            | "null"
            | "template_string"
            | "`"
            | "template_substitution"
            | "${"
            | "}"
            | "await_expression"
            | "await"
            | "=="
            | "!="
            | "<"
            | ">"
            | "<="
            | ">="
            | "+"
            | "-"
            | "*"
            | "/"
            | "%"
            | "&&"
            | "||"
            | "!"
            | "++"
            | "--"
            | "+="
            | "-="
            | "*="
            | "/="
            | "\""
            | "'"
            | "escape_sequence" => {
                // These are structural nodes, no complexity impact
            }
            _ => {
                // Only log truly unrecognized node types
                #[cfg(debug_assertions)]
                eprintln!("Unrecognized JavaScript/TypeScript node type: {node_type}");
            }
        }
        Ok(())
    }

    /// Process Java-specific AST nodes
    pub fn process_java_node(&mut self, node_type: &str, node: &Node) -> Result<()> {
        // Fast path via kind ids when available
        if let Some(ref ids) = self.kind_ids {
            let k = node.kind_id();
            if k == ids.java_method_declaration {
                self.function_count += 1;
                self.enter_scope();
                let _ = self.count_parameters(node, "formal_parameters");
                return Ok(());
            }
            if k == ids.java_if_statement || k == ids.java_for_statement || k == ids.java_while_statement || k == ids.java_switch_expression {
                self.cyclomatic_complexity += 1;
                if k != ids.java_switch_expression {
                    self.cognitive_complexity += 1 + self.current_depth;
                }
                self.enter_scope();
                return Ok(());
            }
            if k == ids.java_return_statement {
                self.return_points += 1;
                return Ok(());
            }
            if k == ids.java_block {
                // scope exits handled by traversal; nothing to do here
                return Ok(());
            }
        }

        match node_type {
            "method_declaration" | "constructor_declaration" => {
                self.function_count += 1;
                self.enter_scope();
                self.count_parameters(node, "formal_parameters")?;
            }
            "class_declaration" | "interface_declaration" => {
                self.enter_scope();
            }
            "if_statement" | "for_statement" | "while_statement" | "switch_expression" => {
                self.cyclomatic_complexity += 1;
                self.cognitive_complexity += if node_type == "switch_expression" { 0 } else { 1 + self.current_depth };
                self.enter_scope();
            }
            "return_statement" => {
                self.return_points += 1;
            }
            _ => {
                #[cfg(debug_assertions)]
                eprintln!("Unhandled Java node type: {node_type}");
            }
        }
        Ok(())
    }

    /// Process Zig-specific AST nodes with enhanced parallel pattern matching
    pub fn process_zig_node(&mut self, node_type: &str, node: &Node) -> Result<()> {
        match node_type {
            // Zig function definitions
            "fn_decl" | "function_declaration" => {
                self.function_count += 1;
                self.enter_scope();
                self.count_parameters(node, "parameter_list")?;
            }
            // Control flow statements
            "if_statement" | "if_expression" => {
                self.cyclomatic_complexity += 1;
                self.cognitive_complexity += 1 + self.current_depth;
                self.enter_scope();
            }
            "while_statement" | "while_expression" | "for_statement" => {
                self.cyclomatic_complexity += 1;
                self.cognitive_complexity += 1 + self.current_depth;
                self.enter_scope();
            }
            "switch_statement" | "switch_expression" => {
                self.cyclomatic_complexity += 1;
                self.enter_scope();
            }
            "switch_case" => {
                self.cyclomatic_complexity += 1;
            }
            "return_statement" => {
                self.return_points += 1;
            }
            // Error handling
            "try_expression" | "catch_expression" => {
                self.cyclomatic_complexity += 1;
                self.cognitive_complexity += 1;
            }
            // Zig-specific constructs
            "comptime" | "defer" | "errdefer" => {
                self.cognitive_complexity += 1;
            }
            _ => {
                #[cfg(debug_assertions)]
                eprintln!("Unhandled Zig node type: {node_type}");
            }
        }
        Ok(())
    }

    /// Process V Lang-specific AST nodes with parallel processing optimization
    pub fn process_v_node(&mut self, node_type: &str, node: &Node) -> Result<()> {
        match node_type {
            // V function definitions
            "fn_declaration" | "function_declaration" => {
                self.function_count += 1;
                self.enter_scope();
                self.count_parameters(node, "parameter_list")?;
            }
            // Control flow
            "if_statement" | "if_expression" => {
                self.cyclomatic_complexity += 1;
                self.cognitive_complexity += 1 + self.current_depth;
                self.enter_scope();
            }
            "for_statement" | "for_in_statement" => {
                self.cyclomatic_complexity += 1;
                self.cognitive_complexity += 1 + self.current_depth;
                self.enter_scope();
            }
            "match_statement" => {
                self.cyclomatic_complexity += 1;
                self.enter_scope();
            }
            "match_branch" => {
                self.cyclomatic_complexity += 1;
            }
            "return_statement" => {
                self.return_points += 1;
            }
            // V-specific constructs
            "or_block" | "optional_propagation" => {
                self.cyclomatic_complexity += 1;
                self.cognitive_complexity += 1;
            }
            _ => {
                #[cfg(debug_assertions)]
                eprintln!("Unhandled V Lang node type: {node_type}");
            }
        }
        Ok(())
    }

    /// Process Gleam-specific AST nodes with functional programming patterns
    pub fn process_gleam_node(&mut self, node_type: &str, node: &Node) -> Result<()> {
        match node_type {
            // Gleam function definitions
            "function_definition" | "anonymous_function" => {
                self.function_count += 1;
                self.enter_scope();
                self.count_parameters(node, "function_parameters")?;
            }
            // Pattern matching and control flow
            "case_expression" => {
                self.cyclomatic_complexity += 1;
                self.enter_scope();
            }
            "case_clause" => {
                self.cyclomatic_complexity += 1;
            }
            // Gleam uses if/else as expressions
            "if_expression" => {
                self.cyclomatic_complexity += 1;
                self.cognitive_complexity += 1 + self.current_depth;
            }
            // Result and Option handling
            "try_expression" => {
                self.cyclomatic_complexity += 1;
                self.cognitive_complexity += 1;
            }
            // Gleam-specific constructs
            "use_expression" | "assert_expression" => {
                self.cognitive_complexity += 1;
            }
            "todo" | "panic" => {
                self.return_points += 1;
            }
            _ => {
                #[cfg(debug_assertions)]
                eprintln!("Unhandled Gleam node type: {node_type}");
            }
        }
        Ok(())
    }

    /// Enhanced parallel processing for file analysis with rayon integration
    pub fn process_files_parallel(
        files: &[(String, SupportedLanguage)],
    ) -> Result<Vec<crate::analysis::metrics::ComplexityMetrics>> {
        use rayon::prelude::*;

        files
            .par_iter()
            .map(|(content, language)| {
                let visitor = ComplexityVisitor::new(content, *language);
                // For now, return basic metrics - full AST parsing would require proper tree-sitter integration
                Ok(visitor.build_metrics())
            })
            .collect()
    }

    /// Process generic AST nodes for languages without specific handlers
    pub fn process_generic_node(&mut self, node_type: &str, _node: &Node) -> Result<()> {
        match node_type {
            "function_definition"
            | "function_declaration"
            | "function_expression"
            | "method_definition"
            | "method_declaration"
            | "arrow_function"
            | "lambda"
            | "function"
            | "def" => {
                self.function_count += 1;
                self.enter_scope();
            }
            "if_statement" | "if_expression" | "conditional" | "ternary_expression" => {
                self.cyclomatic_complexity += 1;
                self.cognitive_complexity += 1 + self.current_depth;
                self.enter_scope();
            }
            "while_statement" | "while_loop" | "for_statement" | "for_loop"
            | "for_in_statement" | "for_of_statement" | "foreach_statement"
            | "do_while_statement" | "repeat_statement" => {
                self.cyclomatic_complexity += 1;
                self.cognitive_complexity += 1 + self.current_depth;
                self.enter_scope();
            }
            "switch_statement" | "switch_expression" | "case_statement" | "match_statement"
            | "pattern_match" => {
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
                eprintln!(
                    "Unrecognized node type for {}: {}",
                    self.language, node_type
                );
            }
        }
        Ok(())
    }
}

// NOTE: KindIds and per-language caches are now provided by crate::analysis::ast::kind_ids
