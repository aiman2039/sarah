use std::process::ExitCode;
use std::sync::Arc;

use anyhow::Result;
use rmcp::handler::server::router::tool::ToolRouter;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::{CallToolResult, ContentBlock, Implementation, ServerCapabilities, ServerConfig};
use rmcp::{ServerHandler, ServiceExt, tool, tool_handler, tool_router};

use super::requests::{
    CheckFileRequest, DiffRequest, RatchetToolRequest, ReportRequest, WarningsRequest,
};
use super::tools::ToolHelpers;
use crate::cache::ScanCache;

const INSTRUCTIONS: &str = "sarah measures tokens, lines, functions, types, imports, duplicates, \
cycles and folder structure under sarah.toml. Call `check_file` after editing a file, \
`warnings` or `diff` before finishing a branch, `ratchet` after shrinking code, and `report` \
to plan refactors. Rules include large_file, too_many_functions, too_many_imports, \
large_function, deep_function, too_many_params, too_many_bool_params, complex_function, \
large_type, forbidden_import, circular_import, duplicate_code, todo_comment, unused_allow \
and folder_* checks.";

/// The MCP server; shares an in-memory `ScanCache` across tool calls.
pub struct McpServer {
    tool_router: ToolRouter<Self>,
    cache: Arc<ScanCache>,
}

impl Clone for McpServer {
    fn clone(&self) -> Self {
        Self {
            tool_router: self.tool_router.clone(),
            cache: Arc::clone(&self.cache),
        }
    }
}

#[tool_router]
impl McpServer {
    /// Serve on stdin/stdout until the client disconnects.
    pub fn run_stdio() -> Result<ExitCode> {
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()?;
        runtime.block_on(async {
            let server = Self {
                tool_router: Self::tool_router(),
                cache: Arc::new(ScanCache::new()),
            };
            server
                .serve(rmcp::transport::stdio())
                .await?
                .waiting()
                .await?;
            Ok(ExitCode::SUCCESS)
        })
    }

    #[tool(
        description = "Full sarah report for a directory: totals, largest files and functions, folder stats and warnings. Markdown by default, or JSON."
    )]
    async fn report(&self, Parameters(request): Parameters<ReportRequest>) -> CallToolResult {
        let cache = Arc::clone(&self.cache);
        Self::blocking(move || ToolHelpers::report(request, &cache)).await
    }

    #[tool(
        description = "Only the warnings (files, functions, types, imports, cycles, duplicates and folders over limits), as JSON. Use changed_since for branch warnings."
    )]
    async fn warnings(&self, Parameters(request): Parameters<WarningsRequest>) -> CallToolResult {
        let cache = Arc::clone(&self.cache);
        Self::blocking(move || ToolHelpers::warnings(request, &cache)).await
    }

    #[tool(
        description = "One file's tokens, lines, functions and warnings (including parent-folder rules), as JSON."
    )]
    async fn check_file(
        &self,
        Parameters(request): Parameters<CheckFileRequest>,
    ) -> CallToolResult {
        let cache = Arc::clone(&self.cache);
        Self::blocking(move || ToolHelpers::check_file(request, &cache)).await
    }

    #[tool(
        description = "Token, line, file and warning deltas against the merge base with a git revision, as JSON."
    )]
    async fn diff(&self, Parameters(request): Parameters<DiffRequest>) -> CallToolResult {
        let cache = Arc::clone(&self.cache);
        Self::blocking(move || ToolHelpers::diff(request, &cache)).await
    }

    #[tool(
        description = "Check or lower limits in sarah.toml to today's maximums (dry_run = true by default)."
    )]
    async fn ratchet(&self, Parameters(request): Parameters<RatchetToolRequest>) -> CallToolResult {
        let cache = Arc::clone(&self.cache);
        Self::blocking(move || ToolHelpers::ratchet(request, &cache)).await
    }
}

impl McpServer {
    /// Run a tool body off the async runtime; errors go back to the agent as tool errors.
    async fn blocking(work: impl FnOnce() -> Result<String> + Send + 'static) -> CallToolResult {
        let outcome = tokio::task::spawn_blocking(work)
            .await
            .map_err(anyhow::Error::from)
            .and_then(|result| result);
        match outcome {
            Ok(text) => CallToolResult::success(vec![ContentBlock::text(text)]),
            Err(err) => CallToolResult::error(vec![ContentBlock::text(format!("{err:#}"))]),
        }
    }
}

#[tool_handler(router = self.tool_router)]
impl ServerHandler for McpServer {
    fn get_info(&self) -> ServerConfig {
        ServerConfig::new(ServerCapabilities::builder().enable_tools().build())
            .with_server_info(Implementation::new("sarah", env!("CARGO_PKG_VERSION")))
            .with_instructions(INSTRUCTIONS)
    }
}
