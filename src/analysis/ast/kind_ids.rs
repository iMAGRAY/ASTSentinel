use anyhow::Result;
use lazy_static::lazy_static;
use std::collections::HashMap;
use crate::analysis::ast::languages::{LanguageCache, SupportedLanguage};

#[derive(Copy, Clone)]
pub struct KindIds {
    // Python
    pub py_function_definition: u16,
    pub py_async_function_definition: u16,
    pub py_parameters_id: Option<u16>,
    pub py_return_statement: u16,
    pub py_if_statement: u16,
    pub py_elif_clause: u16,
    pub py_while_statement: u16,
    pub py_for_statement: u16,
    pub py_try_statement: u16,
    pub py_except_clause: u16,
    pub py_class_definition: u16,
    // JS/TS
    pub js_function_declaration: u16,
    pub js_function_expression: u16,
    pub js_method_definition: u16,
    pub js_if_statement: u16,
    pub js_for_statement: u16,
    pub js_while_statement: u16,
    pub js_return_statement: u16,
    pub ts_function_declaration: u16,
    // Java
    pub java_method_declaration: u16,
    pub java_if_statement: u16,
    pub java_for_statement: u16,
    pub java_while_statement: u16,
    pub java_switch_expression: u16,
    pub java_return_statement: u16,
    pub java_block: u16,
    // C#
    pub cs_method_declaration: u16,
    pub cs_constructor_declaration: u16,
    pub cs_if_statement: u16,
    pub cs_for_statement: u16,
    pub cs_while_statement: u16,
    pub cs_foreach_statement: u16,
    pub cs_switch_statement: u16,
    pub cs_return_statement: u16,
    pub cs_block: u16,
    // Go
    pub go_function_declaration: u16,
    pub go_method_declaration: u16,
    pub go_if_statement: u16,
    pub go_for_statement: u16,
    pub go_switch_statement: u16,
    pub go_select_statement: u16,
    pub go_return_statement: u16,
    pub go_block: u16,
    // C
    pub c_function_definition: u16,
    pub c_if_statement: u16,
    pub c_for_statement: u16,
    pub c_while_statement: u16,
    pub c_switch_statement: u16,
    pub c_return_statement: u16,
    pub c_compound_statement: u16,
    // C++
    pub cpp_function_definition: u16,
    pub cpp_if_statement: u16,
    pub cpp_for_statement: u16,
    pub cpp_while_statement: u16,
    pub cpp_switch_statement: u16,
    pub cpp_return_statement: u16,
    pub cpp_compound_statement: u16,
    // PHP
    pub php_function_definition: u16,
    pub php_method_declaration: u16,
    pub php_if_statement: u16,
    pub php_for_statement: u16,
    pub php_while_statement: u16,
    pub php_switch_statement: u16,
    pub php_return_statement: u16,
    pub php_compound_statement: u16,
    // Ruby
    pub ruby_method: u16,
    pub ruby_def: u16,
    pub ruby_if: u16,
    pub ruby_elsif: u16,
    pub ruby_while: u16,
    pub ruby_for: u16,
    pub ruby_case: u16,
    pub ruby_when: u16,
    pub ruby_return: u16,
}

impl KindIds {
    pub fn for_language(lang: SupportedLanguage) -> Result<Self> {
        match lang {
            SupportedLanguage::Rust | SupportedLanguage::Json | SupportedLanguage::Yaml | SupportedLanguage::Toml => {
                anyhow::bail!("No kind ids for non tree-sitter languages")
            }
            _ => {}
        }

        let l = LanguageCache::get_or_create_language(lang)?;
        let id = |name: &str, named: bool| -> u16 { l.id_for_node_kind(name, named) };
        Ok(Self {
            // Python
            py_function_definition: id("function_definition", true),
            py_async_function_definition: id("async_function_definition", true),
            py_parameters_id: Some(id("parameters", true)).filter(|v| *v != 0),
            py_return_statement: id("return_statement", true),
            py_if_statement: id("if_statement", true),
            py_elif_clause: id("elif_clause", true),
            py_while_statement: id("while_statement", true),
            py_for_statement: id("for_statement", true),
            py_try_statement: id("try_statement", true),
            py_except_clause: id("except_clause", true),
            py_class_definition: id("class_definition", true),
            // JS/TS
            js_function_declaration: id("function_declaration", true),
            js_function_expression: id("function_expression", true),
            js_method_definition: id("method_definition", true),
            js_if_statement: id("if_statement", true),
            js_for_statement: id("for_statement", true),
            js_while_statement: id("while_statement", true),
            js_return_statement: id("return_statement", true),
            ts_function_declaration: id("function_declaration", true),
            // Java
            java_method_declaration: id("method_declaration", true),
            java_if_statement: id("if_statement", true),
            java_for_statement: id("for_statement", true),
            java_while_statement: id("while_statement", true),
            java_switch_expression: id("switch_expression", true),
            java_return_statement: id("return_statement", true),
            java_block: id("block", true),
            // C#
            cs_method_declaration: id("method_declaration", true),
            cs_constructor_declaration: id("constructor_declaration", true),
            cs_if_statement: id("if_statement", true),
            cs_for_statement: id("for_statement", true),
            cs_while_statement: id("while_statement", true),
            cs_foreach_statement: id("foreach_statement", true),
            cs_switch_statement: id("switch_statement", true),
            cs_return_statement: id("return_statement", true),
            cs_block: id("block", true),
            // Go
            go_function_declaration: id("function_declaration", true),
            go_method_declaration: id("method_declaration", true),
            go_if_statement: id("if_statement", true),
            go_for_statement: id("for_statement", true),
            go_switch_statement: id("switch_statement", true),
            go_select_statement: id("select_statement", true),
            go_return_statement: id("return_statement", true),
            go_block: id("block", true),
            // C
            c_function_definition: id("function_definition", true),
            c_if_statement: id("if_statement", true),
            c_for_statement: id("for_statement", true),
            c_while_statement: id("while_statement", true),
            c_switch_statement: id("switch_statement", true),
            c_return_statement: id("return_statement", true),
            c_compound_statement: id("compound_statement", true),
            // C++
            cpp_function_definition: id("function_definition", true),
            cpp_if_statement: id("if_statement", true),
            cpp_for_statement: id("for_statement", true),
            cpp_while_statement: id("while_statement", true),
            cpp_switch_statement: id("switch_statement", true),
            cpp_return_statement: id("return_statement", true),
            cpp_compound_statement: id("compound_statement", true),
            // PHP
            php_function_definition: id("function_definition", true),
            php_method_declaration: id("method_declaration", true),
            php_if_statement: id("if_statement", true),
            php_for_statement: id("for_statement", true),
            php_while_statement: id("while_statement", true),
            php_switch_statement: id("switch_statement", true),
            php_return_statement: id("return_statement", true),
            php_compound_statement: id("compound_statement", true),
            // Ruby
            ruby_method: id("method", true),
            ruby_def: id("def", true),
            ruby_if: id("if", true),
            ruby_elsif: id("elsif", true),
            ruby_while: id("while", true),
            ruby_for: id("for", true),
            ruby_case: id("case", true),
            ruby_when: id("when", true),
            ruby_return: id("return", true),
        })
    }
}

lazy_static! {
    static ref KIND_IDS: HashMap<SupportedLanguage, KindIds> = {
        let mut m = HashMap::new();
        for &lang in [
            SupportedLanguage::Python,
            SupportedLanguage::JavaScript,
            SupportedLanguage::TypeScript,
            SupportedLanguage::Java,
            SupportedLanguage::CSharp,
            SupportedLanguage::Go,
            SupportedLanguage::C,
            SupportedLanguage::Cpp,
            SupportedLanguage::Php,
            SupportedLanguage::Ruby,
        ]
        .iter()
        {
            if let Ok(ids) = KindIds::for_language(lang) {
                m.insert(lang, ids);
            }
        }
        m
    };
}

pub fn get_for_language(lang: SupportedLanguage) -> Option<KindIds> {
    KIND_IDS.get(&lang).copied()
}

