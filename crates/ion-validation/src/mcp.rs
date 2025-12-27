//! MCP (Model Context Protocol) integration for AI agents
//!
//! This module provides MCP-compatible interfaces for AI agents to interact
//! with the validation framework through structured requests and responses.
//!
//! **Note**: This is planned for future implementation and requires the `mcp` feature.

// TODO: Implement MCP server interface
// TODO: Add validation plan serialization/deserialization for MCP
// TODO: Add event streaming over MCP transport
// TODO: Add capability discovery via MCP

/// Placeholder for MCP server implementation
pub struct McpServer;

impl McpServer {
    /// Create a new MCP server (placeholder)
    pub fn new() -> Self {
        Self
    }
}

impl Default for McpServer {
    fn default() -> Self {
        Self::new()
    }
}
