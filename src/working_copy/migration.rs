// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
//! Embedded documents must cross the same schema boundary as named projects.
use super::*;

pub(super) fn decode(bytes: &[u8]) -> Result<Record, CopyError> {
    #[derive(Deserialize)]
    struct Envelope {
        named: Option<NamedFile>,
        project: serde_json::Value,
    }
    let mut envelope: Envelope =
        serde_json::from_slice(bytes).map_err(|e| CopyError::Invalid(e.to_string()))?;
    let version = envelope
        .project
        .get("format_version")
        .and_then(|v| v.as_u64())
        .and_then(|v| u32::try_from(v).ok())
        .ok_or_else(|| CopyError::Invalid("Missing document version".into()))?;
    // The content baseline uses the old canonical serialization, not the raw
    // envelope's JSON order. Translate it only when it proves the copy was clean.
    let previous = match version {
        1 => {
            let old: crate::model::Document = serde_json::from_value(envelope.project.clone())
                .map_err(|e| CopyError::Invalid(e.to_string()))?;
            Some(serde_json::to_vec_pretty(&old).map_err(|e| CopyError::Invalid(e.to_string()))?)
        }
        2 => {
            let old: Project = serde_json::from_value(envelope.project.clone())
                .map_err(|e| CopyError::Invalid(e.to_string()))?;
            Some(version_two_bytes(&old)?)
        }
        3..=15 => {
            let old: Project = serde_json::from_value(envelope.project.clone())
                .map_err(|e| CopyError::Invalid(e.to_string()))?;
            Some(serde_json::to_vec_pretty(&old).map_err(|e| CopyError::Invalid(e.to_string()))?)
        }
        document::FORMAT_VERSION => None,
        unsupported => return Err(CopyError::UnsupportedVersion(unsupported)),
    };
    let project_bytes =
        serde_json::to_vec(&envelope.project).map_err(|e| CopyError::Invalid(e.to_string()))?;
    let project = document::decode(&project_bytes).map_err(CopyError::Invalid)?;
    let encoded = project.encode().map_err(CopyError::Invalid)?;
    if let (Some(previous), Some(named)) = (previous, &mut envelope.named)
        && named.base_content == Some(fingerprint(&previous))
    {
        named.base_content = Some(fingerprint(&encoded));
    }
    Ok(Record {
        version: VERSION,
        named: envelope.named,
        project,
    })
}

/// Frozen v2 field order/omission rules, used solely to recognize an old clean
/// baseline. Never write this projection back to the user's project or record.
fn version_two_bytes(project: &Project) -> Result<Vec<u8>, CopyError> {
    #[derive(Serialize)]
    struct Alternate<'a> {
        id: document::Id,
        name: &'a str,
        content: &'a document::Content,
    }
    #[derive(Serialize)]
    struct Page<'a> {
        id: document::Id,
        name: &'a str,
        notes: &'a str,
        parent: Option<document::Id>,
        content: &'a document::Content,
        alternates: Vec<Alternate<'a>>,
    }
    #[derive(Serialize)]
    struct VersionTwo<'a> {
        format_version: u32,
        title: &'a str,
        next_id: document::Id,
        settings: &'a document::Settings,
        pages: Vec<Page<'a>>,
        definitions: &'a [document::Definition],
        assets: &'a [document::Asset],
        #[serde(skip_serializing_if = "Option::is_none")]
        font: Option<&'a std::sync::Arc<crate::fonts::ProjectFont>>,
    }
    let old = VersionTwo {
        format_version: 2,
        title: &project.title,
        next_id: project.next_id,
        settings: &project.settings,
        pages: project
            .pages
            .iter()
            .map(|p| Page {
                id: p.id,
                name: &p.name,
                notes: &p.notes,
                parent: p.parent,
                content: &p.content,
                alternates: p
                    .alternates
                    .iter()
                    .map(|a| Alternate {
                        id: a.id,
                        name: &a.name,
                        content: &a.content,
                    })
                    .collect(),
            })
            .collect(),
        definitions: &project.definitions,
        assets: &project.assets,
        font: project.font.as_ref(),
    };
    serde_json::to_vec_pretty(&old).map_err(|e| CopyError::Invalid(e.to_string()))
}
