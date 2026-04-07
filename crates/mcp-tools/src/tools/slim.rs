//! Slim tool set -- 3 essential tools for basic automation.
//!
//! Provides: take_screenshot, navigate_page, evaluate_script.
//!
//! The slim server implementation lives in `crate::slim_server::SlimServer`.
//! When `--slim` is passed on the CLI, `SlimServer` is used instead of
//! `ChromeDevToolsServer`, exposing only 3 tools.
