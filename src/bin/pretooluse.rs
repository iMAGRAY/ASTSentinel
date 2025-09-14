#![cfg_attr(
    not(test),
    deny(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::panic,
        clippy::todo,
        clippy::unimplemented,
        clippy::dbg_macro
    )
)]
#![allow(clippy::items_after_test_module)]
use anyhow::{Context, Result};
use std::fs::File;
use std::io::{self, Read};

use rust_validation_hooks::analysis::ast::{MultiLanguageAnalyzer, SupportedLanguage};
use rust_validation_hooks::analysis::dependencies::analyze_project_dependencies;
use rust_validation_hooks::analysis::project::{
    format_project_structure_for_ai, scan_project_structure, ScanConfig,
};
use rust_validation_hooks::*;
// Use universal AI client
use rust_validation_hooks::providers::ai::UniversalAIClient;
// Test file validator removed - AI handles validation
// Use diff formatter for better AI context
use rust_validation_hooks::config;
use rust_validation_hooks::validation::diff_formatter::{
    format_code_diff, format_edit_diff, format_multi_edit_diff,
};

#[inline]
fn dev_flag_enabled(name: &str) -> bool {
    #[cfg(debug_assertions)]
    {
        match std::env::var(name) {
            Ok(v) => v != "0" && !v.is_empty(),
            Err(_) => false,
        }
    }
    #[cfg(not(debug_assertions))]
    {
        let _ = name;
        false
    }
}
// =============================
// Heuristics for anti-cheating
// =============================
#[derive(Default, Debug)]
struct HeuristicAssessment {
    is_whitespace_or_comments_only: bool,
    is_return_constant_only: bool,
    is_print_or_log_only: bool,
    has_todo_or_placeholder: bool,
    has_empty_catch_or_except: bool,
    is_new_file_minimal: bool,
    // Stronger signals to reduce ложные срабатывания
    const_return_ignores_params: bool,
    logic_calls_removed: bool,
    // Контекст, где минимальная реализация допустима (версия/здоровье/ping)
    is_allowed_minimal_context: bool,
    has_silent_result_discard: bool,
    summary: String,
}

// ==============
// API Contract heuristics (regex-based; Python/JS/TS)
// ==============
fn extract_signatures_regex(
    language: Option<SupportedLanguage>,
    code: &str,
) -> std::collections::HashMap<String, Vec<String>> {
    use regex::Regex;
    let mut map = std::collections::HashMap::new();
    let Some(lang) = language else {
        return map;
    };
    match lang {
        SupportedLanguage::Python => {
            if let Ok(re) = Regex::new(r"(?m)^\s*def\s+([A-Za-z_][A-Za-z0-9_]*)\s*\(([^)]*)\)") {
                for cap in re.captures_iter(code) {
                    let name = cap.get(1).map(|m| m.as_str()).unwrap_or("").to_string();
                    let params = cap.get(2).map(|m| m.as_str()).unwrap_or("");
                    let list = params
                        .split(',')
                        .map(|p| p.trim())
                        .filter(|p| !p.is_empty())
                        .map(|p| {
                            let base = p
                                .split(':')
                                .next()
                                .unwrap_or("")
                                .split('=')
                                .next()
                                .unwrap_or("")
                                .trim();
                            base.trim_start_matches('*').to_string()
                        })
                        .filter(|p| !matches!(p.as_str(), "self" | "cls" | "args" | "kwargs"))
                        .collect::<Vec<_>>();
                    if !name.is_empty() {
                        map.insert(name, list);
                    }
                }
            }
        }
        SupportedLanguage::JavaScript | SupportedLanguage::TypeScript => {
            if let Ok(re_fn) = Regex::new(r"(?m)function\s+([A-Za-z_][A-Za-z0-9_]*)\s*\(([^)]*)\)") {
                for cap in re_fn.captures_iter(code) {
                    let name = cap.get(1).map(|m| m.as_str()).unwrap_or("").to_string();
                    let params = cap.get(2).map(|m| m.as_str()).unwrap_or("");
                    let list = params
                        .split(',')
                        .map(|p| p.trim())
                        .filter(|p| !p.is_empty())
                        .map(|p| {
                            p.trim_start_matches("...")
                                .split(':')
                                .next()
                                .unwrap_or("")
                                .trim_end_matches('?')
                                .to_string()
                        })
                        .collect::<Vec<_>>();
                    if !name.is_empty() {
                        map.insert(name, list);
                    }
                }
            }
            // Class methods: name(param){ ... }
            if let Ok(re_m) = Regex::new(r"(?m)^\s*([A-Za-z_][A-Za-z0-9_]*)\s*\(([^)]*)\)\s*\{") {
                for cap in re_m.captures_iter(code) {
                    let name = cap.get(1).map(|m| m.as_str()).unwrap_or("").to_string();
                    let params = cap.get(2).map(|m| m.as_str()).unwrap_or("");
                    let list = params
                        .split(',')
                        .map(|p| p.trim())
                        .filter(|p| !p.is_empty())
                        .map(|p| {
                            p.trim_start_matches("...")
                                .split(':')
                                .next()
                                .unwrap_or("")
                                .to_string()
                        })
                        .collect::<Vec<_>>();
                    if !name.is_empty() {
                        map.entry(name).or_insert(list.clone());
                    }
                }
            }
        }
        _ => {}
    }
    map
}

fn contract_weakening_reasons(
    language: Option<SupportedLanguage>,
    old_code: &str,
    new_code: &str,
) -> Vec<String> {
    let before = extract_signatures_regex(language, old_code);
    let after = extract_signatures_regex(language, new_code);
    if before.is_empty() && after.is_empty() {
        return vec![];
    }
    let mut reasons = Vec::new();
    for (name, bparams) in before.iter() {
        if let Some(aparams) = after.get(name) {
            if aparams.len() < bparams.len() {
                reasons.push(format!(
                    "Function `{}`: parameter count reduced ({} -> {})",
                    name,
                    bparams.len(),
                    aparams.len()
                ));
            } else {
                for bp in bparams {
                    if !bp.is_empty() && !aparams.iter().any(|x| x == bp) {
                        reasons.push(format!(
                            "Function `{}`: parameter `{}` removed or renamed",
                            name, bp
                        ));
                    }
                }
            }
        } else {
            reasons.push(format!("Function `{}`: removed from module", name));
        }
    }
    reasons
}

// Heuristic: find call sites that still pass removed parameters or exceed new
// arity
fn find_contract_callsite_issues(
    language: Option<SupportedLanguage>,
    old_code: &str,
    new_code: &str,
) -> Vec<String> {
    use std::collections::HashMap;
    let lang = language;
    let before = extract_signatures_regex(lang, old_code);
    let after = extract_signatures_regex(lang, new_code);
    let mut issues = Vec::new();

    // Helper: count top-level args and detect named args in a slice
    fn analyze_args_slice(slice: &str) -> (usize, Vec<String>) {
        let mut depth = 0usize;
        let mut count = 0usize;
        let mut named = Vec::new();
        let mut token = String::new();
        let mut i = 0;
        let bytes = slice.as_bytes();
        while i < bytes.len() {
            let c = bytes[i] as char;
            match c {
                '(' | '[' | '{' => {
                    depth += 1;
                    token.clear();
                }
                ')' | ']' | '}' => {
                    depth = depth.saturating_sub(1);
                }
                ',' => {
                    if depth == 0 {
                        count += 1;
                        token.clear();
                    }
                }
                '=' => {
                    if depth == 0 {
                        let name = token.trim();
                        if !name.is_empty() {
                            named.push(name.to_string());
                        }
                    }
                }
                _ => {
                    if depth == 0 {
                        token.push(c);
                    }
                }
            }
            i += 1;
        }
        // If non-empty args without trailing comma, increment count
        let trimmed = slice.trim();
        if !trimmed.is_empty() {
            count = count.saturating_add(1);
        }
        (count, named)
    }

    // Extract removed signatures
    let mut removed_params: HashMap<String, Vec<String>> = HashMap::new();
    let mut reduced_arity: HashMap<String, (usize, usize)> = HashMap::new();
    for (name, bparams) in before.iter() {
        if let Some(aparams) = after.get(name) {
            if aparams.len() < bparams.len() {
                reduced_arity.insert(name.clone(), (bparams.len(), aparams.len()));
            }
            let mut removed = Vec::new();
            for bp in bparams {
                // Python boilerplate ignore
                if matches!(lang, Some(SupportedLanguage::Python))
                    && (bp == "self" || bp == "cls" || bp == "args" || bp == "kwargs")
                {
                    continue;
                }
                if !bp.is_empty() && !aparams.iter().any(|x| x == bp) {
                    removed.push(bp.clone());
                }
            }
            if !removed.is_empty() {
                removed_params.insert(name.clone(), removed);
            }
        } else {
            // Function removed entirely — flag
            issues.push(format!("Function `{}` removed; consider migration plan", name));
        }
    }

    if removed_params.is_empty() && reduced_arity.is_empty() {
        return issues;
    }

    let code = new_code;
    // Scan for function calls by name (simple heuristic)
    for (fname, removed) in removed_params.iter() {
        let mut pos = 0usize;
        let mut seen_named = Vec::new();
        while let Some(idx) = code[pos..].find(fname) {
            let i = pos + idx;
            // ensure word boundary and next char is '('
            if i > 0 {
                let prev = code.as_bytes()[i - 1] as char;
                if prev.is_ascii_alphanumeric() || prev == '_' {
                    pos = i + fname.len();
                    continue;
                }
            }
            let after = i + fname.len();
            let tail = &code[after..];
            let open = tail.find('(');
            if let Some(op) = open {
                let j = after + op + 1; // position after '('
                let mut depth = 1i32;
                let mut end = j;
                while end < code.len() {
                    let ch = code.as_bytes()[end] as char;
                    if ch == '(' {
                        depth += 1;
                    } else if ch == ')' {
                        depth -= 1;
                        if depth == 0 {
                            break;
                        }
                    }
                    end += 1;
                }
                if depth == 0 && end <= code.len() {
                    let args_slice = &code[j..end];
                    let (_argc, named) = analyze_args_slice(args_slice);
                    for r in removed {
                        let needle = format!("{}=", r);
                        if args_slice.contains(&needle) || named.iter().any(|n| n == r) {
                            seen_named.push(r.clone());
                        }
                    }
                    pos = end + 1;
                    continue;
                }
            }
            pos = after;
        }
        if !seen_named.is_empty() {
            issues.push(format!(
                "Calls to `{fname}` still pass removed named params: {}",
                seen_named.join(", ")
            ));
        }
    }

    for (fname, (old_n, new_n)) in reduced_arity.iter() {
        let mut pos = 0usize;
        let mut offending = 0usize;
        while let Some(idx) = code[pos..].find(fname) {
            let i = pos + idx;
            if i > 0 {
                let prev = code.as_bytes()[i - 1] as char;
                if prev.is_ascii_alphanumeric() || prev == '_' {
                    pos = i + fname.len();
                    continue;
                }
            }
            let after = i + fname.len();
            let tail = &code[after..];
            if let Some(op) = tail.find('(') {
                let j = after + op + 1;
                let mut depth = 1i32;
                let mut end = j;
                while end < code.len() {
                    let ch = code.as_bytes()[end] as char;
                    if ch == '(' {
                        depth += 1;
                    } else if ch == ')' {
                        depth -= 1;
                        if depth == 0 {
                            break;
                        }
                    }
                    end += 1;
                }
                if depth == 0 {
                    let args_slice = &code[j..end];
                    let (argc, _) = analyze_args_slice(args_slice);
                    if argc > *new_n {
                        offending += 1;
                    }
                    pos = end + 1;
                    continue;
                }
            }
            pos = after;
        }
        if offending > 0 {
            issues.push(format!(
                "Calls to `{}` exceed new arity ({} -> {}): {} occurrences",
                fname, old_n, new_n, offending
            ));
        }
    }

    issues
}

fn extract_old_new_contents(hook_input: &HookInput) -> (String, Option<String>, Option<String>) {
    let file_path = extract_file_path(&hook_input.tool_input);
    match hook_input.tool_name.as_str() {
        "Edit" => {
            let old = hook_input
                .tool_input
                .get("old_string")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            let new = hook_input
                .tool_input
                .get("new_string")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            (file_path, old, new)
        }
        "MultiEdit" => {
            if let Some(edits) = hook_input.tool_input.get("edits").and_then(|v| v.as_array()) {
                let mut old = String::new();
                let mut new = String::new();
                for e in edits.iter().take(1000) {
                    if let Some(o) = e.get("old_string").and_then(|v| v.as_str()) {
                        old.push_str(o);
                        old.push('\n');
                    }
                    if let Some(n) = e.get("new_string").and_then(|v| v.as_str()) {
                        new.push_str(n);
                        new.push('\n');
                    }
                }
                (
                    file_path,
                    if old.is_empty() { None } else { Some(old) },
                    if new.is_empty() { None } else { Some(new) },
                )
            } else {
                let old = std::fs::read_to_string(&file_path).ok();
                (file_path, old, None)
            }
        }
        "Write" => {
            let old = std::fs::read_to_string(&file_path).ok();
            let new = hook_input
                .tool_input
                .get("content")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            (file_path, old, new)
        }
        _ => (file_path, None, None),
    }
}

// -----------------
// Unit tests (private) for contract heuristics
// -----------------
#[cfg(test)]
mod tests {
    use super::{
        contract_weakening_reasons, detect_function_stub, detect_return_constant, extract_signatures_regex,
    };
    use rust_validation_hooks::analysis::ast::SupportedLanguage;

    #[test]
    fn unit_contract_detects_python_param_reduction() {
        let old = "def f(a, b):\n    return a + b\n";
        let new = "def f(a):\n    return a\n";
        let reasons = contract_weakening_reasons(Some(SupportedLanguage::Python), old, new);
        assert!(!reasons.is_empty(), "expected contract weakening reasons");
        assert!(reasons.iter().any(|r| r.contains("parameter count reduced")));
    }

    #[test]
    fn unit_contract_preserves_js_same_signature() {
        let old = "function sum(a, b){ return a + b }\n";
        let new = "function sum(a, b){ const c = a + b; return c }\n";
        // сигнатуры одинаковые → пусто
        let reasons = contract_weakening_reasons(Some(SupportedLanguage::JavaScript), old, new);
        assert!(reasons.is_empty(), "no weakening expected for same signature");
        // sanity: разбор сигнатур видит два параметра
        let sig_old = extract_signatures_regex(Some(SupportedLanguage::JavaScript), old);
        assert_eq!(sig_old.get("sum").map(|v| v.len()), Some(2));
    }

    #[test]
    fn unit_detects_return_constant_python() {
        let code = "def f(x):\n    return 1\n";
        assert!(
            detect_return_constant(code),
            "should detect return literal in python"
        );
    }

    #[test]
    fn unit_detects_return_constant_js() {
        let code = "function f(){ return true; }";
        assert!(detect_return_constant(code), "should detect return literal in js");
    }

    #[test]
    fn unit_detects_function_stub_python_pass() {
        let code = "def f(x):\n    # TODO\n    pass\n";
        assert!(
            detect_function_stub(Some(SupportedLanguage::Python), code),
            "should detect python pass stub"
        );
    }

    #[test]
    fn unit_detects_function_stub_js_throw() {
        let code = "function f(){ throw new Error('Not implemented'); }";
        assert!(
            detect_function_stub(Some(SupportedLanguage::JavaScript), code),
            "should detect js throw Not implemented stub"
        );
    }
}

fn normalize_code_for_signal(code: &str) -> String {
    let mut s = String::new();
    let mut i = 0;
    let b = code.as_bytes();
    while i < b.len() {
        if i + 1 < b.len() && b[i] == b'/' && b[i + 1] == b'*' {
            i += 2;
            while i + 1 < b.len() && !(b[i] == b'*' && b[i + 1] == b'/') {
                i += 1;
            }
            if i + 1 < b.len() {
                i += 2;
            }
            continue;
        }
        if i + 1 < b.len() && b[i] == b'/' && b[i + 1] == b'/' {
            while i < b.len() && b[i] != b'\n' {
                i += 1;
            }
            continue;
        }
        if b[i] == b'#' {
            while i < b.len() && b[i] != b'\n' {
                i += 1;
            }
            continue;
        }
        s.push(b[i] as char);
        i += 1;
    }
    s.split_whitespace().collect::<Vec<_>>().join("")
}

fn detect_return_constant(code: &str) -> bool {
    use once_cell::sync::Lazy;
    // Match: return <literal>  where literal is number/bool/null/none or quoted
    // string.
    static RET_RE: Lazy<Result<regex::Regex, regex::Error>> = Lazy::new(|| {
        // Match return literal anywhere on the line (not only at start)
        regex::Regex::new(r#"(?i)\breturn\s+(?:\d+|true|false|null|none|\"[^\"]*\"|'[^']*')\s*(?:;|[,})]|$)"#)
    });
    // Match arrow functions that immediately return a literal:  => <literal>
    static ARROW_RE: Lazy<Result<regex::Regex, regex::Error>> = Lazy::new(|| {
        regex::Regex::new(r#"=>\s*(?:\d+|true|false|null|none|\"[^\"]*\"|'[^']*')\s*(?:[,)};]|$)"#)
    });
    if let Ok(re) = RET_RE.as_ref() {
        if re.is_match(code) {
            return true;
        }
    }
    if let Ok(re) = ARROW_RE.as_ref() {
        if re.is_match(code) {
            return true;
        }
    }
    false
}

fn detect_js_ts_function_stub(code: &str) -> bool {
    use regex::Regex;
    if let Ok(re) = Regex::new(
        r#"(?i)\bthrow\s+new\s+Error\s*\(\s*['\"][^'\"]*(not\s+implemented|todo)[^'\"]*['\"]\s*\)"#,
    ) {
        if re.is_match(code) {
            return true;
        }
    }
    if let Ok(re) =
        Regex::new(r#"(?s)function\s+[A-Za-z_][A-Za-z0-9_]*\s*\([^)]*\)\s*\{\s*(/\*.*?\*/|//.*?\n|\s)*\}"#)
    {
        if re.is_match(code) {
            return true;
        }
    }
    if let Ok(re) =
        Regex::new(r#"(?s)^[ \t]*[A-Za-z_][A-Za-z0-9_]*\s*\([^)]*\)\s*\{\s*(/\*.*?\*/|//.*?\n|\s)*\}$"#)
    {
        if re.is_match(code) {
            return true;
        }
    }
    false
}

fn detect_python_function_stub(code: &str) -> bool {
    use regex::Regex;
    if let Ok(re) =
        Regex::new(r#"(?sm)^\s*def\s+[A-Za-z_][A-Za-z0-9_]*\s*\([^)]*\):\s*(?:#.*\n|\s)*pass\s*$"#)
    {
        if re.is_match(code) {
            return true;
        }
    }
    if let Ok(re) = Regex::new(
        r#"(?sm)^\s*def\s+[A-Za-z_][A-Za-z0-9_]*\s*\([^)]*\):\s*(?:#.*\n|\s)*raise\s+NotImplementedError\b[^\n]*$"#,
    ) {
        if re.is_match(code) {
            return true;
        }
    }
    false
}

fn detect_function_stub(language: Option<SupportedLanguage>, code: &str) -> bool {
    match language {
        Some(SupportedLanguage::Python) => detect_python_function_stub(code),
        Some(SupportedLanguage::JavaScript) | Some(SupportedLanguage::TypeScript) => {
            detect_js_ts_function_stub(code)
        }
        _ => false,
    }
}

#[inline]
fn detect_print_only(code: &str) -> bool {
    let low = code.to_ascii_lowercase();
    // Fast precheck: any known print/log patterns?
    const TOKENS: [&str; 5] = [
        "console.log(",
        "print(",
        "logger.",
        "system.out.println(",
        "debug(",
    ];
    if !TOKENS.iter().any(|t| low.contains(t)) {
        return false;
    }
    // Count alphanumeric characters while skipping known tokens to avoid allocation
    let bytes = low.as_bytes();
    let mut i = 0usize;
    let mut alnum = 0usize;
    while i < bytes.len() {
        // Try to skip a known token at current offset
        let mut skipped = false;
        for t in TOKENS {
            let tb = t.as_bytes();
            if i + tb.len() <= bytes.len() && &bytes[i..i + tb.len()] == tb {
                i += tb.len();
                skipped = true;
                break;
            }
        }
        if skipped {
            continue;
        }
        let c = bytes[i] as char;
        if c.is_ascii_alphanumeric() {
            alnum += 1;
        }
        i += 1;
    }
    alnum < 6
}

fn detect_todo_placeholder(code: &str) -> bool {
    let low = code.to_ascii_lowercase();
    low.contains("todo") || low.contains("fixme") || low.contains("notimplemented") || low.contains("pass\n")
}

fn detect_empty_catch_except(code: &str) -> bool {
    let low = code.to_ascii_lowercase();

    // Удаляем все пробельные символы для точной проверки
    let compact: String = low.chars().filter(|c| !c.is_whitespace()).collect();

    // Java/C#/JS: catch(любое_исключение){} или catch(любое_исключение){}
    // Проверяем есть ли catch блок с пустым телом
    if compact.contains("catch(") {
        // Ищем паттерн catch(...){} где внутри {} нет кода
        if let Some(catch_pos) = compact.find("catch(") {
            if let Some(open_brace) = compact[catch_pos..].find("){") {
                let after_brace = catch_pos + open_brace + 2;
                if after_brace < compact.len() && compact.chars().nth(after_brace) == Some('}') {
                    return true;
                }
            }
        }
    }

    // Python: except: pass
    if low.contains("except") && low.contains(":") {
        let except_pass = low.replace('\n', " ").replace('\t', " ");
        if except_pass.contains("except:") && except_pass.contains("pass") {
            // Проверяем что pass идёт сразу после except:
            if let Some(except_pos) = except_pass.find("except:") {
                let after_except = &except_pass[except_pos + 7..].trim();
                if after_except.starts_with("pass") {
                    return true;
                }
            }
        }
    }

    // JS/TS: Promise .catch(() => {}) or .catch(function(){})
    if compact.contains(".catch(()=>{})") || compact.contains(".catch(function(){})") {
        return true;
    }

    false
}

fn detect_silent_result_discard(old: &str, new: &str) -> bool {
    // Go: проверяем изменение с обработки ошибки на _ = func()
    let old_low = old.to_ascii_lowercase();
    let new_low = new.to_ascii_lowercase();

    // Удаляем пробелы для точного сравнения
    let old_compact: String = old_low.chars().filter(|c| !c.is_whitespace()).collect();
    let new_compact: String = new_low.chars().filter(|c| !c.is_whitespace()).collect();

    // Go: было if err := func(); err != nil { ... }, стало _ = func()
    if old_compact.contains("iferr:=") && old_compact.contains("err!=nil") {
        if new_compact.contains("_=") && !new_compact.contains("iferr:=") {
            return true;
        }
    }

    false
}

fn assess_change(hook_input: &HookInput) -> HeuristicAssessment {
    let (_path, old_opt, new_opt) = extract_old_new_contents(hook_input);
    let mut assess = HeuristicAssessment::default();
    if let (Some(old), Some(new)) = (old_opt.as_ref(), new_opt.as_ref()) {
        let norm_old = normalize_code_for_signal(old);
        let norm_new = normalize_code_for_signal(new);
        if norm_old == norm_new {
            assess.is_whitespace_or_comments_only = true;
        }
    }
    let combined_old = old_opt.clone().unwrap_or_default();
    let combined_new = new_opt.clone().unwrap_or_default();
    assess.is_return_constant_only = detect_return_constant(&combined_new);
    assess.is_print_or_log_only = detect_print_only(&combined_new);
    // NOTE: we no дольше блокируем TODO/FIXME как таковые; оставляем для сводки
    assess.has_todo_or_placeholder = detect_todo_placeholder(&combined_new);
    assess.has_empty_catch_or_except = detect_empty_catch_except(&combined_new);
    // Проверяем молчаливый discard результатов (сравнение старого и нового кода)
    if old_opt.is_some() && new_opt.is_some() {
        assess.has_silent_result_discard = detect_silent_result_discard(&combined_old, &combined_new);
    }
    // New, minimal file creation (do not block for simple stubs like print("ok"))
    if old_opt.is_none() && new_opt.is_some() {
        let norm = normalize_code_for_signal(&combined_new);
        assess.is_new_file_minimal = combined_new.trim().len() <= 64 || norm.len() <= 32;
    }
    // Allowed minimal context: version/health/ping endpoints or files
    let file_path = extract_file_path(&hook_input.tool_input).to_ascii_lowercase();
    let fname_ok =
        file_path.contains("version") || file_path.contains("health") || file_path.contains("ping");
    assess.is_allowed_minimal_context = fname_ok;

    // Stronger signals
    assess.const_return_ignores_params = detect_const_return_ignores_params(&combined_new);
    assess.logic_calls_removed = {
        let old_calls = old_opt.as_ref().map(|s| count_call_like_tokens(s)).unwrap_or(0);
        let new_calls = count_call_like_tokens(&combined_new);
        old_calls >= 3 && new_calls == 0 && (assess.is_return_constant_only || assess.is_print_or_log_only)
    };
    let mut parts = Vec::new();
    if assess.is_whitespace_or_comments_only {
        parts.push("no-op change (whitespace/comments only)".to_string());
    }
    if assess.is_return_constant_only {
        parts.push("returns constant only".to_string());
    }
    if assess.is_print_or_log_only {
        parts.push("print/log only".to_string());
    }
    if assess.const_return_ignores_params {
        parts.push("constant return ignores params".to_string());
    }
    if assess.logic_calls_removed {
        parts.push("logic/calls removed".to_string());
    }
    if assess.has_todo_or_placeholder {
        parts.push("TODO present".to_string());
    }
    if assess.has_empty_catch_or_except {
        parts.push("empty catch/except".to_string());
    }
    if assess.has_silent_result_discard {
        parts.push("silent result discard".to_string());
    }
    if assess.is_new_file_minimal {
        parts.push("new minimal file".to_string());
    }
    assess.summary = if parts.is_empty() {
        "no red flags".to_string()
    } else {
        parts.join(", ")
    };
    assess
}

// Heuristic: constant return while parameters exist and are unused
fn detect_const_return_ignores_params(code: &str) -> bool {
    use regex::Regex;
    // Python: def f(a,b): ... return <lit>
    if let Ok(re) = Regex::new(
        r#"(?s)def\s+[A-Za-z_][A-Za-z0-9_]*\s*\(([^)]*[^\s)])\)\s*:\s*(?:#.*\n|\s)*return\s+(?:\d+|True|False|None|"[^"]*"|'[^']*')\b"#,
    ) {
        if let Some(cap) = re.captures(code) {
            let params = cap.get(1).map(|m| m.as_str()).unwrap_or("");
            let body_start = cap.get(0).map(|m| m.end()).unwrap_or(0);
            let body = &code[body_start.saturating_sub(80)..body_start]; // small window is enough
            let names: Vec<&str> = params
                .split(',')
                .map(|p| p.trim())
                .filter(|p| !p.is_empty())
                .collect();
            if !names.is_empty() && !names.iter().any(|n| body.contains(n)) {
                return true;
            }
        }
    }
    // JS/TS: function f(a,b){ return <lit>; } or (a,b)=> <lit>
    if let Ok(re) = Regex::new(
        r#"(?s)function\s+[A-Za-z_][A-Za-z0-9_]*\s*\(([^)]*[^\s)])\)\s*\{\s*return\s+(?:\d+|true|false|null|"[^"]*"|'[^']*')\s*;?\s*\}"#,
    ) {
        if let Some(cap) = re.captures(code) {
            let params = cap.get(1).map(|m| m.as_str()).unwrap_or("");
            let body_start = cap.get(0).map(|m| m.end()).unwrap_or(0);
            let body = &code[body_start.saturating_sub(80)..body_start];
            let names: Vec<&str> = params
                .split(',')
                .map(|p| p.trim().trim_start_matches("..."))
                .filter(|p| !p.is_empty())
                .collect();
            if !names.is_empty() && !names.iter().any(|n| body.contains(n)) {
                return true;
            }
        }
    }
    if let Ok(re) = Regex::new(r#"(?s)\(([^)]*[^\s)])\)\s*=>\s*(?:\d+|true|false|null|"[^"]*"|'[^']*')\b"#) {
        if let Some(cap) = re.captures(code) {
            let params = cap.get(1).map(|m| m.as_str()).unwrap_or("");
            let names: Vec<&str> = params
                .split(',')
                .map(|p| p.trim().trim_start_matches("..."))
                .filter(|p| !p.is_empty())
                .collect();
            if !names.is_empty() {
                return true; // arrow literal return can’t reference params
                             // inline before =>
            }
        }
    }
    false
}

// Heuristic: approximate number of call-like tokens (identifier followed by
// '(')
fn count_call_like_tokens(s: &str) -> usize {
    let mut c = 0usize;
    let bytes = s.as_bytes();
    let mut i = 0usize;
    while i + 1 < bytes.len() {
        let ch = bytes[i] as char;
        if (ch.is_ascii_alphabetic() || ch == '_') {
            let mut j = i + 1;
            while j < bytes.len() {
                let cj = bytes[j] as char;
                if cj.is_ascii_alphanumeric() || cj == '_' {
                    j += 1;
                    continue;
                }
                break;
            }
            if j < bytes.len() && bytes[j] == b'(' {
                // Cheap filters to avoid keywords/defs
                let ident = &s[i..j].to_ascii_lowercase();
                if ident != "if"
                    && ident != "for"
                    && ident != "while"
                    && ident != "switch"
                    && ident != "return"
                    && ident != "function"
                    && ident != "def"
                    && ident != "class"
                {
                    c += 1;
                }
            }
            i = j;
            continue;
        }
        i += 1;
    }
    c
}

// Removed GrokSecurityClient - now using UniversalAIClient from ai_providers
// module

use std::path::PathBuf;

/// Validate path for security and ensure it's a directory
fn validate_prompts_path(path: &PathBuf) -> Option<PathBuf> {
    // Canonicalize handles path traversal, symlinks, and normalization
    match std::fs::canonicalize(path) {
        Ok(canonical) => {
            if canonical.is_dir() {
                Some(canonical)
            } else {
                None
            }
        }
        Err(e) => {
            tracing::warn!(error=%e, "Cannot validate prompts path");
            None
        }
    }
}

/// Get the prompts directory path - always next to executable
fn get_prompts_dir() -> PathBuf {
    // Always look for prompts directory next to executable
    let exe_path = match std::env::current_exe() {
        Ok(path) => path,
        Err(e) => {
            tracing::error!(error=%e, "Cannot determine executable path; falling back to ./prompts");
            return PathBuf::from("prompts");
        }
    };

    let parent = match exe_path.parent() {
        Some(parent) => parent,
        None => {
            tracing::error!("Cannot get parent directory of executable; falling back to ./prompts");
            return PathBuf::from("prompts");
        }
    };

    // Production scenario: prompts directory next to executable
    let prompts_path = parent.join("prompts");

    if let Some(validated) = validate_prompts_path(&prompts_path) {
        tracing::info!(dir=?validated, "Using prompts directory");
        return validated;
    }

    // Final fallback
    tracing::warn!("Prompts directory not found next to executable; using current directory");
    PathBuf::from("prompts")
}

/// Load prompt from file relative to prompts directory with security validation
fn load_prompt(prompt_file: &str) -> Result<String> {
    // Validate filename to prevent path traversal
    let path = std::path::Path::new(prompt_file);

    // Check for path traversal attempts
    if prompt_file.contains("..") || prompt_file.contains("/") || prompt_file.contains("\\") {
        anyhow::bail!(
            "Invalid prompt filename - must be a simple filename without path separators: {}",
            prompt_file
        );
    }

    // Additional validation: ensure it's just a filename
    let components: Vec<_> = path.components().collect();
    if components.len() != 1 || !matches!(components[0], std::path::Component::Normal(_)) {
        anyhow::bail!(
            "Invalid prompt filename - must be a simple filename: {}",
            prompt_file
        );
    }

    let prompt_path = get_prompts_dir().join(prompt_file);

    // Final validation: ensure the resolved path is within the prompts directory
    if let (Ok(canonical_prompt), Ok(canonical_dir)) = (
        std::fs::canonicalize(&prompt_path),
        std::fs::canonicalize(get_prompts_dir()),
    ) {
        if !canonical_prompt.starts_with(&canonical_dir) {
            anyhow::bail!("Security error: prompt file path escapes the prompts directory");
        }
    }

    std::fs::read_to_string(&prompt_path)
        .with_context(|| format!("Failed to read prompt file: {:?}", prompt_path))
}

/// Read and summarize transcript from JSONL file with current task
/// identification
fn read_transcript_summary(path: &str, max_messages: usize, _max_chars: usize) -> Result<String> {
    use std::io::BufRead;
    use std::io::BufReader;

    let file = File::open(path).context("Failed to open transcript file")?;
    let reader = BufReader::new(file);

    let mut all_messages = Vec::new();

    // Parse JSONL format - each line is a separate JSON object
    for line in reader.lines() {
        let line = line.context("Failed to read line from transcript")?;
        if line.trim().is_empty() {
            continue;
        }

        if let Ok(entry) = serde_json::from_str::<serde_json::Value>(&line) {
            // Extract message from the entry
            if let Some(msg) = entry.get("message") {
                // Handle different message formats
                if let Some(role) = msg.get("role").and_then(|v| v.as_str()) {
                    let content = if let Some(content_arr) = msg.get("content").and_then(|v| v.as_array()) {
                        // Handle content array (assistant messages)
                        content_arr
                            .iter()
                            .filter_map(|c| {
                                if let Some(text) = c.get("text").and_then(|v| v.as_str()) {
                                    Some(text.to_string())
                                } else {
                                    c.get("name")
                                        .and_then(|v| v.as_str())
                                        .map(|tool_name| format!("[Tool: {}]", tool_name))
                                }
                            })
                            .collect::<Vec<_>>()
                            .join(" ")
                    } else if let Some(text) = msg.get("content").and_then(|v| v.as_str()) {
                        // Handle simple string content (user messages)
                        text.to_string()
                    } else {
                        String::new()
                    };

                    if !content.is_empty() {
                        all_messages.push((role.to_string(), content));
                    }
                }
            }
        }
    }

    // Find the last user message to identify current task
    let last_user_message = all_messages
        .iter()
        .rev()
        .find(|(role, _)| role == "user")
        .map(|(_, content)| content.clone());

    // Take last N messages (max 20)
    let max_msgs = max_messages.min(20);
    let start = if all_messages.len() > max_msgs {
        all_messages.len() - max_msgs
    } else {
        0
    };

    let mut result = String::new();
    let mut char_count = 0;

    // Add current task context at the beginning
    if let Some(current_task) = &last_user_message {
        let task_truncated = if current_task.chars().count() > 150 {
            let truncated_chars: String = current_task.chars().take(147).collect();
            format!("{}...", truncated_chars)
        } else {
            current_task.clone()
        };

        let task_str = format!("CURRENT USER TASK: {}\n\nRECENT CONVERSATION:\n", task_truncated);
        result.push_str(&task_str);
        char_count += task_str.len();
    }

    for (role, content) in all_messages[start..].iter() {
        // Truncate individual messages to 100 chars (UTF-8 safe)
        let truncated = if content.chars().count() > 100 {
            let truncated_chars: String = content.chars().take(97).collect();
            format!("{}...", truncated_chars)
        } else {
            content.clone()
        };

        // Mark the last user message as current task
        let msg_str = if role == "user" && Some(content) == last_user_message.as_ref() {
            format!("[{}] (CURRENT TASK): {}\n", role, truncated)
        } else {
            format!("[{}]: {}\n", role, truncated)
        };

        // Stop if we exceed 2000 chars
        if char_count + msg_str.len() > 2000 {
            result.push_str("...\n");
            break;
        }

        result.push_str(&msg_str);
        char_count += msg_str.len();
    }

    Ok(result)
}

// File structure checking function removed - AI handles all validation

/// Build comprehensive error chain from an error
fn build_error_chain(error: &dyn std::error::Error) -> Vec<String> {
    const MAX_DEPTH: usize = 10;
    const MAX_ERROR_LENGTH: usize = 500;

    let mut error_chain = Vec::new();
    let mut current_error = error;

    // Add the main error
    let main_error = current_error.to_string();
    let truncated = if main_error.len() > MAX_ERROR_LENGTH {
        format!("{}... (truncated)", &main_error[..MAX_ERROR_LENGTH])
    } else {
        main_error
    };
    error_chain.push(truncated);

    // Walk the error chain
    let mut depth = 0;
    while let Some(source) = current_error.source() {
        let source_str = source.to_string();
        let truncated = if source_str.len() > MAX_ERROR_LENGTH {
            format!("{}... (truncated)", &source_str[..MAX_ERROR_LENGTH])
        } else {
            source_str
        };
        error_chain.push(truncated);
        current_error = source;
        depth += 1;

        if depth >= MAX_DEPTH {
            error_chain.push("...error chain truncated (too deep)...".to_string());
            break;
        }
    }

    error_chain
}

/// Format error chain into a comprehensive message
fn format_error_message(error_chain: &[String]) -> String {
    if error_chain.is_empty() {
        return "Unknown error occurred".to_string();
    }

    if error_chain.len() == 1 {
        error_chain[0].clone()
    } else {
        // Format as hierarchical error message
        let mut message = error_chain[0].clone();
        if error_chain.len() > 1 {
            message.push_str("\nDetails: ");
            message.push_str(&error_chain[1..].join(" <- "));
        }
        message
    }
}

/// Safely escape string for JSON output
fn escape_json_string(s: &str) -> String {
    let mut result = String::with_capacity(s.len() * 2);

    for ch in s.chars() {
        match ch {
            '"' => result.push_str("\\\""),
            '\\' => result.push_str("\\\\"),
            '\n' => result.push_str("\\n"),
            '\r' => result.push_str("\\r"),
            '\t' => result.push_str("\\t"),
            '\u{0008}' => result.push_str("\\b"),
            '\u{000C}' => result.push_str("\\f"),
            c if c.is_control() => {
                // Escape other control characters as Unicode
                result.push_str(&format!("\\u{:04x}", c as u32));
            }
            c => result.push(c),
        }
    }

    result
}

/// Format quality enforcement message for denied code
fn format_quality_message(reason: &str) -> String {
    // Replace literal \n with actual newlines from AI response
    let cleaned_reason = reason.replace("\\n", "\n");

    format!(
        "РЕАЛИЗУЙТЕ КОД С КАЧЕСТВОМ, А НЕ ПРОСТО ЧТОБЫ ЗАВЕРШИТЬ ЗАДАЧУ\n[плохой и поддельный код всегда будет заблокирован]\n\nвыявленные проблемы в ваших изменениях:\n{}\n\nУЛУЧШИТЕ СВОЮ РАБОТУ — не убегайте от проблем, создавая минимальные упрощённые реализации\n[попытки сделать это также будут заблокированы]",
        cleaned_reason
    )
}

/// Output error response with proper fallback handling
fn output_error_response(error: &anyhow::Error) {
    // Build and log error chain
    let error_chain = build_error_chain(&**error);

    tracing::error!("PreToolUse validation error");
    tracing::debug!(?error, "Detailed debug error");
    tracing::error!(%error, "Display error");
    tracing::error!(depth = error_chain.len(), "Error chain depth");
    for (i, err) in error_chain.iter().enumerate() {
        tracing::error!(level = i, %err, "Error chain element");
    }

    // Format comprehensive error message
    let error_message = format_error_message(&error_chain);
    tracing::error!(message=%error_message, "Final error message");

    // Create output structure
    let output = PreToolUseOutput {
        hook_specific_output: PreToolUseHookOutput {
            hook_event_name: "PreToolUse".to_string(),
            permission_decision: "deny".to_string(),
            permission_decision_reason: Some(error_message.clone()),
        },
    };

    // Try to serialize normally
    match serde_json::to_string(&output) {
        Ok(json) => {
            println!("{}", json);
        }
        Err(ser_err) => {
            // Fallback with manual JSON construction
            tracing::error!(error=%ser_err, "Serialization failed for PreToolUse output");
            let escaped = escape_json_string(&error_message);
            println!(
                r#"{{"hook_specific_output":{{"hook_event_name":"PreToolUse","permission_decision":"deny","permission_decision_reason":"{}"}}}}"#,
                escaped
            );
        }
    }
}

/// Main PreToolUse hook execution
#[tokio::main]
async fn main() -> Result<()> {
    // Defensive: log any unexpected panic as a structured error
    std::panic::set_hook(Box::new(|info| {
        rust_validation_hooks::telemetry::init();
        tracing::error!(panic=%info, "panic in pretooluse");
    }));
    // Initialize structured logging (stderr). Safe to call multiple times.
    rust_validation_hooks::telemetry::init();
    // Optional offline AST-only mode (no network): decide allow/deny from local
    // AST/security heuristics
    if dev_flag_enabled("PRETOOL_AST_ONLY") {
        // Read input from stdin
        let mut input = String::new();
        io::stdin()
            .read_to_string(&mut input)
            .context("Failed to read stdin")?;
        let hook_input: HookInput = serde_json::from_str(&input).context("Failed to parse hook input")?;

        // Extract file path and content to analyze
        let file_path = extract_file_path(&hook_input.tool_input);
        let language = file_path
            .split('.')
            .next_back()
            .and_then(SupportedLanguage::from_extension);

        let code: String = match hook_input.tool_name.as_str() {
            "Write" => hook_input
                .tool_input
                .get("content")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
                .or_else(|| std::fs::read_to_string(&file_path).ok())
                .unwrap_or_default(),
            "Edit" => hook_input
                .tool_input
                .get("new_string")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
                .unwrap_or_else(|| std::fs::read_to_string(&file_path).unwrap_or_default()),
            "MultiEdit" => {
                if let Some(edits) = hook_input.tool_input.get("edits").and_then(|v| v.as_array()) {
                    let mut buf = String::new();
                    for e in edits.iter().take(1000) {
                        if let Some(ns) = e.get("new_string").and_then(|v| v.as_str()) {
                            buf.push_str(ns);
                            buf.push('\n');
                        }
                    }
                    buf
                } else {
                    std::fs::read_to_string(&file_path).unwrap_or_default()
                }
            }
            _ => String::new(),
        };

        // Default to allow if no code/language
        if code.trim().is_empty() || language.is_none() {
            let output = PreToolUseOutput {
                hook_specific_output: PreToolUseHookOutput {
                    hook_event_name: "PreToolUse".to_string(),
                    permission_decision: "allow".to_string(),
                    permission_decision_reason: None,
                },
            };
            println!(
                "{}",
                serde_json::to_string(&output).context("Failed to serialize output")?
            );
            return Ok(());
        }

        // Heuristic assessment for AST-only path
        let heur = {
            // Build a minimal HookInput clone to reuse helper
            assess_change(&hook_input)
        };

        // Structural sanity: fast syntax check where available
        if let Some(lang) = language {
            if let Err(e) = MultiLanguageAnalyzer::analyze_with_tree_sitter_timeout(
                &code,
                lang,
                std::time::Duration::from_millis(800),
            ) {
                // Treat parse/syntax errors as structural harm
                let reason = format!("Structural integrity check failed: {e}");
                let output = PreToolUseOutput {
                    hook_specific_output: PreToolUseHookOutput {
                        hook_event_name: "PreToolUse".to_string(),
                        permission_decision: "deny".to_string(),
                        permission_decision_reason: Some(format_quality_message(&reason)),
                    },
                };
                println!(
                    "{}",
                    serde_json::to_string(&output).context("Failed to serialize output")?
                );
                return Ok(());
            }
        }

        // If WRITE is a no-op (old == new ignoring whitespace/comments), allow
        if hook_input.tool_name == "Write" {
            let (_p, old_opt, new_opt) = extract_old_new_contents(&hook_input);
            if let (Some(o), Some(n)) = (old_opt, new_opt) {
                if normalize_code_for_signal(&o) == normalize_code_for_signal(&n) {
                    // If dangerous patterns present, do not allow silently
                    let low = n.to_ascii_lowercase();
                    let has_creds = low.contains("password")
                        || low.contains("secret")
                        || low.contains("api_key")
                        || low.contains("token");
                    let has_sql = (low.contains("select") && low.contains("where"))
                        || (low.contains("insert") && low.contains("values"))
                        || (low.contains("update") && low.contains("set"))
                        || (low.contains("delete") && low.contains("from"));
                    if !(has_creds || has_sql) {
                        let output = PreToolUseOutput {
                            hook_specific_output: PreToolUseHookOutput {
                                hook_event_name: "PreToolUse".to_string(),
                                permission_decision: "allow".to_string(),
                                permission_decision_reason: None,
                            },
                        };
                        println!(
                            "{}",
                            serde_json::to_string(&output).context("Failed to serialize output")?
                        );
                        return Ok(());
                    }
                }
            }
        }

        if
        /* do not block TODO/FIXME */
        heur.has_empty_catch_or_except
            || heur.has_silent_result_discard
            || ((heur.is_return_constant_only || heur.is_print_or_log_only) && !heur.is_new_file_minimal)
            || (detect_function_stub(language, &code) && hook_input.tool_name != "Write")
        {
            let reason = format!("Anti-cheating: {0}", heur.summary);
            let output = PreToolUseOutput {
                hook_specific_output: PreToolUseHookOutput {
                    hook_event_name: "PreToolUse".to_string(),
                    permission_decision: "deny".to_string(),
                    permission_decision_reason: Some(format_quality_message(&reason)),
                },
            };
            println!(
                "{}",
                serde_json::to_string(&output).context("Failed to serialize output")?
            );
            return Ok(());
        }

        // If EDIT is a no-op (old == new ignoring whitespace/comments), soft-deny
        // (converted to deny)
        if hook_input.tool_name == "Edit" {
            let (_p, old_opt, new_opt) = extract_old_new_contents(&hook_input);
            if let (Some(o), Some(n)) = (old_opt, new_opt) {
                if normalize_code_for_signal(&o) == normalize_code_for_signal(&n) {
                    let reason = "No-op change (whitespace/comments only)";
                    let output = PreToolUseOutput {
                        hook_specific_output: PreToolUseHookOutput {
                            hook_event_name: "PreToolUse".to_string(),
                            permission_decision: "deny".to_string(),
                            permission_decision_reason: Some(format_quality_message(reason)),
                        },
                    };
                    println!(
                        "{}",
                        serde_json::to_string(&output).context("Failed to serialize output")?
                    );
                    return Ok(());
                }
            }
        }

        // API contract weakening heuristic (deny in AST-only mode)
        if hook_input.tool_name == "Edit" || hook_input.tool_name == "MultiEdit" {
            let (path, old_opt, new_opt) = extract_old_new_contents(&hook_input);
            if let (Some(old), Some(new)) = (old_opt, new_opt) {
                let lang = path
                    .split('.')
                    .next_back()
                    .and_then(SupportedLanguage::from_extension);
                let reasons = contract_weakening_reasons(lang, &old, &new);
                if !reasons.is_empty() {
                    let reason = format!("API contract weakening detected:\n{}", reasons.join("\n"));
                    let output = PreToolUseOutput {
                        hook_specific_output: PreToolUseHookOutput {
                            hook_event_name: "PreToolUse".to_string(),
                            permission_decision: "deny".to_string(),
                            permission_decision_reason: Some(format_quality_message(&reason)),
                        },
                    };
                    println!(
                        "{}",
                        serde_json::to_string(&output).context("Failed to serialize output")?
                    );
                    return Ok(());
                }
            }
        }

        // Analyze with AST quality scorer
        let scorer = analysis::ast::quality_scorer::AstQualityScorer::new();
        // Avoid unwrap: re-check language presence defensively
        let language = match language {
            Some(l) => l,
            None => {
                let output = PreToolUseOutput {
                    hook_specific_output: PreToolUseHookOutput {
                        hook_event_name: "PreToolUse".to_string(),
                        permission_decision: "allow".to_string(),
                        permission_decision_reason: None,
                    },
                };
                println!(
                    "{}",
                    serde_json::to_string(&output).context("Failed to serialize output")?
                );
                return Ok(());
            }
        };
        let score =
            scorer
                .analyze(&code, language)
                .unwrap_or_else(|_| analysis::ast::quality_scorer::QualityScore {
                    total_score: 1000,
                    functionality_score: 300,
                    reliability_score: 200,
                    maintainability_score: 200,
                    performance_score: 150,
                    security_score: 100,
                    standards_score: 50,
                    concrete_issues: vec![],
                });

        // Load runtime config (sensitivity, ignore globs, environment)
        let cfg = config::load_config();

        // Decide: deny on security signals based on sensitivity and context
        use analysis::ast::quality_scorer::{IssueCategory, IssueSeverity};
        let mut deny_reasons: Vec<String> = Vec::new();
        for i in &score.concrete_issues {
            // Do not block on unfinished work markers (TODO/FIXME/etc.) per policy
            if matches!(i.category, IssueCategory::UnfinishedWork) {
                continue;
            }
            // Skip ignored files
            if config::should_ignore_path(&cfg, &file_path) {
                continue;
            }

            // Determine effective severity threshold by sensitivity
            let min_sev = match cfg.sensitivity {
                config::Sensitivity::Low => IssueSeverity::Critical,
                config::Sensitivity::Medium => IssueSeverity::Major,
                config::Sensitivity::High => IssueSeverity::Minor,
            };

            // In test context, relax creds if allowlisted variables present in code
            let is_test_ctx = config::is_test_context(&cfg, &file_path);
            let allowlisted = config::code_contains_allowlisted_vars(&cfg, &code);

            // Decide if issue should trigger deny
            let mut triggers = false;
            if i.severity as u8 <= min_sev as u8 {
                // Severity ordering via discriminant: Critical(0) < Major(1) < Minor(2)
                triggers = true;
            }
            // Always treat certain categories as critical triggers regardless of
            // sensitivity
            if matches!(
                i.category,
                IssueCategory::CommandInjection | IssueCategory::PathTraversal
            ) {
                triggers = true;
            }

            // Relax hardcoded creds in test context with allowlisted vars
            if is_test_ctx && allowlisted && matches!(i.category, IssueCategory::HardcodedCredentials) {
                triggers = false;
            }

            if triggers {
                deny_reasons.push(format!("Line {}: {} [{}]", i.line, i.message, i.rule_id));
            }
        }

        let (permission_decision, permission_decision_reason) = if !deny_reasons.is_empty() {
            (
                "deny".to_string(),
                Some(format_quality_message(&deny_reasons.join("\n"))),
            )
        } else {
            ("allow".to_string(), None)
        };

        let output = PreToolUseOutput {
            hook_specific_output: PreToolUseHookOutput {
                hook_event_name: "PreToolUse".to_string(),
                permission_decision,
                permission_decision_reason,
            },
        };
        println!(
            "{}",
            serde_json::to_string(&output).context("Failed to serialize output")?
        );
        return Ok(());
    }

    // Load configuration (env/.env next to executable has priority)
    let config = Config::from_env_graceful().context("Failed to load configuration")?;

    // Read input from stdin
    let mut input = String::new();
    io::stdin()
        .read_to_string(&mut input)
        .context("Failed to read stdin")?;

    // Parse hook input
    let hook_input: HookInput = serde_json::from_str(&input).context("Failed to parse hook input")?;

    // Debug logging (to file) is disabled in release builds
    if cfg!(debug_assertions) {
        let log_file_path = std::env::temp_dir().join("pretooluse_debug.log");
        if let Ok(mut log_file) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log_file_path)
        {
            use std::io::Write;
            let timestamp = chrono::Local::now().format("%Y-%m-%d %H:%M:%S");
            writeln!(log_file, "\n=== PreToolUse Hook Debug [{}] ===", timestamp).ok();
            writeln!(log_file, "Tool name: {}", hook_input.tool_name).ok();
            writeln!(log_file, "Session ID: {:?}", hook_input.session_id).ok();
            writeln!(log_file, "Transcript path: {:?}", hook_input.transcript_path).ok();
            writeln!(log_file, "CWD: {:?}", hook_input.cwd).ok();
            writeln!(log_file, "Hook event: {:?}", hook_input.hook_event_name).ok();
            writeln!(
                log_file,
                "CLAUDE_PROJECT_DIR env: {:?}",
                std::env::var("CLAUDE_PROJECT_DIR").ok()
            )
            .ok();

            if let Some(transcript_path) = &hook_input.transcript_path {
                writeln!(
                    log_file,
                    "Attempting to read transcript from: {}",
                    transcript_path
                )
                .ok();
                match read_transcript_summary(transcript_path, 15, 1500) {
                    Ok(summary) => {
                        writeln!(log_file, "Transcript content (last 15 msgs, max 1500 chars):").ok();
                        writeln!(log_file, "{}", summary).ok();
                    }
                    Err(e) => {
                        let _ = writeln!(log_file, "Could not read transcript: {}", e);
                    }
                }
            }
            writeln!(log_file, "==============================").ok();
        }
        tracing::info!(path=?log_file_path, "PreToolUse hook: decision logged");
    }

    // Extract content and file path
    let content = extract_content_from_tool_input(&hook_input.tool_name, &hook_input.tool_input);
    let file_path = extract_file_path(&hook_input.tool_input);

    // Check project structure for Write operations (not Edit/MultiEdit)
    if hook_input.tool_name == "Write" && !file_path.is_empty() {
        // Get transcript context for checking user intent
        let _transcript_context = if let Some(transcript_path) = &hook_input.transcript_path {
            read_transcript_summary(transcript_path, 5, 500).ok()
        } else {
            None
        };

        // File structure checking removed - AI handles all validation now
    }

    // Test file validation removed - AI handles all validation now

    // All operations now go through AI validation - no automatic allows

    // All file validation now handled by AI - no automatic skipping based on file
    // extensions

    // If no API key for selected provider, fall back to offline AST-based decision
    // path
    if config
        .get_api_key_for_provider(&config.pretool_provider)
        .is_empty()
    {
        tracing::warn!(provider=%config.pretool_provider, "No API key; falling back to AST-only validation");
        // Determine language from file extension
        let language = file_path
            .split('.')
            .next_back()
            .and_then(SupportedLanguage::from_extension);

        // Default to allow if no code/language
        if content.trim().is_empty() || language.is_none() {
            let output = PreToolUseOutput {
                hook_specific_output: PreToolUseHookOutput {
                    hook_event_name: "PreToolUse".to_string(),
                    permission_decision: "allow".to_string(),
                    permission_decision_reason: None,
                },
            };
            println!(
                "{}",
                serde_json::to_string(&output).context("Failed to serialize output")?
            );
            return Ok(());
        }

        // Heuristic assessment
        let heur = assess_change(&hook_input);

        // Structural sanity: fast syntax check
        if let Some(lang) = language {
            if let Err(e) = MultiLanguageAnalyzer::analyze_with_tree_sitter_timeout(
                &content,
                lang,
                std::time::Duration::from_millis(800),
            ) {
                let reason = format!("Structural integrity check failed: {e}");
                let output = PreToolUseOutput {
                    hook_specific_output: PreToolUseHookOutput {
                        hook_event_name: "PreToolUse".to_string(),
                        permission_decision: "deny".to_string(),
                        permission_decision_reason: Some(format_quality_message(&reason)),
                    },
                };
                println!(
                    "{}",
                    serde_json::to_string(&output).context("Failed to serialize output")?
                );
                return Ok(());
            }
        }

        // If WRITE is a no-op (old == new ignoring whitespace/comments), allow unless
        // dangerous patterns
        if hook_input.tool_name == "Write" {
            let (_p, old_opt, new_opt) = extract_old_new_contents(&hook_input);
            if let (Some(o), Some(n)) = (old_opt, new_opt) {
                if normalize_code_for_signal(&o) == normalize_code_for_signal(&n) {
                    let low = n.to_ascii_lowercase();
                    let has_creds = low.contains("password")
                        || low.contains("secret")
                        || low.contains("api_key")
                        || low.contains("token");
                    let has_sql = (low.contains("select") && low.contains("where"))
                        || (low.contains("insert") && low.contains("values"))
                        || (low.contains("update") && low.contains("set"))
                        || (low.contains("delete") && low.contains("from"));
                    if !(has_creds || has_sql) {
                        let output = PreToolUseOutput {
                            hook_specific_output: PreToolUseHookOutput {
                                hook_event_name: "PreToolUse".to_string(),
                                permission_decision: "allow".to_string(),
                                permission_decision_reason: None,
                            },
                        };
                        println!(
                            "{}",
                            serde_json::to_string(&output).context("Failed to serialize output")?
                        );
                        return Ok(());
                    }
                }
            }
        }

        // Block fake implementations
        if
        /* do not block TODO/FIXME */
        heur.has_empty_catch_or_except
            || heur.has_silent_result_discard
            || ((heur.is_return_constant_only || heur.is_print_or_log_only) && !heur.is_new_file_minimal)
            || (detect_function_stub(language, &content) && hook_input.tool_name != "Write")
        {
            let reason = format!("Anti-cheating: {0}", heur.summary);
            let output = PreToolUseOutput {
                hook_specific_output: PreToolUseHookOutput {
                    hook_event_name: "PreToolUse".to_string(),
                    permission_decision: "deny".to_string(),
                    permission_decision_reason: Some(format_quality_message(&reason)),
                },
            };
            println!(
                "{}",
                serde_json::to_string(&output).context("Failed to serialize output")?
            );
            return Ok(());
        }

        // No-op Edit → deny
        if hook_input.tool_name == "Edit" {
            let (_p, old_opt, new_opt) = extract_old_new_contents(&hook_input);
            if let (Some(o), Some(n)) = (old_opt, new_opt) {
                if normalize_code_for_signal(&o) == normalize_code_for_signal(&n) {
                    let reason = "No-op change (whitespace/comments only)";
                    let output = PreToolUseOutput {
                        hook_specific_output: PreToolUseHookOutput {
                            hook_event_name: "PreToolUse".to_string(),
                            permission_decision: "deny".to_string(),
                            permission_decision_reason: Some(format_quality_message(reason)),
                        },
                    };
                    println!(
                        "{}",
                        serde_json::to_string(&output).context("Failed to serialize output")?
                    );
                    return Ok(());
                }
            }
        }

        // API contract weakening (deny)
        if hook_input.tool_name == "Edit" || hook_input.tool_name == "MultiEdit" {
            let (path, old_opt, new_opt) = extract_old_new_contents(&hook_input);
            if let (Some(old), Some(new)) = (old_opt, new_opt) {
                let lang = path
                    .split('.')
                    .next_back()
                    .and_then(SupportedLanguage::from_extension);
                let reasons = contract_weakening_reasons(lang, &old, &new);
                if !reasons.is_empty() {
                    let reason = format!("API contract weakening detected:\n{}", reasons.join("\n"));
                    let output = PreToolUseOutput {
                        hook_specific_output: PreToolUseHookOutput {
                            hook_event_name: "PreToolUse".to_string(),
                            permission_decision: "deny".to_string(),
                            permission_decision_reason: Some(format_quality_message(&reason)),
                        },
                    };
                    println!(
                        "{}",
                        serde_json::to_string(&output).context("Failed to serialize output")?
                    );
                    return Ok(());
                }
            }
        }

        // AST scoring
        let scorer = analysis::ast::quality_scorer::AstQualityScorer::new();
        let language = match language {
            Some(l) => l,
            None => {
                let output = PreToolUseOutput {
                    hook_specific_output: PreToolUseHookOutput {
                        hook_event_name: "PreToolUse".to_string(),
                        permission_decision: "allow".to_string(),
                        permission_decision_reason: None,
                    },
                };
                println!(
                    "{}",
                    serde_json::to_string(&output).context("Failed to serialize output")?
                );
                return Ok(());
            }
        };
        let score = scorer.analyze(&content, language).unwrap_or_else(|_| {
            analysis::ast::quality_scorer::QualityScore {
                total_score: 1000,
                functionality_score: 300,
                reliability_score: 200,
                maintainability_score: 200,
                performance_score: 150,
                security_score: 100,
                standards_score: 50,
                concrete_issues: vec![],
            }
        });

        // Policy evaluation
        let cfg = config::load_config();
        use analysis::ast::quality_scorer::{IssueCategory, IssueSeverity};
        let mut deny_reasons: Vec<String> = Vec::new();
        for i in &score.concrete_issues {
            if matches!(i.category, IssueCategory::UnfinishedWork) {
                continue;
            }
            if config::should_ignore_path(&cfg, &file_path) {
                continue;
            }
            let min_sev = match cfg.sensitivity {
                config::Sensitivity::Low => IssueSeverity::Critical,
                config::Sensitivity::Medium => IssueSeverity::Major,
                config::Sensitivity::High => IssueSeverity::Minor,
            };
            let is_test_ctx = config::is_test_context(&cfg, &file_path);
            let allowlisted = config::code_contains_allowlisted_vars(&cfg, &content);
            let mut triggers = i.severity as u8 <= min_sev as u8;
            if matches!(
                i.category,
                IssueCategory::SqlInjection | IssueCategory::CommandInjection | IssueCategory::PathTraversal
            ) {
                triggers = true;
            }
            if is_test_ctx && allowlisted && matches!(i.category, IssueCategory::HardcodedCredentials) {
                triggers = false;
            }
            if triggers {
                deny_reasons.push(format!("Line {}: {} [{}]", i.line, i.message, i.rule_id));
            }
        }
        let (decision, reason) = if deny_reasons.is_empty() {
            ("allow".to_string(), None)
        } else {
            (
                "deny".to_string(),
                Some(format_quality_message(&deny_reasons.join("\n"))),
            )
        };
        let output = PreToolUseOutput {
            hook_specific_output: PreToolUseHookOutput {
                hook_event_name: "PreToolUse".to_string(),
                permission_decision: decision,
                permission_decision_reason: reason,
            },
        };
        println!(
            "{}",
            serde_json::to_string(&output).context("Failed to serialize output")?
        );
        return Ok(());
    }

    // Perform security validation with context
    match perform_validation(&config, &content, &hook_input).await {
        Ok(validation) => {
            let (decision, reason) = match validation.decision.as_str() {
                "allow" => ("allow".to_string(), None),
                "deny" | "ask" => {
                    // Note: Claude Code hooks only support "allow" and "deny" decisions
                    // "ask" must be converted to "deny" with an informative message
                    if validation.decision == "ask" {
                        tracing::info!(
                            "'ask' decision converted to 'deny' (Claude Code only supports allow/deny)"
                        );
                    }
                    let formatted_reason = format_quality_message(&validation.reason);
                    ("deny".to_string(), Some(formatted_reason))
                }
                unknown => {
                    tracing::warn!(decision=%unknown, "Unknown validation decision; defaulting to deny for safety");
                    let formatted_reason =
                        format_quality_message(&format!("Unknown decision type: {unknown}"));
                    ("deny".to_string(), Some(formatted_reason))
                }
            };

            let output = PreToolUseOutput {
                hook_specific_output: PreToolUseHookOutput {
                    hook_event_name: "PreToolUse".to_string(),
                    permission_decision: decision,
                    permission_decision_reason: reason,
                },
            };
            println!(
                "{}",
                serde_json::to_string(&output).context("Failed to serialize output")?
            );
        }
        Err(e) => {
            output_error_response(&e);
        }
    }

    Ok(())
}

/// Format code changes as diff for better AI understanding
fn format_code_as_diff(hook_input: &HookInput) -> String {
    let mut diff = String::new();

    // Extract file path
    let file_path = extract_file_path(&hook_input.tool_input);

    match hook_input.tool_name.as_str() {
        "Edit" => {
            // Extract old_string and new_string from tool_input
            if let Some(old_string) = hook_input.tool_input.get("old_string").and_then(|v| v.as_str()) {
                if let Some(new_string) = hook_input.tool_input.get("new_string").and_then(|v| v.as_str()) {
                    // Try to read the current file content for context
                    let file_content = std::fs::read_to_string(&file_path).ok();
                    diff = format_edit_diff(
                        &file_path,
                        file_content.as_deref(),
                        old_string,
                        new_string,
                        3, // 3 lines of context
                    );
                }
            }
        }
        "MultiEdit" => {
            // Extract edits array from tool_input
            if let Some(edits_value) = hook_input.tool_input.get("edits") {
                if let Some(edits_array) = edits_value.as_array() {
                    let mut edits = Vec::new();
                    for edit in edits_array {
                        if let (Some(old), Some(new)) = (
                            edit.get("old_string").and_then(|v| v.as_str()),
                            edit.get("new_string").and_then(|v| v.as_str()),
                        ) {
                            edits.push((old.to_string(), new.to_string()));
                        }
                    }

                    // Try to read the current file content for context
                    let file_content = std::fs::read_to_string(&file_path).ok();

                    diff = format_multi_edit_diff(&file_path, file_content.as_deref(), &edits);
                }
            }
        }
        "Write" => {
            // For Write operations, show as new file creation
            if let Some(content) = hook_input.tool_input.get("content").and_then(|v| v.as_str()) {
                // Check if file exists
                let old_content = std::fs::read_to_string(&file_path).ok();

                diff = format_code_diff(
                    &file_path,
                    old_content.as_deref(),
                    Some(content),
                    3, // 3 lines of context
                );
            }
        }
        _ => {
            // For other operations, just show the content if available
            let content = extract_content_from_tool_input(&hook_input.tool_name, &hook_input.tool_input);
            if !content.is_empty() {
                diff = format!("Content:\n{content}");
            }
        }
    }

    diff
}

/// Perform security validation using Grok with context
async fn perform_validation(
    config: &Config,
    content: &str,
    hook_input: &HookInput,
) -> Result<SecurityValidation> {
    // Load security prompt and anti-patterns
    let mut prompt = load_prompt("edit_validation.txt").context("Failed to load edit validation prompt")?;

    // Load anti-patterns for comprehensive validation
    let anti_patterns = load_prompt("anti_patterns.txt").unwrap_or_else(|_| String::new());
    if !anti_patterns.is_empty() {
        prompt = format!("{prompt}\n\nANTI-PATTERNS REFERENCE:\n{anti_patterns}");
    }

    // Load language preference with fallback to RUSSIAN
    let language = load_prompt("language.txt")
        .unwrap_or_else(|_| "RUSSIAN".to_string())
        .trim()
        .to_string();

    // Extract file path and add it to context
    let file_path = extract_file_path(&hook_input.tool_input);
    if !file_path.is_empty() {
        prompt = format!("{prompt}\n\nFILE BEING MODIFIED: {file_path}");
    }

    // Format the code changes as diff for better AI understanding
    let diff_context = format_code_as_diff(hook_input);
    if !diff_context.is_empty() {
        prompt = format!("{prompt}\n\nCODE CHANGES (diff format):\n{diff_context}");
    }

    // Add heuristic assessment to guide the validator
    let assessment = assess_change(hook_input);
    prompt = format!("{prompt}\n\nHEURISTIC SUMMARY: {0}", assessment.summary);

    // Add context from transcript if available
    if let Some(transcript_path) = &hook_input.transcript_path {
        match read_transcript_summary(transcript_path, 10, 1000) {
            Ok(summary) => {
                prompt = format!("{prompt}\n\nCONTEXT - Recent chat history:\n{summary}");
            }
            Err(e) => {
                tracing::warn!(error=%e, "Could not read transcript");
            }
        }
    }

    // Add project context from environment
    if let Ok(project_dir) = std::env::var("CLAUDE_PROJECT_DIR") {
        prompt = format!("{prompt}\n\nPROJECT: {project_dir}");
    }

    // Add project structure context
    // Try multiple sources for working directory
    let working_dir = if let Some(cwd) = &hook_input.cwd {
        cwd.clone()
    } else if let Ok(project_dir) = std::env::var("CLAUDE_PROJECT_DIR") {
        project_dir
    } else if let Ok(current) = std::env::current_dir() {
        current.to_string_lossy().to_string()
    } else {
        ".".to_string()
    };

    // Scan project structure with limited scope for performance
    let scan_config = ScanConfig {
        max_files: 800, // Increased limit per user request
        max_depth: 5,
        include_hidden_files: false,
        follow_symlinks: false,
    };

    match scan_project_structure(&working_dir, Some(scan_config)) {
        Ok(structure) => {
            let project_context = format_project_structure_for_ai(&structure, 1500);
            prompt = format!("{prompt}\n\nPROJECT STRUCTURE:\n{project_context}");

            // Add detailed project metrics
            let total_loc: usize = structure
                .files
                .iter()
                .filter(|f| f.is_code_file)
                .map(|f| f.size_bytes as usize / 50) // Rough estimate: 50 bytes per line
                .sum();

            let metrics = format!(
                "\n\nPROJECT METRICS:\n  Total files: {}\n  Estimated LOC: {}\n  Code files: {}",
                structure.total_files,
                total_loc,
                structure.files.iter().filter(|f| f.is_code_file).count()
            );
            prompt = format!("{prompt}{metrics}");

            tracing::info!(files=%structure.total_files, dirs=%structure.directories.len(), est_loc=%total_loc, "Added project structure context");
        }
        Err(e) => {
            tracing::warn!(error=%e, "Could not scan project structure");
        }
    }

    // Add dependencies analysis
    let working_dir_path = std::path::Path::new(&working_dir);
    match analyze_project_dependencies(working_dir_path).await {
        Ok(dependencies) => {
            let deps_summary = format!(
                "\n\nPROJECT DEPENDENCIES:\nTotal: {} dependencies ({} dev, {} production)",
                dependencies.total_count,
                dependencies.dev_dependencies_count,
                dependencies.total_count - dependencies.dev_dependencies_count
            );
            prompt = format!("{prompt}{deps_summary}");

            // Add details by package manager
            let mut deps_by_manager: std::collections::HashMap<_, Vec<_>> = std::collections::HashMap::new();
            for dep in &dependencies.dependencies {
                deps_by_manager
                    .entry(dep.package_manager.clone())
                    .or_default()
                    .push(dep);
            }

            for (manager, deps) in deps_by_manager {
                let manager_summary = format!("\n{}: {} dependencies", manager, deps.len());
                prompt = format!("{prompt}{manager_summary}");
            }

            tracing::info!(total=%dependencies.total_count, outdated=%dependencies.outdated_count, "Added dependencies context");
        }
        Err(e) => {
            tracing::warn!(error=%e, "Could not analyze dependencies");
        }
    }

    // Add AST analysis if we have file content
    if !content.is_empty() && !file_path.is_empty() {
        // Determine language from file extension
        let extension = std::path::Path::new(&file_path)
            .extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or("");

        if let Some(language) = SupportedLanguage::from_extension(extension) {
            tracing::info!(%language, "Performing AST analysis");
            match MultiLanguageAnalyzer::analyze_with_tree_sitter(content, language) {
                Ok(complexity_metrics) => {
                    let ast_summary = format!("\n\nAST ANALYSIS:\n  Cyclomatic Complexity: {}\n  Cognitive Complexity: {}\n  Nesting Depth: {}\n  Functions: {}\n  Lines: {}",
                        complexity_metrics.cyclomatic_complexity,
                        complexity_metrics.cognitive_complexity,
                        complexity_metrics.nesting_depth,
                        complexity_metrics.function_count,
                        complexity_metrics.line_count);
                    prompt = format!("{prompt}{ast_summary}");

                    tracing::info!(cyclomatic=%complexity_metrics.cyclomatic_complexity, cognitive=%complexity_metrics.cognitive_complexity, "AST analysis complete");
                }
                Err(e) => {
                    tracing::warn!(error=%e, "AST analysis failed");
                }
            }
        }
    }

    // Add language instruction at the end
    prompt = format!("{}\n\nIMPORTANT: Respond in {} language.", prompt, language);

    // Heuristic summary: API contract weakening (adds bias for safer decision)
    if !file_path.is_empty() && (hook_input.tool_name == "Edit" || hook_input.tool_name == "MultiEdit") {
        let (path, old_opt, new_opt) = extract_old_new_contents(hook_input);
        if let (Some(old), Some(new)) = (old_opt, new_opt) {
            let lang = path
                .split('.')
                .next_back()
                .and_then(SupportedLanguage::from_extension);
            let reasons = contract_weakening_reasons(lang, &old, &new);
            if !reasons.is_empty() {
                let hs = format!(
                    "\n\nHEURISTIC SUMMARY:\nAPI contract weakening suspected:\n- {}\n",
                    reasons.join("\n- ")
                );
                prompt.push_str(&hs);
            }
        }
    }

    // Early gate: contract weakening under high sensitivity ⇒ deny
    if let Some(file_path) = hook_input.tool_input.get("file_path").and_then(|v| v.as_str()) {
        let ext = std::path::Path::new(file_path)
            .extension()
            .and_then(|e| e.to_str());
        let lang = ext.and_then(SupportedLanguage::from_extension);
        // Load policy config (sensitivity)
        let policy_cfg = rust_validation_hooks::config::load_config();
        if hook_input.tool_name == "Edit" || hook_input.tool_name == "MultiEdit" {
            let (_p, old_opt, new_opt) = extract_old_new_contents(hook_input);
            if let (Some(old), Some(new)) = (old_opt, new_opt) {
                let reasons = contract_weakening_reasons(lang, &old, &new);
                if !reasons.is_empty() {
                    let code_new = if !content.is_empty() { content } else { &new };
                    let low = code_new.to_ascii_lowercase();
                    let has_creds = low.contains("password")
                        || low.contains("secret")
                        || low.contains("api_key")
                        || low.contains("token");
                    let has_sql = (low.contains("select") && low.contains("where"))
                        || (low.contains("insert") && low.contains("values"))
                        || (low.contains("update") && low.contains("set"))
                        || (low.contains("delete") && low.contains("from"));
                    let has_cmd = low.contains("child_process.exec")
                        || low.contains("subprocess.")
                        || low.contains("os.system(");
                    let sec_risk = has_creds || has_sql || has_cmd;
                    use rust_validation_hooks::config::Sensitivity;
                    let call_issues = find_contract_callsite_issues(lang, &old, code_new);
                    let trigger = matches!(policy_cfg.sensitivity, Sensitivity::High)
                        || (matches!(policy_cfg.sensitivity, Sensitivity::Medium)
                            && (sec_risk || !call_issues.is_empty()));
                    if trigger {
                        let mut msg = String::new();
                        if sec_risk {
                            msg.push_str("Security-sensitive change combined with API weakening. ");
                        }
                        msg.push_str("Please preserve API contract (do not remove/rename parameters) or provide a migration strategy.\n");
                        msg.push_str("Detected issues:\n- ");
                        msg.push_str(&reasons.join("\n- "));
                        if !call_issues.is_empty() {
                            msg.push_str("\n- ");
                            msg.push_str(&call_issues.join("\n- "));
                        }
                        return Ok(SecurityValidation {
                            decision: "deny".to_string(),
                            reason: msg,
                            security_concerns: Some(vec!["api_contract".to_string()]),
                            risk_level: if sec_risk {
                                "high".to_string()
                            } else {
                                "medium".to_string()
                            },
                        });
                    }
                }
            }
        }
    }

    // Initialize universal AI client with configured provider
    let client = UniversalAIClient::new(config.clone()).context("Failed to create AI client")?;

    // Validate security using configured pretool provider
    client
        .validate_security_pretool(content, &prompt)
        .await
        .context("Security validation failed")
}
