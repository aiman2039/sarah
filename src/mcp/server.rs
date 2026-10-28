use std::process::ExitCode;

use anyhow::Result;
use rmcp::handler::server::router::tool::ToolRouter;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::{CallToolResult, ContentBlock, Implementation, ServerCapabilities, ServerConfig};
use rmcp::{ServerHandler, ServiceExt, tool, tool_handler, tool_router};

use super::requests::{CheckFileRequest, ReportRequest, WarningsRequest};
use super::tools::ToolHelpers;

const INSTRUCTIONS: &str = "sarah measures tokens, lines and function sizes per file and warns \
when they exceed the limits in sarah.toml. Call `check_file` after you edit a file, \
`warnings` (with `changed_since`) before you finish a change, and `report` to decide what to \
refactor. Token counts use OpenAI encodings and approximate Claude's.";

/// The MCP server; each tool runs one sarah analysis on a blocking thread.
#[derive(Clone)]
pub struct McpServer {
    tool_router: ToolRouter<Self>,
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
        Self::blocking(move || ToolHelpers::report(request)).await
    }

    #[tool(
        description = "Only the warnings (files, functions and folders over their limits), as JSON. Use changed_since to see only what your branch touched."
    )]
    async fn warnings(&self, Parameters(request): Parameters<WarningsRequest>) -> CallToolResult {
        Self::blocking(move || ToolHelpers::warnings(request)).await
    }

    #[tool(
        description = "One file's tokens, lines, functions (lines, tokens, depth, params, cyclomatic and cognitive complexity) and its warnings, as JSON."
    )]
    async fn check_file(
        &self,
        Parameters(request): Parameters<CheckFileRequest>,
    ) -> CallToolResult {
        Self::blocking(move || ToolHelpers::check_file(request)).await
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
