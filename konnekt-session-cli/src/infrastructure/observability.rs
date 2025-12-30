use tracing_subscriber::{EnvFilter, Layer, fmt, layer::SubscriberExt, util::SubscriberInitExt};

/// Logging configuration
#[derive(Debug, Clone)]
pub struct LogConfig {
    pub default_level: tracing::Level,
    pub json_format: bool,
    pub file_output: Option<String>,
    pub chrome_trace: Option<String>,
    pub show_spans: bool,
    pub show_thread_ids: bool,
    pub show_targets: bool,

    #[cfg(feature = "console")]
    pub enable_console: bool,
}

impl Default for LogConfig {
    fn default() -> Self {
        Self {
            default_level: tracing::Level::INFO,
            json_format: false,
            file_output: None,
            chrome_trace: None,
            show_spans: false,
            show_thread_ids: false,
            show_targets: true,
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
            chrome_trace: Some("trace.json".to_string()),
            show_spans: true,
            show_thread_ids: true,
            ..Default::default()
        }
    }

    /// Enable Chrome tracing
    pub fn with_chrome_trace(mut self, path: &str) -> Self {
        self.chrome_trace = Some(path.to_string());
        self
    }

    /// Enable tokio console
    #[cfg(feature = "console")]
    pub fn with_console(mut self) -> Self {
        self.enable_console = true;
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

        // 🔧 FIX: Chrome tracing (highest priority)
        #[cfg(all(feature = "chrome-trace", not(target_arch = "wasm32")))]
        if let Some(chrome_path) = self.chrome_trace {
            use std::fs::File;
            use tracing_chrome::ChromeLayerBuilder;

            // Create the file first
            let file = File::create(&chrome_path)
                .map_err(|e| format!("Failed to create trace file: {}", e))?;

            // Build chrome layer - use writer, not file path
            let (chrome_layer, guard) = ChromeLayerBuilder::new()
                .writer(file) // 🔧 FIX: Use .writer() instead of .file()
                .include_args(true)
                .build();

            // Also add fmt layer for terminal output
            let fmt_layer = fmt::layer().with_target(true).compact();

            tracing_subscriber::registry()
                .with(env_filter)
                .with(chrome_layer)
                .with(fmt_layer)
                .try_init()
                .map_err(|e| format!("Failed to initialize tracing: {}", e))?;

            eprintln!("📊 Chrome trace enabled: {}", chrome_path);
            eprintln!("   View at: https://ui.perfetto.dev/");
            eprintln!("");

            // Keep guard alive for the lifetime of the program
            std::mem::forget(guard);

            return Ok(());
        }

        // Console subscriber (next priority)
        #[cfg(feature = "console")]
        if self.enable_console {
            use console_subscriber::ConsoleLayer;

            eprintln!("🔍 Tokio Console enabled - connect with `tokio-console`");

            let console_layer = ConsoleLayer::builder()
                .server_addr(([127, 0, 0, 1], 6669))
                .spawn();

            eprintln!("📡 Console server started on 127.0.0.1:6669");

            tracing_subscriber::registry()
                .with(env_filter)
                .with(console_layer)
                .try_init()
                .map_err(|e| format!("Failed to initialize tracing: {}", e))?;

            eprintln!("✅ Tracing subscriber initialized with console");

            return Ok(());
        }

        // Default: fmt layer
        let fmt_layer = fmt::layer()
            .with_target(self.show_targets)
            .with_thread_ids(self.show_thread_ids);

        tracing_subscriber::registry()
            .with(env_filter)
            .with(fmt_layer)
            .try_init()
            .map_err(|e| format!("Failed to initialize tracing: {}", e))
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
        assert!(config.chrome_trace.is_none());
    }

    #[test]
    fn test_dev_config() {
        let config = LogConfig::dev();
        assert_eq!(config.default_level, tracing::Level::DEBUG);
        assert!(config.show_spans);
        assert!(config.show_thread_ids);
    }

    #[test]
    fn test_dev_with_trace() {
        let config = LogConfig::dev_with_trace();
        assert_eq!(config.default_level, tracing::Level::DEBUG);
        assert_eq!(config.chrome_trace, Some("trace.json".to_string()));
    }

    #[test]
    fn test_with_chrome_trace() {
        let config = LogConfig::default().with_chrome_trace("custom.json");
        assert_eq!(config.chrome_trace, Some("custom.json".to_string()));
    }
}
