//! # cdp-client
//!
//! Low-level Chrome DevTools Protocol client over WebSocket.
//!
//! Provides typed access to CDP domains: Page, DOM, Input, Network,
//! Runtime, Accessibility, Tracing, Emulation, Target, and Extensions.

pub mod accessibility;
pub mod browser;
pub mod connection;
pub mod dom;
pub mod emulation;
pub mod error;
pub mod extensions;
pub mod input;
pub mod network;
pub mod page;
pub mod runtime;
pub mod session;
pub mod target;
pub mod tracing;

pub use browser::{Browser, Channel, ConnectConfig, LaunchConfig};
pub use connection::CdpConnection;
pub use error::{CdpError, CdpResult};
pub use page::CdpPage;
pub use session::{BrowserSession, CdpSession};
