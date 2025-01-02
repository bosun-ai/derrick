use std::collections::HashMap;

use anyhow::Result;

use dropshot::{
    endpoint, ApiDescription, ConfigDropshot, ConfigLogging, ConfigLoggingLevel, HandlerTaskMode,
    HttpError, HttpResponseOk, HttpServerStarter, Path, RequestContext, TypedBody,
};

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tokio::sync::Mutex;

use crate::server::Server;
use crate::workspace_controllers::CommandOutput;

pub async fn serve_http(server: Server) -> Result<()> {
    let log = ConfigLogging::StderrTerminal {
        level: ConfigLoggingLevel::Info,
    }
    .to_logger("workspace-provider")
    .map_err(|e| anyhow::anyhow!("Failed to create logger: {:?}", e))?;

    let mut api = ApiDescription::new();
    api.register(create_workspace)?;
    api.register(destroy_workspace)?;
    api.register(list_workspaces)?;
    api.register(cmd)?;
    api.register(cmd_with_output)?;
    api.register(write_file)?;
    api.register(read_file)?;
    api.register(health)?;

    let server_mutex = Mutex::new(server);

    let server = HttpServerStarter::new(
        &ConfigDropshot {
      bind_address: "127.0.0.1:50080".parse().unwrap(),
      request_body_max_bytes: /* 100MB */ 100 * 1024 * 1024,
      default_handler_task_mode: HandlerTaskMode::Detached,
      log_headers: Default::default(),
  },
        api,
        server_mutex,
        &log,
    )
    .map_err(|error| anyhow::anyhow!("Failed to start server: {:?}", error))?;

    server
        .start()
        .await
        .map_err(|error| anyhow::anyhow!("Server failed: {:?}", error))?;

    Ok(())
}

// HTTP Server endpoints:
// POST /workspaces                                 creates a new workspace
// DELETE /workspaces/:workspace_id                 destroys a workspace
// GET /workspaces                                  lists existing workspaces
//
// Workspace actions
// POST /workspaces/:workspace_id/cmd               runs a command in the workspace
// POST /workspaces/:workspace_id/cmd_with_output   runs a command in the workspace and returns the output
// POST /workspaces/:workspace_id/write_file        writes a file in the workspace
// POST /workspaces/:workspace_id/read_file         reads a file in the workspace

// GET /health                                    returns the health of the workspace provider

#[derive(Serialize, JsonSchema)]
struct HealthResponse {
    healthy: bool,
}

#[endpoint {
    method = GET,
    path = "/health",
}]
async fn health(
    _rqctx: RequestContext<Mutex<Server>>,
) -> Result<HttpResponseOk<HealthResponse>, HttpError> {
    Ok(HttpResponseOk(HealthResponse { healthy: true }))
}

#[derive(Serialize, JsonSchema)]
struct WorkspaceResponse {
    id: String,
}

#[derive(Serialize, JsonSchema)]
struct WorkspaceListResponse {
    workspaces: Vec<WorkspaceResponse>,
}

#[derive(Deserialize, JsonSchema)]
struct CreateWorkspaceRequest {
    env: Option<HashMap<String, String>>,
}

#[endpoint {
    method = POST,
    path = "/workspaces",
}]
async fn create_workspace(
    rqctx: RequestContext<Mutex<Server>>,
    body: TypedBody<CreateWorkspaceRequest>,
) -> Result<HttpResponseOk<WorkspaceResponse>, HttpError> {
    let id = rqctx
        .context()
        .lock()
        .await
        .create_workspace(body.into_inner().env.unwrap_or_default())
        .await
        .map_err(|e| {
            tracing::error!("Failed to create workspace: {:?}", e);
            HttpError::for_internal_error("Failed to create workspace".to_string())
        })?;
    Ok(HttpResponseOk(WorkspaceResponse { id }))
}

#[derive(Deserialize, JsonSchema)]
struct SinglePathIdParam {
    id: String,
}

#[endpoint {
    method = DELETE,
    path = "/workspaces/{id}",
}]
async fn destroy_workspace(
    rqctx: RequestContext<Mutex<Server>>,
    path: Path<SinglePathIdParam>,
) -> Result<HttpResponseOk<bool>, HttpError> {
    let success = rqctx
        .context()
        .lock()
        .await
        .destroy_workspace(&path.into_inner().id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to destroy workspace: {:?}", e);
            HttpError::for_internal_error("Failed to destroy workspace".to_string())
        })?;
    Ok(HttpResponseOk(success))
}

#[endpoint {
    method = GET,
    path = "/workspaces",
}]
async fn list_workspaces(
    rqctx: RequestContext<Mutex<Server>>,
) -> Result<HttpResponseOk<WorkspaceListResponse>, HttpError> {
    let workspaces = rqctx
        .context()
        .lock()
        .await
        .list_workspaces()
        .await
        .map_err(|e| {
            tracing::error!("Failed to list workspaces: {:?}", e);
            HttpError::for_internal_error("Failed to list workspaces".to_string())
        })?;
    Ok(HttpResponseOk(WorkspaceListResponse {
        workspaces: workspaces
            .iter()
            .map(|id| WorkspaceResponse { id: id.clone() })
            .collect(),
    }))
}

#[derive(Deserialize, JsonSchema)]
struct CmdRequest {
    cmd: String,
    working_dir: Option<String>,
    env: Option<HashMap<String, String>>,
    timeout: Option<u64>,
}

#[endpoint {
    method = POST,
    path = "/workspaces/{id}/cmd",
}]
async fn cmd(
    rqctx: RequestContext<Mutex<Server>>,
    path: Path<SinglePathIdParam>,
    body: TypedBody<CmdRequest>,
) -> Result<HttpResponseOk<()>, HttpError> {
    let body = body.into_inner();
    rqctx
        .context()
        .lock()
        .await
        .cmd(
            &path.into_inner().id,
            &body.cmd,
            body.working_dir.as_deref(),
            body.env.unwrap_or_default(),
            body.timeout.map(|t| Duration::from_secs(t)),
        )
        .await
        .map_err(|e| {
            tracing::error!("Failed to run command: {:?}", e);
            HttpError::for_internal_error("Failed to run command".to_string())
        })?;
    Ok(HttpResponseOk(()))
}

#[derive(Serialize, JsonSchema)]
struct CommandOutputResponse {
    output: String,
    exit_code: i32,
}

impl From<CommandOutput> for CommandOutputResponse {
    fn from(output: CommandOutput) -> Self {
        Self {
            output: output.output,
            exit_code: output.exit_code,
        }
    }
}

#[endpoint {
    method = POST,
    path = "/workspaces/{id}/cmd_with_output",
}]
async fn cmd_with_output(
    rqctx: RequestContext<Mutex<Server>>,
    path: Path<SinglePathIdParam>,
    body: TypedBody<CmdRequest>,
) -> Result<HttpResponseOk<CommandOutputResponse>, HttpError> {
    let body = body.into_inner();
    let output = rqctx
        .context()
        .lock()
        .await
        .cmd_with_output(
            &path.into_inner().id,
            &body.cmd,
            body.working_dir.as_deref(),
            body.env.unwrap_or_default(),
            body.timeout.map(|t| Duration::from_secs(t)),
        )
        .await
        .map_err(|e| {
            tracing::error!("Failed to run command with output: {:?}", e);
            HttpError::for_internal_error("Failed to run command with output".to_string())
        })?;
    Ok(HttpResponseOk(output.into()))
}

#[derive(Deserialize, JsonSchema)]
struct WriteFileRequest {
    path: String,
    content: String,
    working_dir: Option<String>,
}

#[endpoint {
    method = POST,
    path = "/workspaces/{id}/write_file",
}]
async fn write_file(
    rqctx: RequestContext<Mutex<Server>>,
    path: Path<SinglePathIdParam>,
    body: TypedBody<WriteFileRequest>,
) -> Result<HttpResponseOk<()>, HttpError> {
    let body = body.into_inner();
    rqctx
        .context()
        .lock()
        .await
        .write_file(
            &path.into_inner().id,
            &body.path,
            &body.content,
            body.working_dir.as_deref(),
        )
        .await
        .map_err(|e| {
            tracing::error!("Failed to write file: {:?}", e);
            HttpError::for_internal_error("Failed to write file".to_string())
        })?;
    Ok(HttpResponseOk(()))
}

#[derive(Deserialize, JsonSchema)]
struct ReadFileRequest {
    path: String,
    working_dir: Option<String>,
}

#[endpoint {
    method = POST,
    path = "/workspaces/{id}/read_file",
}]
async fn read_file(
    rqctx: RequestContext<Mutex<Server>>,
    path: Path<SinglePathIdParam>,
    body: TypedBody<ReadFileRequest>,
) -> Result<HttpResponseOk<String>, HttpError> {
    let body = body.into_inner();
    let content = rqctx
        .context()
        .lock()
        .await
        .read_file(
            &path.into_inner().id,
            &body.path,
            body.working_dir.as_deref(),
        )
        .await
        .map_err(|e| {
            tracing::error!("Failed to read file: {:?}", e);
            HttpError::for_internal_error("Failed to read file".to_string())
        })?;
    Ok(HttpResponseOk(content))
}
