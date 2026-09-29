use std::path::PathBuf;

use eduport_lib::{agent, db::Db};
use rmcp::{
    ErrorData as McpError, RoleServer, ServerHandler, ServiceExt,
    model::{
        CallToolRequestParams, CallToolResponse, CallToolResult, ContentBlock, Implementation,
        ListToolsResult, PaginatedRequestParams, ServerCapabilities, ServerConfig, Tool,
        ToolAnnotations,
    },
    service::RequestContext,
    transport::stdio,
};
use serde_json::Value;

#[derive(Clone)]
struct EduportServer {
    db_path: PathBuf,
    allow_writes: bool,
}

impl ServerHandler for EduportServer {
    fn get_info(&self) -> ServerConfig {
        ServerConfig::new(ServerCapabilities::builder().enable_tools().build())
            .with_server_info(Implementation::new("eduport-mcp", env!("CARGO_PKG_VERSION")))
            .with_instructions(
                "Eduport research records and imported files. Read tools are paginated. Write tools, when enabled, follow Eduport's entity permissions and may return pending approval.",
            )
    }

    async fn list_tools(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListToolsResult, McpError> {
        let tools = agent::mcp_tool_definitions(self.allow_writes)
            .into_iter()
            .filter_map(|definition| {
                let function = definition.get("function")?;
                let name = function.get("name")?.as_str()?.to_owned();
                let description = function.get("description")?.as_str()?.to_owned();
                let schema = function.get("parameters")?.as_object()?.clone();
                let write = !agent::mcp_tool_definitions(false).iter().any(|read| {
                    read.pointer("/function/name").and_then(Value::as_str) == Some(name.as_str())
                });
                Some(
                    Tool::new(name, description, schema).with_annotations(
                        ToolAnnotations::new()
                            .read_only(!write)
                            .destructive(write)
                            .open_world(false),
                    ),
                )
            })
            .collect();
        Ok(ListToolsResult::with_all_items(tools))
    }

    async fn call_tool(
        &self,
        request: CallToolRequestParams,
        _context: RequestContext<RoleServer>,
    ) -> Result<CallToolResponse, McpError> {
        let mut db = match Db::open(&self.db_path) {
            Ok(db) => db,
            Err(error) => {
                return Ok(CallToolResult::error(vec![ContentBlock::text(format!(
                    "Could not open Eduport database: {error}"
                ))])
                .into());
            }
        };
        let args = Value::Object(request.arguments.unwrap_or_default());
        let result = match agent::mcp_call(&mut db, &request.name, &args, self.allow_writes) {
            Ok(value) => {
                let mut response =
                    CallToolResult::success(vec![ContentBlock::text(value.to_string())]);
                response.structured_content = Some(value);
                response
            }
            Err(error) => CallToolResult::error(vec![ContentBlock::text(error)]),
        };
        Ok(result.into())
    }
}

fn db_path_from_args() -> Result<(PathBuf, bool), String> {
    let mut explicit_path = None;
    let mut allow_writes = false;
    let mut args = std::env::args().skip(1);
    while let Some(argument) = args.next() {
        match argument.as_str() {
            "--db" => explicit_path = Some(PathBuf::from(args.next().ok_or("--db needs a path")?)),
            "--allow-writes" => allow_writes = true,
            _ => return Err(format!("Unknown argument: {argument}")),
        }
    }
    let path = explicit_path
        .or_else(|| std::env::var_os("EDUPORT_DB_PATH").map(PathBuf::from))
        .or_else(|| {
            #[cfg(target_os = "macos")]
            {
                std::env::var_os("HOME").map(|home| {
                    PathBuf::from(home).join(
                        "Library/Application Support/io.github.rusenbb.eduport/eduport.sqlite",
                    )
                })
            }
            #[cfg(not(target_os = "macos"))]
            {
                None
            }
        })
        .ok_or("Set EDUPORT_DB_PATH or pass --db PATH")?;
    if !path.is_file() {
        return Err(format!(
            "Eduport database does not exist: {}",
            path.display()
        ));
    }
    Ok((path, allow_writes))
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let (db_path, allow_writes) = db_path_from_args()
        .map_err(|error| std::io::Error::new(std::io::ErrorKind::InvalidInput, error))?;
    Db::open(&db_path)?;
    let service = EduportServer {
        db_path,
        allow_writes,
    }
    .serve(stdio())
    .await?;
    service.waiting().await?;
    Ok(())
}
