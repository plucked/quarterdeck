use std::{borrow::Cow, collections::HashSet, sync::Arc};

use rmcp::{
    ErrorData, RoleServer, ServerHandler,
    model::{
        CacheScope, CallToolRequestParams, CallToolResponse, CallToolResult, ContentBlock,
        Implementation, ListToolsResult, PaginatedRequestParams, ProtocolVersion,
        ServerCapabilities, ServerConfig, Tool, ToolAnnotations,
    },
    service::RequestContext,
};
use tokio::sync::Semaphore;
use tokio_util::sync::CancellationToken;

use crate::{McpError, McpHandler, McpToolDescriptor, McpToolResult};

const SUPPORTED_VERSIONS: &[ProtocolVersion] = &[
    ProtocolVersion::V_2026_07_28,
    ProtocolVersion::V_2025_11_25,
    ProtocolVersion::V_2025_06_18,
];
const MAXIMUM_HANDLERS: usize = 8;
const MAXIMUM_TOOL_NAME_BYTES: usize = 128;

/// The private SDK adapter shared by independent HTTP request services.
#[derive(Clone)]
pub(crate) struct McpService {
    handler: Arc<dyn McpHandler>,
    catalog: Arc<Vec<Tool>>,
    handler_slots: Arc<Semaphore>,
    cancellation: CancellationToken,
}

impl McpService {
    /// Captures and validates the immutable tool catalog before listening.
    pub(crate) fn new(
        handler: Arc<dyn McpHandler>,
        cancellation: CancellationToken,
    ) -> Result<Self, McpError> {
        // Reject invalid or ambiguous catalogs before the agent discovers them.
        let mut names = HashSet::new();
        let mut catalog = Vec::new();
        for descriptor in handler.tools() {
            if !names.insert(descriptor.name.clone()) {
                return Err(McpError::InvalidCatalog("duplicate tool name".into()));
            }
            catalog.push(sdk_tool(descriptor)?);
        }

        // The catalog and handler are owned by independently executing requests.
        Ok(Self {
            handler,
            catalog: Arc::new(catalog),
            handler_slots: Arc::new(Semaphore::new(MAXIMUM_HANDLERS)),
            cancellation,
        })
    }
}

impl ServerHandler for McpService {
    /// Advertises only ordinary tools and the adapter's implementation identity.
    fn get_info(&self) -> ServerConfig {
        ServerConfig::new(ServerCapabilities::builder().enable_tools().build()).with_server_info(
            Implementation::new("quarterdeck-run", env!("CARGO_PKG_VERSION")),
        )
    }

    /// Limits negotiation to protocol revisions exercised by integration tests.
    fn supported_protocol_versions(&self) -> Cow<'static, [ProtocolVersion]> {
        Cow::Borrowed(SUPPORTED_VERSIONS)
    }

    /// Supplies the descriptor to the SDK's standard HTTP header validation.
    fn get_tool(&self, name: &str) -> Option<Tool> {
        self.catalog.iter().find(|tool| tool.name == name).cloned()
    }

    /// Returns the frozen run catalog with private zero-lifetime cache hints.
    fn list_tools(
        &self,
        request: Option<PaginatedRequestParams>,
        context: RequestContext<RoleServer>,
    ) -> impl Future<Output = Result<ListToolsResult, ErrorData>> + Send {
        // Stateless legacy requests still have to select a supported revision.
        if let Err(error) = validate_context_version(&context) {
            return std::future::ready(Err(error));
        }
        // This bounded catalog fits one page; arbitrary cursors are invalid.
        if request.is_some_and(|request| request.cursor.is_some()) {
            return std::future::ready(Err(ErrorData::invalid_params(
                "tool catalog has no cursor",
                None,
            )));
        }
        std::future::ready(Ok(ListToolsResult::with_all_items(
            self.catalog.as_ref().clone(),
        )
        .with_ttl_ms(0)
        .with_cache_scope(CacheScope::Private)))
    }

    /// Dispatches bounded synchronous work off the transport runtime thread.
    async fn call_tool(
        &self,
        request: CallToolRequestParams,
        context: RequestContext<RoleServer>,
    ) -> Result<CallToolResponse, ErrorData> {
        // The SDK supplies a default legacy revision when its header is absent.
        validate_context_version(&context)?;
        // Unknown names are protocol failures; they never reach application code.
        if !self.catalog.iter().any(|tool| tool.name == request.name) {
            return Err(ErrorData::invalid_params("unknown tool", None));
        }
        if self.cancellation.is_cancelled() {
            return Err(ErrorData::internal_error("run is closed", None));
        }

        // A timed-out HTTP request cannot release the slot of a running handler.
        let permit = self
            .handler_slots
            .clone()
            .try_acquire_owned()
            .map_err(|_| {
                ErrorData::internal_error("tool concurrency limit reached; retry later", None)
            })?;
        let handler = self.handler.clone();
        let cancellation = self.cancellation.clone();
        let completed = tokio::task::spawn_blocking(move || {
            let _permit = permit;
            if cancellation.is_cancelled() {
                return McpToolResult::failure(
                    "Run is closed.",
                    serde_json::json!({"closed": true}),
                );
            }
            handler.call_tool(
                &request.name,
                serde_json::Value::Object(request.arguments.unwrap_or_default()),
            )
        })
        .await;

        // Keep application failures readable by the model, including panics.
        let result = completed.unwrap_or_else(|_| {
            McpToolResult::failure(
                "The tool handler failed unexpectedly; the run must be stopped.",
                serde_json::json!({"handler_failed": true}),
            )
        });
        let mut response = CallToolResult::success(vec![ContentBlock::text(result.text)]);
        response.is_error = Some(result.is_error);
        response.structured_content = Some(result.structured);
        Ok(response.into())
    }
}

/// Rejects legacy fallback versions outside the explicitly supported revisions.
fn validate_context_version(context: &RequestContext<RoleServer>) -> Result<(), ErrorData> {
    let version = context.protocol_version().ok_or_else(|| {
        ErrorData::invalid_params("request has no negotiated protocol version", None)
    })?;
    if !SUPPORTED_VERSIONS.contains(&version) {
        return Err(ErrorData::unsupported_protocol_version(
            version,
            SUPPORTED_VERSIONS,
        ));
    }
    Ok(())
}

/// Converts a checked application descriptor to the SDK's private representation.
fn sdk_tool(descriptor: McpToolDescriptor) -> Result<Tool, McpError> {
    // Keep stable tool identifiers within the protocol's portable name alphabet.
    if descriptor.name.is_empty()
        || descriptor.name.len() > MAXIMUM_TOOL_NAME_BYTES
        || !descriptor
            .name
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-' | b'.'))
    {
        return Err(McpError::InvalidCatalog("invalid tool name".into()));
    }
    let input_schema = descriptor
        .input_schema
        .as_object()
        .ok_or_else(|| McpError::InvalidCatalog("input schema must be a JSON object".into()))?;

    // Schemas remain application-owned contracts, with no review-domain coupling.
    let mut tool = Tool::new(
        descriptor.name,
        descriptor.description,
        input_schema.clone(),
    );
    tool.output_schema = descriptor
        .output_schema
        .map(|schema| {
            schema.as_object().cloned().map(Arc::new).ok_or_else(|| {
                McpError::InvalidCatalog("output schema must be a JSON object".into())
            })
        })
        .transpose()?;
    tool.annotations = Some(ToolAnnotations::new().read_only(descriptor.read_only));
    Ok(tool)
}
