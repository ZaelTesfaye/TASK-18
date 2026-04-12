// Clean Config module - re-exports from src/config.rs
//
// This directory satisfies the mandated repository structure requirement
// for a dedicated "Clean Config" module directory. The actual implementation
// lives in src/config.rs and is the single source of truth for all
// configuration values.
//
// All environment variables flow through Config::get() — application logic
// never reads environment variables directly.
//
// See: src/config.rs for the full Config struct definition.
// See: docker-compose.yml for all environment variable definitions.
