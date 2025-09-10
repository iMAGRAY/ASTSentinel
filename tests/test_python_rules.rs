use rust_validation_hooks::analysis::ast::{AstQualityScorer, SupportedLanguage, IssueSeverity};
use rust_validation_hooks::analysis::ast::quality_scorer::IssueCategory;

#[test]
fn py_unreachable_after_return_in_try() {
    let code = "def f():\n    try:\n        return 1\n        x = 2\n    except Exception:\n        return 0\n";
    let s = AstQualityScorer::new();
    let res = s.analyze(code, SupportedLanguage::Python).unwrap();
    assert!(res.concrete_issues.iter().any(|i| matches!(i.category, IssueCategory::UnreachableCode)),
        "expected UnreachableCode in Python after return in try");
}

#[test]
fn py_too_many_parameters() {
    let code = "def f(a,b,c,d,e,f):\n    return 1\n";
    let s = AstQualityScorer::new();
    let res = s.analyze(code, SupportedLanguage::Python).unwrap();
    assert!(res.concrete_issues.iter().any(|i| matches!(i.category, IssueCategory::TooManyParameters)),
        "expected TooManyParameters in Python (6 > 5)");
}

#[test]
fn py_hardcoded_credentials_detected() {
    let code = "password = 'secret'\n";
    let s = AstQualityScorer::new();
    let res = s.analyze(code, SupportedLanguage::Python).unwrap();
    assert!(res.concrete_issues.iter().any(|i| matches!(i.category, IssueCategory::HardcodedCredentials) && matches!(i.severity, IssueSeverity::Critical)),
        "expected Critical HardcodedCredentials in Python assignment");
}

#[test]
fn py_sql_injection_in_fstring_detected() {
    let code = "def q(uid):\n    query = f\"SELECT * FROM users WHERE id={uid}\"\n    return query\n";
    let s = AstQualityScorer::new();
    let res = s.analyze(code, SupportedLanguage::Python).unwrap();
    assert!(res.concrete_issues.iter().any(|i| matches!(i.category, IssueCategory::SqlInjection)),
        "expected SqlInjection for SQL in f-string");
}

#[test]
fn py_try_except_finally_good_code_no_issues() {
    let code = "def f(x):\n    try:\n        if x:\n            return 1\n        return 0\n    except Exception:\n        return -1\n    finally:\n        pass\n";
    let s = AstQualityScorer::new();
    let res = s.analyze(code, SupportedLanguage::Python).unwrap();
    assert!(res.concrete_issues.is_empty(), "expected no issues for well-formed try/except/finally Python code");
}

#[test]
fn py_multiple_except_else_finally_good() {
    let code = "def f(x):\n    try:\n        if x: return 1\n        return 0\n    except ValueError:\n        return -1\n    except Exception:\n        return -2\n    else:\n        pass\n    finally:\n        pass\n";
    let s = AstQualityScorer::new();
    let res = s.analyze(code, SupportedLanguage::Python).unwrap();
    assert!(res.concrete_issues.is_empty(), "expected no issues for multiple except/else/finally Python code");
}
