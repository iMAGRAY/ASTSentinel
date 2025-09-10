use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::sync::Mutex;

static TIMINGS: Lazy<Mutex<HashMap<String, Vec<u128>>>> = Lazy::new(|| Mutex::new(HashMap::new()));

pub fn enabled() -> bool {
    std::env::var("AST_TIMINGS").map(|v| !v.is_empty()).unwrap_or(false)
}

pub fn record(label: &str, dur_ms: u128) {
    if !enabled() { return; }
    let mut g = TIMINGS.lock().unwrap();
    g.entry(label.to_string()).or_default().push(dur_ms);
}

fn quantiles(mut v: Vec<u128>) -> (u128, u128, u128, u128) {
    if v.is_empty() { return (0,0,0,0); }
    v.sort_unstable();
    let len = v.len() as f64;
    let idx = |p: f64| -> usize { ((p * (len-1.0)).round() as isize).max(0) as usize };
    let p50 = v[idx(0.50)];
    let p95 = v[idx(0.95)];
    let p99 = v[idx(0.99)];
    let sum: u128 = v.iter().copied().sum();
    let avg = sum / (v.len() as u128);
    (p50, p95, p99, avg)
}

pub fn summary() -> String {
    if !enabled() { return String::new(); }
    let g = TIMINGS.lock().unwrap();
    if g.is_empty() { return String::new(); }
    let mut out = String::new();
    out.push_str("=== TIMINGS (ms) ===\n");
    let mut keys: Vec<_> = g.keys().cloned().collect();
    keys.sort();
    for k in keys {
        if let Some(v) = g.get(&k) {
            let (p50,p95,p99,avg) = quantiles(v.clone());
            out.push_str(&format!("{}: count={} p50={} p95={} p99={} avg={}\n", k, v.len(), p50, p95, p99, avg));
        }
    }
    out
}

