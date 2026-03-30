use serde::{Deserialize, Serialize};

pub trait Request: serde::Serialize {
    const METHOD: &'static str;
}

pub type ProgressToken = NumberOrString;

#[derive(Debug, Eq, Hash, PartialEq, Clone, Deserialize, Serialize)]
#[serde(untagged)]
pub enum NumberOrString {
    Number(i32),
    String(String),
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClientInfo {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GeneralClientCapabilities {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub position_encodings: Option<Vec<String>>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClientCapabilities {
    // pub workspace: Option<WorkspaceClientCapabilities>,
    // pub text_document: Option<TextDocumentClientCapabilities>,
    // pub notebook_document: Option<NotebookDocumentClientCapabilities>,
    // pub window: Option<WindowClientCapabilities>,
    pub general: Option<GeneralClientCapabilities>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkDoneProgressParams {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub work_done_token: Option<ProgressToken>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InitializeRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub process_id: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub root_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub root_uri: Option<String>,
    pub capabilities: ClientCapabilities,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub client_info: Option<ClientInfo>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub locale: Option<String>,
    #[serde(flatten)]
    pub work_done_progress_params: WorkDoneProgressParams,
}

impl Request for InitializeRequest {
    const METHOD: &'static str = "initialize";
}
