/// Metrics collection and monitoring system for validation hooks
/// Provides comprehensive performance, usage, and quality metrics
use anyhow::Result;
use chrono::{DateTime, Utc};
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

/// Thread-safe metrics collector with efficient concurrent access
pub struct MetricsCollector {
    // Performance metrics
    execution_times: DashMap<String, Vec<Duration>>,
    total_requests: AtomicU64,
    successful_requests: AtomicU64,
    failed_requests: AtomicU64,

    // Usage metrics
    provider_usage: DashMap<String, AtomicU64>,
    language_stats: DashMap<String, LanguageMetrics>,
    feature_usage: DashMap<String, AtomicU64>,

    // Quality metrics
    validation_results: DashMap<String, ValidationStats>,
    issue_categories: DashMap<String, AtomicU64>,

    // System metrics
    memory_usage_samples: DashMap<String, Vec<usize>>,
    cpu_usage_samples: DashMap<String, Vec<f32>>,

    // Session tracking
    session_start: Instant,
    active_sessions: AtomicUsize,
}

/// Language-specific usage statistics
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct LanguageMetrics {
    pub files_analyzed: AtomicU64,
    pub total_lines: AtomicU64,
    pub average_complexity: f32,
    pub common_issues: BTreeMap<String, u64>,
    pub performance_avg_ms: f32,
}

/// Validation statistics for different categories
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct ValidationStats {
    pub total_validations: AtomicU64,
    pub allowed_count: AtomicU64,
    pub denied_count: AtomicU64,
    pub asked_count: AtomicU64,
    pub average_confidence: f32,
    pub security_issues_found: AtomicU64,
}

/// Comprehensive metrics snapshot for reporting
#[derive(Debug, Serialize, Deserialize)]
pub struct MetricsSnapshot {
    pub timestamp: DateTime<Utc>,
    pub uptime_seconds: u64,

    // Performance summary
    pub performance: PerformanceMetrics,

    // Usage summary
    pub usage: UsageMetrics,

    // Quality summary
    pub quality: QualityMetrics,

    // System health
    pub system: SystemMetrics,

    // Trends and insights
    pub trends: TrendAnalysis,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PerformanceMetrics {
    pub total_requests: u64,
    pub success_rate: f32,
    pub average_response_time_ms: f32,
    pub p95_response_time_ms: f32,
    pub p99_response_time_ms: f32,
    pub slowest_operations: Vec<OperationTiming>,
    pub requests_per_second: f32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OperationTiming {
    pub operation: String,
    pub average_ms: f32,
    pub max_ms: f32,
    pub count: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UsageMetrics {
    pub active_sessions: usize,
    pub total_sessions: u64,
    pub provider_breakdown: BTreeMap<String, u64>,
    pub language_breakdown: BTreeMap<String, LanguageUsage>,
    pub feature_usage: BTreeMap<String, u64>,
    pub peak_concurrent_requests: usize,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LanguageUsage {
    pub files_count: u64,
    pub total_lines: u64,
    pub avg_complexity: f32,
    pub most_common_issues: Vec<IssueFrequency>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct IssueFrequency {
    pub issue_type: String,
    pub count: u64,
    pub percentage: f32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct QualityMetrics {
    pub validation_summary: ValidationSummary,
    pub issue_distribution: BTreeMap<String, u64>,
    pub security_findings: SecurityMetrics,
    pub code_quality_trends: QualityTrends,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ValidationSummary {
    pub total_validations: u64,
    pub allow_rate: f32,
    pub deny_rate: f32,
    pub ask_rate: f32,
    pub average_confidence: f32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SecurityMetrics {
    pub total_security_issues: u64,
    pub critical_issues: u64,
    pub high_risk_issues: u64,
    pub security_score_average: f32,
    pub common_vulnerabilities: Vec<VulnerabilityStats>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct VulnerabilityStats {
    pub vulnerability_type: String,
    pub count: u64,
    pub severity_distribution: BTreeMap<String, u64>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct QualityTrends {
    pub complexity_trend: Vec<f32>, // Rolling average over time
    pub issue_count_trend: Vec<u64>,
    pub quality_improvement_rate: f32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SystemMetrics {
    pub memory_usage_mb: f32,
    pub memory_peak_mb: f32,
    pub cpu_usage_percent: f32,
    pub cpu_peak_percent: f32,
    pub cache_hit_rate: f32,
    pub error_rate: f32,
    pub uptime_hours: f32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TrendAnalysis {
    pub request_volume_trend: TrendDirection,
    pub performance_trend: TrendDirection,
    pub quality_trend: TrendDirection,
    pub error_rate_trend: TrendDirection,
    pub recommendations: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum TrendDirection {
    Improving,
    Stable,
    Degrading,
    Insufficient_Data,
}

impl Default for MetricsCollector {
    fn default() -> Self {
        Self::new()
    }
}

impl MetricsCollector {
    /// Create a new metrics collector
    pub fn new() -> Self {
        Self {
            execution_times: DashMap::new(),
            total_requests: AtomicU64::new(0),
            successful_requests: AtomicU64::new(0),
            failed_requests: AtomicU64::new(0),
            provider_usage: DashMap::new(),
            language_stats: DashMap::new(),
            feature_usage: DashMap::new(),
            validation_results: DashMap::new(),
            issue_categories: DashMap::new(),
            memory_usage_samples: DashMap::new(),
            cpu_usage_samples: DashMap::new(),
            session_start: Instant::now(),
            active_sessions: AtomicUsize::new(0),
        }
    }

    /// Record execution time for an operation
    pub fn record_execution_time(&self, operation: &str, duration: Duration) {
        let mut times = self
            .execution_times
            .entry(operation.to_string())
            .or_insert_with(Vec::new);

        // Keep only last 1000 measurements for memory efficiency
        if times.len() >= 1000 {
            times.remove(0);
        }
        times.push(duration);
    }

    /// Record successful request
    pub fn record_success(&self) {
        self.total_requests.fetch_add(1, Ordering::Relaxed);
        self.successful_requests.fetch_add(1, Ordering::Relaxed);
    }

    /// Record failed request
    pub fn record_failure(&self) {
        self.total_requests.fetch_add(1, Ordering::Relaxed);
        self.failed_requests.fetch_add(1, Ordering::Relaxed);
    }

    /// Record provider usage
    pub fn record_provider_usage(&self, provider: &str) {
        let counter = self
            .provider_usage
            .entry(provider.to_string())
            .or_insert_with(|| AtomicU64::new(0));
        counter.fetch_add(1, Ordering::Relaxed);
    }

    /// Record language analysis
    pub fn record_language_analysis(&self, language: &str, lines: u64, complexity: f32, issues: &[String]) {
        let mut lang_metrics = self
            .language_stats
            .entry(language.to_string())
            .or_insert_with(LanguageMetrics::default);

        lang_metrics.files_analyzed.fetch_add(1, Ordering::Relaxed);
        lang_metrics.total_lines.fetch_add(lines, Ordering::Relaxed);

        // Update rolling average complexity
        let current_files = lang_metrics.files_analyzed.load(Ordering::Relaxed) as f32;
        lang_metrics.average_complexity =
            (lang_metrics.average_complexity * (current_files - 1.0) + complexity) / current_files;

        // Count issue types (need to handle this carefully due to HashMap in DashMap)
        drop(lang_metrics); // Release the DashMap reference

        // Update issue counts separately
        for issue in issues {
            let counter = self
                .issue_categories
                .entry(issue.clone())
                .or_insert_with(|| AtomicU64::new(0));
            counter.fetch_add(1, Ordering::Relaxed);
        }
    }

    /// Record validation result
    pub fn record_validation(&self, category: &str, decision: &str, confidence: f32, security_issues: u64) {
        let mut stats = self
            .validation_results
            .entry(category.to_string())
            .or_insert_with(ValidationStats::default);

        stats.total_validations.fetch_add(1, Ordering::Relaxed);

        match decision {
            "allow" => stats.allowed_count.fetch_add(1, Ordering::Relaxed),
            "deny" => stats.denied_count.fetch_add(1, Ordering::Relaxed),
            "ask" => stats.asked_count.fetch_add(1, Ordering::Relaxed),
            _ => 0, // Unknown decision
        };

        if security_issues > 0 {
            stats
                .security_issues_found
                .fetch_add(security_issues, Ordering::Relaxed);
        }

        // Update rolling average confidence
        let total = stats.total_validations.load(Ordering::Relaxed) as f32;
        stats.average_confidence = (stats.average_confidence * (total - 1.0) + confidence) / total;
    }

    /// Record feature usage
    pub fn record_feature_usage(&self, feature: &str) {
        let counter = self
            .feature_usage
            .entry(feature.to_string())
            .or_insert_with(|| AtomicU64::new(0));
        counter.fetch_add(1, Ordering::Relaxed);
    }

    /// Record system resource usage
    pub fn record_system_metrics(&self, memory_mb: usize, cpu_percent: f32) {
        let mut mem_samples = self
            .memory_usage_samples
            .entry("system".to_string())
            .or_insert_with(Vec::new);
        let mut cpu_samples = self
            .cpu_usage_samples
            .entry("system".to_string())
            .or_insert_with(Vec::new);

        // Keep only last 100 samples for memory efficiency
        if mem_samples.len() >= 100 {
            mem_samples.remove(0);
        }
        if cpu_samples.len() >= 100 {
            cpu_samples.remove(0);
        }

        mem_samples.push(memory_mb);
        cpu_samples.push(cpu_percent);
    }

    /// Increment active session count
    pub fn start_session(&self) {
        self.active_sessions.fetch_add(1, Ordering::Relaxed);
    }

    /// Decrement active session count
    pub fn end_session(&self) {
        self.active_sessions.fetch_sub(1, Ordering::Relaxed);
    }

    /// Generate comprehensive metrics snapshot
    pub fn get_snapshot(&self) -> Result<MetricsSnapshot> {
        let now = Utc::now();
        let uptime = self.session_start.elapsed();

        Ok(MetricsSnapshot {
            timestamp: now,
            uptime_seconds: uptime.as_secs(),
            performance: self.calculate_performance_metrics()?,
            usage: self.calculate_usage_metrics()?,
            quality: self.calculate_quality_metrics()?,
            system: self.calculate_system_metrics()?,
            trends: self.analyze_trends()?,
        })
    }

    /// Calculate performance metrics
    fn calculate_performance_metrics(&self) -> Result<PerformanceMetrics> {
        let total = self.total_requests.load(Ordering::Relaxed);
        let successful = self.successful_requests.load(Ordering::Relaxed);

        let success_rate = if total > 0 {
            (successful as f32) / (total as f32) * 100.0
        } else {
            0.0
        };

        // Calculate response time statistics
        let mut all_times: Vec<Duration> = Vec::new();
        let mut operation_timings: Vec<OperationTiming> = Vec::new();

        for entry in self.execution_times.iter() {
            let operation = entry.key();
            let times = entry.value();

            if !times.is_empty() {
                all_times.extend(times.iter().copied());

                let avg_ms = times.iter().map(|d| d.as_millis() as f32).sum::<f32>() / times.len() as f32;
                let max_ms = times.iter().map(|d| d.as_millis() as f32).fold(0.0, f32::max);

                operation_timings.push(OperationTiming {
                    operation: operation.clone(),
                    average_ms: avg_ms,
                    max_ms,
                    count: times.len() as u64,
                });
            }
        }

        // Sort by average time (slowest first)
        operation_timings.sort_by(|a, b| {
            b.average_ms
                .partial_cmp(&a.average_ms)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        let (avg_ms, p95_ms, p99_ms) = if !all_times.is_empty() {
            all_times.sort();
            let avg = all_times.iter().map(|d| d.as_millis() as f32).sum::<f32>() / all_times.len() as f32;
            let p95_idx = (all_times.len() as f32 * 0.95) as usize;
            let p99_idx = (all_times.len() as f32 * 0.99) as usize;
            let p95 = all_times.get(p95_idx).unwrap_or(&Duration::ZERO).as_millis() as f32;
            let p99 = all_times.get(p99_idx).unwrap_or(&Duration::ZERO).as_millis() as f32;
            (avg, p95, p99)
        } else {
            (0.0, 0.0, 0.0)
        };

        let requests_per_second = if uptime.as_secs() > 0 {
            total as f32 / uptime.as_secs() as f32
        } else {
            0.0
        };

        Ok(PerformanceMetrics {
            total_requests: total,
            success_rate,
            average_response_time_ms: avg_ms,
            p95_response_time_ms: p95_ms,
            p99_response_time_ms: p99_ms,
            slowest_operations: operation_timings.into_iter().take(10).collect(),
            requests_per_second,
        })
    }

    /// Calculate usage metrics
    fn calculate_usage_metrics(&self) -> Result<UsageMetrics> {
        let active = self.active_sessions.load(Ordering::Relaxed);

        // Provider breakdown
        let mut provider_breakdown = HashMap::new();
        for entry in self.provider_usage.iter() {
            provider_breakdown.insert(entry.key().clone(), entry.value().load(Ordering::Relaxed));
        }

        // Language breakdown with detailed stats
        let mut language_breakdown = HashMap::new();
        for entry in self.language_stats.iter() {
            let lang = entry.key();
            let metrics = entry.value();

            let files = metrics.files_analyzed.load(Ordering::Relaxed);
            let lines = metrics.total_lines.load(Ordering::Relaxed);

            // Get most common issues for this language
            let mut common_issues: Vec<IssueFrequency> = self
                .issue_categories
                .iter()
                .map(|entry| IssueFrequency {
                    issue_type: entry.key().clone(),
                    count: entry.value().load(Ordering::Relaxed),
                    percentage: 0.0, // Will be calculated below
                })
                .collect();

            // Calculate percentages and sort by frequency
            let total_issues: u64 = common_issues.iter().map(|i| i.count).sum();
            for issue in &mut common_issues {
                issue.percentage = if total_issues > 0 {
                    (issue.count as f32 / total_issues as f32) * 100.0
                } else {
                    0.0
                };
            }
            common_issues.sort_by(|a, b| b.count.cmp(&a.count));
            common_issues.truncate(5); // Keep top 5

            language_breakdown.insert(
                lang.clone(),
                LanguageUsage {
                    files_count: files,
                    total_lines: lines,
                    avg_complexity: metrics.average_complexity,
                    most_common_issues: common_issues,
                },
            );
        }

        // Feature usage
        let mut feature_usage = HashMap::new();
        for entry in self.feature_usage.iter() {
            feature_usage.insert(entry.key().clone(), entry.value().load(Ordering::Relaxed));
        }

        Ok(UsageMetrics {
            active_sessions: active,
            total_sessions: self.total_requests.load(Ordering::Relaxed), // Rough approximation
            provider_breakdown,
            language_breakdown,
            feature_usage,
            peak_concurrent_requests: active, // This would need better tracking in a real implementation
        })
    }

    /// Calculate quality metrics
    fn calculate_quality_metrics(&self) -> Result<QualityMetrics> {
        // Validation summary
        let mut total_validations = 0u64;
        let mut total_allowed = 0u64;
        let mut total_denied = 0u64;
        let mut total_asked = 0u64;
        let mut weighted_confidence = 0f32;
        let mut total_security_issues = 0u64;

        for entry in self.validation_results.iter() {
            let stats = entry.value();
            let validations = stats.total_validations.load(Ordering::Relaxed);
            total_validations += validations;
            total_allowed += stats.allowed_count.load(Ordering::Relaxed);
            total_denied += stats.denied_count.load(Ordering::Relaxed);
            total_asked += stats.asked_count.load(Ordering::Relaxed);
            total_security_issues += stats.security_issues_found.load(Ordering::Relaxed);

            weighted_confidence += stats.average_confidence * validations as f32;
        }

        let validation_summary = ValidationSummary {
            total_validations,
            allow_rate: if total_validations > 0 {
                (total_allowed as f32 / total_validations as f32) * 100.0
            } else {
                0.0
            },
            deny_rate: if total_validations > 0 {
                (total_denied as f32 / total_validations as f32) * 100.0
            } else {
                0.0
            },
            ask_rate: if total_validations > 0 {
                (total_asked as f32 / total_validations as f32) * 100.0
            } else {
                0.0
            },
            average_confidence: if total_validations > 0 {
                weighted_confidence / total_validations as f32
            } else {
                0.0
            },
        };

        // Issue distribution
        let mut issue_distribution = HashMap::new();
        for entry in self.issue_categories.iter() {
            issue_distribution.insert(entry.key().clone(), entry.value().load(Ordering::Relaxed));
        }

        // Security metrics (simplified for now)
        let security_findings = SecurityMetrics {
            total_security_issues,
            critical_issues: total_security_issues / 4, // Rough estimate
            high_risk_issues: total_security_issues / 2,
            security_score_average: 7.5, // Would need more sophisticated calculation
            common_vulnerabilities: vec![], // Would need to track specific vulnerability types
        };

        // Quality trends (simplified)
        let code_quality_trends = QualityTrends {
            complexity_trend: vec![], // Would need historical data
            issue_count_trend: vec![],
            quality_improvement_rate: 0.0,
        };

        Ok(QualityMetrics {
            validation_summary,
            issue_distribution,
            security_findings,
            code_quality_trends,
        })
    }

    /// Calculate system metrics
    fn calculate_system_metrics(&self) -> Result<SystemMetrics> {
        let uptime_hours = self.session_start.elapsed().as_secs_f32() / 3600.0;

        // Calculate memory statistics
        let (memory_avg, memory_peak) = if let Some(mem_samples) = self.memory_usage_samples.get("system") {
            let avg = if !mem_samples.is_empty() {
                mem_samples.iter().sum::<usize>() as f32 / mem_samples.len() as f32
            } else {
                0.0
            };
            let peak = mem_samples.iter().max().copied().unwrap_or(0) as f32;
            (avg, peak)
        } else {
            (0.0, 0.0)
        };

        // Calculate CPU statistics
        let (cpu_avg, cpu_peak) = if let Some(cpu_samples) = self.cpu_usage_samples.get("system") {
            let avg = if !cpu_samples.is_empty() {
                cpu_samples.iter().sum::<f32>() / cpu_samples.len() as f32
            } else {
                0.0
            };
            let peak = cpu_samples.iter().fold(0.0f32, |acc, &x| acc.max(x));
            (avg, peak)
        } else {
            (0.0, 0.0)
        };

        let total_requests = self.total_requests.load(Ordering::Relaxed);
        let failed_requests = self.failed_requests.load(Ordering::Relaxed);
        let error_rate = if total_requests > 0 {
            (failed_requests as f32 / total_requests as f32) * 100.0
        } else {
            0.0
        };

        Ok(SystemMetrics {
            memory_usage_mb: memory_avg,
            memory_peak_mb: memory_peak,
            cpu_usage_percent: cpu_avg,
            cpu_peak_percent: cpu_peak,
            cache_hit_rate: 85.0, // Would need cache metrics
            error_rate,
            uptime_hours,
        })
    }

    /// Analyze trends and provide insights
    fn analyze_trends(&self) -> Result<TrendAnalysis> {
        // This is a simplified version - real trend analysis would need historical data
        let mut recommendations = Vec::new();

        let total_requests = self.total_requests.load(Ordering::Relaxed);
        let failed_requests = self.failed_requests.load(Ordering::Relaxed);

        // Basic recommendations based on current state
        if total_requests == 0 {
            recommendations.push("System has not processed any requests yet".to_string());
        } else if failed_requests * 100 / total_requests > 5 {
            recommendations.push("High error rate detected - investigate failed requests".to_string());
        }

        if self.active_sessions.load(Ordering::Relaxed) > 100 {
            recommendations.push("High concurrent load - consider scaling resources".to_string());
        }

        // Add performance recommendations
        if let Some(execution_times) = self.execution_times.get("validation") {
            if let Some(max_time) = execution_times.iter().max() {
                if max_time.as_millis() > 5000 {
                    recommendations
                        .push("Slow validation detected - optimize AI provider response time".to_string());
                }
            }
        }

        Ok(TrendAnalysis {
            request_volume_trend: TrendDirection::Insufficient_Data,
            performance_trend: TrendDirection::Stable,
            quality_trend: TrendDirection::Stable,
            error_rate_trend: TrendDirection::Stable,
            recommendations,
        })
    }

    /// Export metrics to JSON format
    pub fn export_json(&self) -> Result<String> {
        let snapshot = self.get_snapshot()?;
        serde_json::to_string_pretty(&snapshot).map_err(Into::into)
    }

    /// Reset all metrics (useful for testing)
    pub fn reset(&self) {
        self.execution_times.clear();
        self.total_requests.store(0, Ordering::Relaxed);
        self.successful_requests.store(0, Ordering::Relaxed);
        self.failed_requests.store(0, Ordering::Relaxed);
        self.provider_usage.clear();
        self.language_stats.clear();
        self.feature_usage.clear();
        self.validation_results.clear();
        self.issue_categories.clear();
        self.memory_usage_samples.clear();
        self.cpu_usage_samples.clear();
        self.active_sessions.store(0, Ordering::Relaxed);
    }
}

/// Global metrics instance for easy access
lazy_static::lazy_static! {
    pub static ref GLOBAL_METRICS: Arc<MetricsCollector> = Arc::new(MetricsCollector::new());
}

/// Convenience macros for common metrics operations
#[macro_export]
macro_rules! record_execution_time {
    ($operation:expr, $code:block) => {{
        let start = std::time::Instant::now();
        let result = $code;
        let duration = start.elapsed();
        $crate::metrics::GLOBAL_METRICS.record_execution_time($operation, duration);
        result
    }};
}

#[macro_export]
macro_rules! record_success {
    () => {
        $crate::metrics::GLOBAL_METRICS.record_success();
    };
}

#[macro_export]
macro_rules! record_failure {
    () => {
        $crate::metrics::GLOBAL_METRICS.record_failure();
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;
    use std::time::Duration;

    #[test]
    fn test_metrics_collector_creation() {
        let metrics = MetricsCollector::new();
        assert_eq!(metrics.total_requests.load(Ordering::Relaxed), 0);
        assert_eq!(metrics.active_sessions.load(Ordering::Relaxed), 0);
    }

    #[test]
    fn test_record_execution_time() {
        let metrics = MetricsCollector::new();
        let duration = Duration::from_millis(100);

        metrics.record_execution_time("test_operation", duration);

        assert!(metrics.execution_times.contains_key("test_operation"));
        let times = metrics.execution_times.get("test_operation").unwrap();
        assert_eq!(times.len(), 1);
        assert_eq!(times[0], duration);
    }

    #[test]
    fn test_success_failure_recording() {
        let metrics = MetricsCollector::new();

        metrics.record_success();
        metrics.record_success();
        metrics.record_failure();

        assert_eq!(metrics.total_requests.load(Ordering::Relaxed), 3);
        assert_eq!(metrics.successful_requests.load(Ordering::Relaxed), 2);
        assert_eq!(metrics.failed_requests.load(Ordering::Relaxed), 1);
    }

    #[test]
    fn test_provider_usage_recording() {
        let metrics = MetricsCollector::new();

        metrics.record_provider_usage("openai");
        metrics.record_provider_usage("openai");
        metrics.record_provider_usage("anthropic");

        assert_eq!(
            metrics
                .provider_usage
                .get("openai")
                .unwrap()
                .load(Ordering::Relaxed),
            2
        );
        assert_eq!(
            metrics
                .provider_usage
                .get("anthropic")
                .unwrap()
                .load(Ordering::Relaxed),
            1
        );
    }

    #[test]
    fn test_language_analysis_recording() {
        let metrics = MetricsCollector::new();
        let issues = vec!["security".to_string(), "performance".to_string()];

        metrics.record_language_analysis("rust", 100, 5.5, &issues);

        let lang_stats = metrics.language_stats.get("rust").unwrap();
        assert_eq!(lang_stats.files_analyzed.load(Ordering::Relaxed), 1);
        assert_eq!(lang_stats.total_lines.load(Ordering::Relaxed), 100);
        assert_eq!(lang_stats.average_complexity, 5.5);
    }

    #[test]
    fn test_validation_recording() {
        let metrics = MetricsCollector::new();

        metrics.record_validation("pretool", "allow", 0.9, 0);
        metrics.record_validation("pretool", "deny", 0.8, 2);

        let stats = metrics.validation_results.get("pretool").unwrap();
        assert_eq!(stats.total_validations.load(Ordering::Relaxed), 2);
        assert_eq!(stats.allowed_count.load(Ordering::Relaxed), 1);
        assert_eq!(stats.denied_count.load(Ordering::Relaxed), 1);
        assert_eq!(stats.security_issues_found.load(Ordering::Relaxed), 2);
    }

    #[test]
    fn test_session_tracking() {
        let metrics = MetricsCollector::new();

        metrics.start_session();
        metrics.start_session();
        assert_eq!(metrics.active_sessions.load(Ordering::Relaxed), 2);

        metrics.end_session();
        assert_eq!(metrics.active_sessions.load(Ordering::Relaxed), 1);
    }

    #[test]
    fn test_metrics_snapshot() {
        let metrics = MetricsCollector::new();

        // Add some test data
        metrics.record_success();
        metrics.record_execution_time("test", Duration::from_millis(50));
        metrics.record_provider_usage("openai");

        let snapshot = metrics.get_snapshot().unwrap();

        assert!(snapshot.uptime_seconds > 0);
        assert_eq!(snapshot.performance.total_requests, 1);
        assert_eq!(snapshot.performance.success_rate, 100.0);
        assert!(snapshot.usage.provider_breakdown.contains_key("openai"));
    }

    #[test]
    fn test_concurrent_access() {
        let metrics = Arc::new(MetricsCollector::new());
        let mut handles = Vec::new();

        // Spawn multiple threads to test concurrent access
        for i in 0..10 {
            let metrics_clone = Arc::clone(&metrics);
            let handle = thread::spawn(move || {
                for _ in 0..100 {
                    metrics_clone.record_success();
                    metrics_clone.record_provider_usage(&format!("provider_{}", i % 3));
                    metrics_clone.record_execution_time("test", Duration::from_millis(i as u64));
                }
            });
            handles.push(handle);
        }

        // Wait for all threads to complete
        for handle in handles {
            handle.join().unwrap();
        }

        // Verify results
        assert_eq!(metrics.total_requests.load(Ordering::Relaxed), 1000);
        assert_eq!(metrics.successful_requests.load(Ordering::Relaxed), 1000);

        // Check that providers were recorded
        for i in 0..3 {
            let provider_name = format!("provider_{i}");
            assert!(metrics.provider_usage.contains_key(&provider_name));
        }
    }

    #[test]
    fn test_memory_limits() {
        let metrics = MetricsCollector::new();

        // Test execution time limit
        for i in 0..1500 {
            metrics.record_execution_time("test", Duration::from_millis(i));
        }

        let times = metrics.execution_times.get("test").unwrap();
        assert_eq!(times.len(), 1000); // Should be capped at 1000

        // Test memory usage limit
        for i in 0..150 {
            metrics.record_system_metrics(i * 10, 50.0);
        }

        let mem_samples = metrics.memory_usage_samples.get("system").unwrap();
        assert_eq!(mem_samples.len(), 100); // Should be capped at 100
    }

    #[test]
    fn test_reset_functionality() {
        let metrics = MetricsCollector::new();

        // Add some data
        metrics.record_success();
        metrics.record_provider_usage("test");
        metrics.start_session();

        // Verify data exists
        assert_eq!(metrics.total_requests.load(Ordering::Relaxed), 1);
        assert!(metrics.provider_usage.contains_key("test"));
        assert_eq!(metrics.active_sessions.load(Ordering::Relaxed), 1);

        // Reset and verify everything is cleared
        metrics.reset();

        assert_eq!(metrics.total_requests.load(Ordering::Relaxed), 0);
        assert!(metrics.provider_usage.is_empty());
        assert_eq!(metrics.active_sessions.load(Ordering::Relaxed), 0);
    }
}
