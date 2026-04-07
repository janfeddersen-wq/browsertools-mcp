//! Input automation tools: click, fill, drag, hover, type, press_key, etc.
//!
//! Maps to the 9 input tools from the TypeScript implementation.

// Tool implementations will be added in Phase 2.
// Each tool will:
// 1. Parse input params (uid, text, key, etc.)
// 2. Resolve element UIDs to backend node IDs via the accessibility snapshot
// 3. Use cdp_client::input and cdp_client::dom to dispatch events
