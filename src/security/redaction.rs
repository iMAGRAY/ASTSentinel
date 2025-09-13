use regex::Regex;

/// Redact common secret patterns from arbitrary text.
/// Returns (redacted_text, count_of_replacements)
pub fn redact_with_report(input: &str) -> (String, usize) {
    let mut s = input.to_string();
    let mut count = 0usize;

    // Common key/value patterns: password = foo, api_key: 'abc', token=xyz
    let kv_patterns: &[&str] = &[
        "(?i)\\b(password|pass|pwd)\\b\\s*[:=]\\s*['\\\"]?([^'\\\"\\s]+)['\\\"]?",
        "(?i)\\b(api[_-]?key|apikey|secret|token|access[_-]?key|secret[_-]?access[_-]?key)\\b\\s*[:=]\\s*['\\\"]?([^'\\\"\\s]+)['\\\"]?",
    ];

    for pat in kv_patterns {
        if let Ok(re) = Regex::new(pat) {
            let mut local = 0usize;
            s = re
                .replace_all(&s, |caps: &regex::Captures| {
                    local += 1;
                    format!("{}: <REDACTED>", &caps[1])
                })
                .to_string();
            count += local;
        }
    }

    // Bearer tokens in headers
    if let Ok(re_bearer) = Regex::new(r"(?i)bearer\s+[A-Za-z0-9\-_.=]+") {
        let c = re_bearer.find_iter(&s).count();
        if c > 0 {
            s = re_bearer.replace_all(&s, "Bearer <REDACTED>").to_string();
            count += c;
        }
    }

    // Connection strings / DSNs
    if let Ok(re_conn) = Regex::new(r"(?i)(jdbc:|postgres(?:ql)?:|mongodb:|redis:|amqp:|mongodb\+srv:)[^\s\n]+") {
        let c = re_conn.find_iter(&s).count();
        if c > 0 {
            s = re_conn.replace_all(&s, "<REDACTED:CONN_STRING>").to_string();
            count += c;
        }
    }

    // Long opaque tokens (heuristic): alnum/_/- 32+ chars
    if let Ok(re_long) = Regex::new(r"\b[A-Za-z0-9_\-]{32,}\b") {
        let mut local = 0usize;
        s = re_long
            .replace_all(&s, |_: &regex::Captures| {
                local += 1;
                "<REDACTED:TOKEN>".to_string()
            })
            .to_string();
        count += local;
    }

    (s, count)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn redacts_password_and_key() {
        let src = "db.password = 's3cr3t' and API_KEY=abcd1234efgh5678ijkl9012mnop";
        let (out, n) = redact_with_report(src);
        assert!(out.contains("password: <REDACTED>"));
        assert!(out.contains("<REDACTED:TOKEN>") || out.contains("API_KEY: <REDACTED>"));
        assert!(n >= 1);
    }
}
