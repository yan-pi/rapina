use tracing::Level;
use tracing_subscriber::{EnvFilter, fmt};

/// Configuration for the tracing/logging system.
///
/// Use the builder pattern to configure logging output format and level.
///
/// # Examples
///
/// ```ignore
/// use rapina::prelude::*;
///
/// // JSON logging for production
/// Rapina::new()
///     .with_tracing(TracingConfig::new().json())
///     .router(router)
///     .listen("127.0.0.1:3000")
///     .await
/// ```
#[derive(Debug, Clone)]
pub struct TracingConfig {
    /// Output logs as JSON.
    pub json: bool,
    /// The minimum log level.
    pub level: Level,
    /// Include the target (module path) in logs.
    pub with_target: bool,
    /// Include the source file in logs.
    pub with_file: bool,
    /// Include line numbers in logs.
    pub with_line_number: bool,
}

impl Default for TracingConfig {
    fn default() -> Self {
        Self {
            json: false,
            level: Level::INFO,
            with_target: true,
            with_file: false,
            with_line_number: false,
        }
    }
}

impl TracingConfig {
    /// Creates a new tracing configuration with default values.
    pub fn new() -> Self {
        Self::default()
    }

    /// Enables JSON output format.
    pub fn json(mut self) -> Self {
        self.json = true;
        self
    }

    /// Sets the minimum log level.
    pub fn level(mut self, level: Level) -> Self {
        self.level = level;
        self
    }

    /// Configures whether to include the target in logs.
    pub fn with_target(mut self, enabled: bool) -> Self {
        self.with_target = enabled;
        self
    }

    /// Configures whether to include file names in logs.
    pub fn with_file(mut self, enabled: bool) -> Self {
        self.with_file = enabled;
        self
    }

    /// Configures whether to include line numbers in logs.
    pub fn with_line_number(mut self, enabled: bool) -> Self {
        self.with_line_number = enabled;
        self
    }

    /// Initializes the tracing subscriber with this configuration.
    pub fn init(self) {
        let filter = EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| EnvFilter::new(self.level.to_string()));

        if self.json {
            fmt()
                .with_env_filter(filter)
                .with_target(self.with_target)
                .with_file(self.with_file)
                .with_line_number(self.with_line_number)
                .json()
                .init();
        } else {
            fmt()
                .with_env_filter(filter)
                .with_target(self.with_target)
                .with_file(self.with_file)
                .with_line_number(self.with_line_number)
                .init();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tracing_config_default() {
        let config = TracingConfig::default();
        assert!(!config.json);
        assert_eq!(config.level, Level::INFO);
        assert!(config.with_target);
        assert!(!config.with_file);
        assert!(!config.with_line_number);
    }

    #[test]
    fn test_tracing_config_new() {
        let config = TracingConfig::new();
        assert!(!config.json);
    }

    #[test]
    fn test_tracing_config_json() {
        let config = TracingConfig::new().json();
        assert!(config.json);
    }

    #[test]
    fn test_tracing_config_level() {
        let config = TracingConfig::new().level(Level::DEBUG);
        assert_eq!(config.level, Level::DEBUG);
    }

    #[test]
    fn test_tracing_config_builder_chain() {
        let config = TracingConfig::new()
            .json()
            .level(Level::TRACE)
            .with_target(false)
            .with_file(true)
            .with_line_number(true);

        assert!(config.json);
        assert_eq!(config.level, Level::TRACE);
        assert!(!config.with_target);
        assert!(config.with_file);
        assert!(config.with_line_number);
    }
}
