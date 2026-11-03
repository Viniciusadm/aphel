use aphel_contract::{
    Contract, ContractDescriptor, ExtensionPoint, KeyEntry, KeyKind, contract, extension_point,
};
use serde::{Deserialize, Serialize};
pub const CONTRACT: Contract = contract!("documents", "1.0.0");
pub const SOURCE: ExtensionPoint<DocumentFile> = extension_point!("documents.source");
pub const DESCRIPTOR: ContractDescriptor = ContractDescriptor {
    contract: CONTRACT,
    entries: &[
        KeyEntry::new(
            "documents.source",
            KeyKind::ExtensionPoint,
            "Owner-authorized documents",
        ),
        KeyEntry::new(
            "documents.renderer",
            KeyKind::ExtensionPoint,
            "Document layout and output rendering",
        ),
        KeyEntry::new(
            "documents.signature_provider",
            KeyKind::ExtensionPoint,
            "Electronic signature delivery providers",
        ),
    ],
};
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentHeader {
    pub issuer: String,
    pub title: String,
    pub subtitle: String,
    pub page_label: String,
    pub page_of_label: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Span {
    pub text: String,
    pub bold: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableColumn {
    pub label: String,
    pub weight: f32,
    pub right_aligned: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SummaryLine {
    pub label: String,
    pub value: String,
    pub bold: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Table {
    pub columns: Vec<TableColumn>,
    pub rows: Vec<Vec<String>>,
    pub summary: Vec<SummaryLine>,
    pub empty_message: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Block {
    Heading { level: u8, spans: Vec<Span> },
    Paragraph { lines: Vec<Vec<Span>> },
    List { items: Vec<Vec<Span>> },
    Field { label: String, value: String },
    Gap,
    Table { table: Table },
    PageBreak,
    Rule,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Document {
    pub header: DocumentHeader,
    pub blocks: Vec<Block>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentFile {
    pub file_name: String,
    pub content_type: String,
    pub content: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EnvelopeSigner {
    pub name: String,
    pub email: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EnvelopeRequest {
    pub name: String,
    pub file_name: String,
    pub content: Vec<u8>,
    pub signers: Vec<EnvelopeSigner>,
    pub message: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SentEnvelope {
    pub envelope_id: String,
    pub document_id: String,
    pub signer_ids: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NoticeKind {
    SignerSigned,
    SignerRefused,
    Completed,
    Cancelled,
    Expired,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WebhookNotice {
    pub event_name: String,
    pub kind: Option<NoticeKind>,
    pub references: Vec<String>,
    pub signer_email: Option<String>,
}
