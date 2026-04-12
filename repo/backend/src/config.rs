use once_cell::sync::Lazy;
use std::env;

/// Global application configuration, initialized once on first access.
static CONFIG: Lazy<Config> = Lazy::new(|| Config::from_env());

/// Centralized configuration for the SilverScreen backend.
/// All environment variables are read here; application code accesses
/// settings exclusively through `Config::get()`.
#[derive(Debug, Clone)]
pub struct Config {
    // Database
    pub database_url: String,
    pub database_max_connections: u32,

    // Server
    pub server_host: String,
    pub server_port: u16,

    // JWT
    pub jwt_secret: String,
    pub jwt_access_expiry_minutes: i64,
    pub jwt_refresh_expiry_days: i64,

    // Encryption
    pub encryption_key: String,
    pub encryption_key_version: u32,

    // TLS
    pub enable_tls: bool,

    // Rate limiting
    pub rate_limit_login_max: u32,
    pub rate_limit_login_ip_max: u32,
    pub rate_limit_login_window_seconds: u64,

    // Risk rules
    pub risk_bulk_order_threshold: u32,
    pub risk_bulk_order_window_minutes: u64,
    pub risk_discount_abuse_threshold: u32,
    pub risk_discount_abuse_window_minutes: u64,

    // Backup
    pub backup_dir: String,
    pub backup_retention_count: u32,
    pub backup_encryption_key: String,

    // Retention
    pub retention_orders_years: u32,
    pub retention_auth_logs_years: u32,

    // Logging
    pub rust_log: String,
    pub log_format: String,
}

impl Config {
    /// Returns a reference to the global configuration singleton.
    pub fn get() -> &'static Config {
        &CONFIG
    }

    /// Validates secrets at startup.
    ///
    /// In **dev mode** (`SILVERSCREEN_DEV_MODE=true`), known placeholder values
    /// produce a loud warning but do NOT panic — this lets developers follow the
    /// Quick Start docs without generating secrets first.
    ///
    /// In **production** (dev mode off), placeholder or weak secrets cause a
    /// hard panic to prevent insecure deployments.
    pub fn validate_secrets(&self) {
        let dev_mode = env::var("SILVERSCREEN_DEV_MODE")
            .map(|v| v == "true" || v == "1")
            .unwrap_or(false);

        // Known unsafe values: old hardcoded defaults + local dev defaults + .env.example placeholders
        let reject_jwt = [
            "silverscreen_jwt_secret_change_in_production_2024",
            "local_dev_jwt_secret_not_for_prod_use_64_chars_min_xxxxxx",
            "CHANGE_ME_use_openssl_rand_hex_64_to_generate",
        ];
        let reject_enc = [
            "0123456789abcdef0123456789abcdef",
            "local_dev_encrypt_key_32bytes!!!",
            "CHANGE_ME_use_openssl_rand_hex_32",
        ];
        let reject_backup = [
            "backup_encryption_key_change_in_production",
            "local_dev_backup_key_not_for_prod",
            "CHANGE_ME_use_a_different_random_key",
        ];

        let mut warnings: Vec<String> = Vec::new();

        if reject_jwt.contains(&self.jwt_secret.as_str()) {
            warnings.push("SECURITY: JWT_SECRET is a known default/placeholder. Generate a real secret with `openssl rand -hex 64`.".to_string());
        }
        if reject_enc.contains(&self.encryption_key.as_str()) {
            warnings.push("SECURITY: ENCRYPTION_KEY is a known default/placeholder. Generate a real 32-byte key with `openssl rand -hex 16`.".to_string());
        }
        if reject_backup.contains(&self.backup_encryption_key.as_str()) {
            warnings.push("SECURITY: BACKUP_ENCRYPTION_KEY is a known default/placeholder. Generate a real key with `openssl rand -hex 32`.".to_string());
        }

        // Key strength checks
        if self.backup_encryption_key.is_empty() {
            warnings.push("SECURITY: BACKUP_ENCRYPTION_KEY is empty.".to_string());
        } else if self.backup_encryption_key.len() < 16 {
            warnings.push(format!(
                "SECURITY: BACKUP_ENCRYPTION_KEY is too short ({} chars). Minimum 16 characters required.",
                self.backup_encryption_key.len()
            ));
        }
        if self.encryption_key.len() != 32 {
            warnings.push(format!(
                "SECURITY: ENCRYPTION_KEY must be exactly 32 bytes (got {}). Use `openssl rand -hex 16` to generate.",
                self.encryption_key.len()
            ));
        }
        if self.jwt_secret.len() < 32 {
            warnings.push(format!(
                "SECURITY: JWT_SECRET is too short ({} chars). Minimum 32 characters required.",
                self.jwt_secret.len()
            ));
        }

        for (name, val) in [
            ("JWT_SECRET", &self.jwt_secret),
            ("ENCRYPTION_KEY", &self.encryption_key),
            ("BACKUP_ENCRYPTION_KEY", &self.backup_encryption_key),
        ] {
            if val.contains("CHANGE_ME") {
                warnings.push(format!("SECURITY: {} contains 'CHANGE_ME' — replace with a real secret.", name));
            }
        }

        if !warnings.is_empty() {
            // Encryption keys are ALWAYS a hard failure — even in dev mode,
            // weak encryption keys produce trivially decryptable backups and
            // database encryption that offers no real protection.
            let has_encryption_warning = warnings.iter().any(|w| {
                w.contains("ENCRYPTION_KEY") || w.contains("BACKUP_ENCRYPTION_KEY")
            });

            if has_encryption_warning {
                for w in &warnings {
                    eprintln!("{}", w);
                }
                panic!(
                    "Encryption key validation failed. Weak encryption keys are dangerous \
                     in ALL environments (including dev) because they produce trivially \
                     decryptable data. Generate real keys — see README."
                );
            }

            if dev_mode {
                for w in &warnings {
                    eprintln!("WARNING (dev mode): {}", w);
                }
                eprintln!(
                    "WARNING: Running with insecure JWT secret because SILVERSCREEN_DEV_MODE=true. \
                     Do NOT deploy to production with these values."
                );
            } else {
                for w in &warnings {
                    eprintln!("{}", w);
                }
                panic!(
                    "Secret validation failed. Set SILVERSCREEN_DEV_MODE=true for local development, \
                     or generate real secrets (see README)."
                );
            }
        }
    }

    /// Builds a `Config` from environment variables with sensible defaults.
    fn from_env() -> Self {
        Self {
            // Database
            database_url: env_required("DATABASE_URL"),
            database_max_connections: env_parse("DATABASE_MAX_CONNECTIONS", 20),

            // Server
            server_host: env_or("SERVER_HOST", "0.0.0.0"),
            server_port: env_parse("SERVER_PORT", 8080),

            // JWT
            jwt_secret: env_required("JWT_SECRET"),
            jwt_access_expiry_minutes: env_parse("JWT_ACCESS_EXPIRY_MINUTES", 30),
            jwt_refresh_expiry_days: env_parse("JWT_REFRESH_EXPIRY_DAYS", 7),

            // Encryption
            encryption_key: env_required("ENCRYPTION_KEY"),
            encryption_key_version: env_parse("ENCRYPTION_KEY_VERSION", 1),

            // TLS
            enable_tls: env_parse("ENABLE_TLS", false),

            // Rate limiting
            rate_limit_login_max: env_parse("RATE_LIMIT_LOGIN_MAX", 10),
            rate_limit_login_ip_max: env_parse("RATE_LIMIT_LOGIN_IP_MAX", 10),
            rate_limit_login_window_seconds: env_parse("RATE_LIMIT_LOGIN_WINDOW_SECONDS", 900),

            // Risk rules
            risk_bulk_order_threshold: env_parse("RISK_BULK_ORDER_THRESHOLD", 10),
            risk_bulk_order_window_minutes: env_parse("RISK_BULK_ORDER_WINDOW_MINUTES", 60),
            risk_discount_abuse_threshold: env_parse("RISK_DISCOUNT_ABUSE_THRESHOLD", 5),
            risk_discount_abuse_window_minutes: env_parse(
                "RISK_DISCOUNT_ABUSE_WINDOW_MINUTES",
                60,
            ),

            // Backup
            backup_dir: env_or("BACKUP_DIR", "/data/backups"),
            backup_retention_count: env_parse("BACKUP_RETENTION_COUNT", 14),
            backup_encryption_key: env_required("BACKUP_ENCRYPTION_KEY"),

            // Retention
            retention_orders_years: env_parse("RETENTION_ORDERS_YEARS", 7),
            retention_auth_logs_years: env_parse("RETENTION_AUTH_LOGS_YEARS", 2),

            // Logging
            rust_log: env_or("RUST_LOG", "info"),
            log_format: env_or("LOG_FORMAT", "structured"),
        }
    }
}

/// Reads a required environment variable; panics with a clear message if missing.
fn env_required(key: &str) -> String {
    env::var(key).unwrap_or_else(|_| panic!("Required environment variable {} is not set", key))
}

/// Reads an environment variable or returns a default string.
fn env_or(key: &str, default: &str) -> String {
    env::var(key).unwrap_or_else(|_| default.to_string())
}

/// Reads an environment variable, parses it into the target type, or returns a default.
fn env_parse<T: std::str::FromStr>(key: &str, default: T) -> T {
    env::var(key)
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
}
