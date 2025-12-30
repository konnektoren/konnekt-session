use tracing_subscriber::{EnvFilter, fmt, layer::SubscriberExt, util::SubscriberInitExt};

/// Logging configuration
#[derive(Debug, Clone)]
pub struct LogConfig {
    pub default_level: tracing::Level,
    pub json_format: bool,
    pub file_output: Option<String>,
    pub chrome_trace: bool,
    pub show_spans: bool,
    pub show_thread_ids: bool,
    pub show_targets: bool,
    pub show_logs: bool, // 🆕 NEW: Whether to show logs to stdout/stderr

    #[cfg(feature = "console")]
    pub enable_console: bool,
}

impl Default for LogConfig {
    fn default() -> Self {
        Self {
            default_level: tracing::Level::INFO,
            json_format: false,
            file_output: None,
            chrome_trace: false,
            show_spans: false,
            show_thread_ids: false,
            show_targets: true,
            show_logs: true, // 🆕 Default: show logs
            #[cfg(feature = "console")]
            enable_console: false,
        }
    }
}

impl LogConfig {
    /// Development configuration (verbose, human-readable)
    pub fn dev() -> Self {
        Self {
            default_level: tracing::Level::DEBUG,
            show_spans: true,
            show_thread_ids: true,
            ..Default::default()
        }
    }

    /// Development with Chrome tracing
    pub fn dev_with_trace() -> Self {
        Self {
            default_level: tracing::Level::DEBUG,
            chrome_trace: true,
            show_spans: true,
            show_thread_ids: true,
            ..Default::default()
        }
    }

    /// TUI mode (logs to file, not stdout)
    pub fn tui() -> Self {
        Self {
            default_level: tracing::Level::INFO,
            show_logs: false, // 🆕 Hide logs in TUI mode
            ..Default::default()
        }
    }

    /// Enable Chrome tracing
    pub fn with_chrome_trace(mut self) -> Self {
        self.chrome_trace = true;
        self
    }

    /// Enable tokio console
    #[cfg(feature = "console")]
    pub fn with_console(mut self) -> Self {
        self.enable_console = true;
        self
    }

    /// Hide logs (for TUI)
    pub fn without_logs(mut self) -> Self {
        self.show_logs = false;
        self
    }

    /// Log to file
    pub fn with_file_output(mut self, path: String) -> Self {
        self.file_output = Some(path);
        self
    }

    pub fn init(self) -> Result<(), String> {
        // Build env filter
        let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| {
            EnvFilter::new(format!(
                "{}={}",
                env!("CARGO_PKG_NAME").replace('-', "_"),
                self.default_level
            ))
            .add_directive("matchbox_socket=info".parse().unwrap())
            .add_directive("konnekt_session_core=debug".parse().unwrap())
            .add_directive("konnekt_session_p2p=debug".parse().unwrap())
        });

        // 🔧 Chrome tracing (highest priority)
        #[cfg(all(feature = "chrome-trace", not(target_arch = "wasm32")))]
        if self.chrome_trace {
            use tracing_chrome::ChromeLayerBuilder;

            let (chrome_layer, _guard) = ChromeLayerBuilder::new().build();

            if self.show_logs {
                eprintln!("📊 Chrome trace enabled");
                eprintln!("   Trace file: trace-<timestamp>.json");
                eprintln!("   View at: https://ui.perfetto.dev/");
                eprintln!("");
            }

            // Also add fmt layer for terminal output (if enabled)
            if self.show_logs {
                let fmt_layer = fmt::layer().with_target(true).compact();

                tracing_subscriber::registry()
                    .with(env_filter)
                    .with(chrome_layer)
                    .with(fmt_layer)
                    .try_init()
                    .map_err(|e| format!("Failed to initialize tracing: {}", e))?;
            } else {
                tracing_subscriber::registry()
                    .with(env_filter)
                    .with(chrome_layer)
                    .try_init()
                    .map_err(|e| format!("Failed to initialize tracing: {}", e))?;
            }

            // Keep guard alive for the lifetime of the program
            std::mem::forget(_guard);

            return Ok(());
        }

        // Console subscriber (next priority)
        #[cfg(feature = "console")]
        if self.enable_console {
            use console_subscriber::ConsoleLayer;

            if self.show_logs {
                eprintln!("🔍 Tokio Console enabled - connect with `tokio-console`");
                eprintln!("📡 Console server started on 127.0.0.1:6669");
            }

            let console_layer = ConsoleLayer::builder()
                .server_addr(([127, 0, 0, 1], 6669))
                .spawn();

            tracing_subscriber::registry()
                .with(env_filter)
                .with(console_layer)
                .try_init()
                .map_err(|e| format!("Failed to initialize tracing: {}", e))?;

            if self.show_logs {
                eprintln!("✅ Tracing subscriber initialized with console");
            }

            return Ok(());
        }

        // Default: fmt layer (only if show_logs is true)
        if self.show_logs {
            let fmt_layer = fmt::layer()
                .with_target(self.show_targets)
                .with_thread_ids(self.show_thread_ids);

            tracing_subscriber::registry()
                .with(env_filter)
                .with(fmt_layer)
                .try_init()
                .map_err(|e| format!("Failed to initialize tracing: {}", e))
        } else {
            // Silent mode: no fmt layer, just filter
            tracing_subscriber::registry()
                .with(env_filter)
                .try_init()
                .map_err(|e| format!("Failed to initialize tracing: {}", e))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = LogConfig::default();
        assert_eq!(config.default_level, tracing::Level::INFO);
        assert!(!config.json_format);
        assert!(config.file_output.is_none());
        assert!(!config.chrome_trace);
        assert!(config.show_logs); // 🆕 Default: show logs
    }

    #[test]
    fn test_dev_config() {
        let config = LogConfig::dev();
        assert_eq!(config.default_level, tracing::Level::DEBUG);
        assert!(config.show_spans);
        assert!(config.show_thread_ids);
        assert!(config.show_logs);
    }

    #[test]
    fn test_tui_config() {
        let config = LogConfig::tui();
        assert_eq!(config.default_level, tracing::Level::INFO);
        assert!(!config.show_logs); // 🆕 TUI mode: hide logs
    }

    #[test]
    fn test_dev_with_trace() {
        let config = LogConfig::dev_with_trace();
        assert_eq!(config.default_level, tracing::Level::DEBUG);
        assert!(config.chrome_trace);
    }

    #[test]
    fn test_with_chrome_trace() {
        let config = LogConfig::default().with_chrome_trace();
        assert!(config.chrome_trace);
    }

    #[test]
    fn test_without_logs() {
        let config = LogConfig::default().without_logs();
        assert!(!config.show_logs);
    }

    #[test]
    fn test_with_file_output() {
        let config = LogConfig::default().with_file_output("app.log".to_string());
        assert_eq!(config.file_output, Some("app.log".to_string()));
    }
}
