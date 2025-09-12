use std::sync::Once;

static INIT: Once = Once::new();

/// Initialize structured logging via `tracing`.
///
/// - Respects `RUST_LOG` if set, otherwise defaults to `info`.
/// - Writes logs to stderr to avoid interfering with JSON outputs on stdout.
/// - Safe to call multiple times; subsequent calls are no-ops.
pub fn init() {
    INIT.call_once(|| {
        use tracing_subscriber::prelude::*;
        // If subscriber is already set elsewhere, ignore errors silently
        let env_filter = tracing_subscriber::EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"));

        if std::env::var("LOG_JSON").is_ok() || std::env::var("HOOK_LOG_JSON").is_ok() {
            let fmt_layer = tracing_subscriber::fmt::layer()
                .json()
                .with_writer(std::io::stderr)
                .with_target(false);
            let subscriber = tracing_subscriber::registry().with(env_filter).with(fmt_layer);
            let _ = tracing::subscriber::set_global_default(subscriber);
        } else {
            let fmt_layer = tracing_subscriber::fmt::layer()
                .with_writer(std::io::stderr) // keep stdout clean for hook JSON
                .with_target(false);
            let subscriber = tracing_subscriber::registry().with(env_filter).with(fmt_layer);
            let _ = tracing::subscriber::set_global_default(subscriber);
        }
    });
}
