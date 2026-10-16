// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
//! Durable local project records, scoped ownership and guarded named-file writes.
//! Scheduling and user decisions belong to callers, not this storage boundary.
use crate::{
    document::{self, Project},
    storage::write_atomic,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::{self, Read},
    path::{Path, PathBuf},
};

const VERSION: u32 = 1;
const MAX_RECORD_BYTES: usize = document::MAX_FILE_BYTES + 1024 * 1024;
type Fingerprint = [u8; 32];
fn fingerprint(bytes: &[u8]) -> Fingerprint {
    Sha256::digest(bytes).into()
}

#[derive(Debug, PartialEq, Eq)]
pub enum CopyError {
    Io(String),
    Invalid(String),
    UnsupportedVersion(u32),
    Busy,
    NamedMissing,
    NamedChanged,
    NotNamed,
    ForeignPlatform,
}
impl std::fmt::Display for CopyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(s) | Self::Invalid(s) => f.write_str(s),
            Self::UnsupportedVersion(v) => write!(f, "Working copy version {v} is not supported"),
            Self::Busy => f.write_str("This project's working copy is in use by another window"),
            Self::NamedMissing => {
                f.write_str("The saved file is missing. Save a new copy to choose its location.")
            }
            Self::NamedChanged => {
                f.write_str("The saved file changed outside this window. Local edits are retained.")
            }
            Self::NotNamed => f.write_str("Choose a saved-file location for this project first"),
            Self::ForeignPlatform => {
                f.write_str("The saved-file location belongs to another operating system")
            }
        }
    }
}
impl std::error::Error for CopyError {}
impl From<io::Error> for CopyError {
    fn from(e: io::Error) -> Self {
        Self::Io(e.to_string())
    }
}

// These are local recovery metadata, not portable project paths. Keep native
// units so a valid non-Unicode filename is not silently changed or rejected.
#[derive(Clone, Serialize, Deserialize)]
#[serde(tag = "platform", content = "value", rename_all = "snake_case")]
enum NativePath {
    Unix(Vec<u8>),
    Windows(Vec<u16>),
}
impl NativePath {
    fn capture(path: &Path) -> Self {
        #[cfg(unix)]
        {
            use std::os::unix::ffi::OsStrExt;
            Self::Unix(path.as_os_str().as_bytes().to_vec())
        }
        #[cfg(windows)]
        {
            use std::os::windows::ffi::OsStrExt;
            Self::Windows(path.as_os_str().encode_wide().collect())
        }
    }
    fn restore(&self) -> Result<PathBuf, CopyError> {
        match self {
            #[cfg(unix)]
            Self::Unix(bytes) => {
                use std::os::unix::ffi::OsStringExt;
                Ok(std::ffi::OsString::from_vec(bytes.clone()).into())
            }
            #[cfg(windows)]
            Self::Windows(wide) => {
                use std::os::windows::ffi::OsStringExt;
                Ok(std::ffi::OsString::from_wide(wide).into())
            }
            _ => Err(CopyError::ForeignPlatform),
        }
    }
    fn key(&self) -> String {
        let hash = fingerprint(&serde_json::to_vec(self).expect("native units serialize"));
        format!(
            "named-{}",
            hash.iter().map(|b| format!("{b:02x}")).collect::<String>()
        )
    }
}
#[derive(Clone, Serialize, Deserialize)]
struct NamedFile {
    path: NativePath,
    #[serde(deserialize_with = "required_fingerprint")]
    base: Option<Fingerprint>,
    #[serde(deserialize_with = "required_fingerprint")]
    base_content: Option<Fingerprint>,
    #[serde(deserialize_with = "required_fingerprint")]
    pending: Option<Fingerprint>,
}
// Null is an explicit state; an omitted field is damaged metadata, not a
// reservation to create a missing named file.
fn required_fingerprint<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<Option<Fingerprint>, D::Error> {
    Option::<Fingerprint>::deserialize(deserializer)
}
#[derive(Clone, Serialize, Deserialize)]
struct Record {
    version: u32,
    named: Option<NamedFile>,
    project: Project,
}

pub struct CopySummary {
    pub id: String,
    pub title: String,
    pub named_path: Option<PathBuf>,
    pub has_local_changes: bool,
}
pub struct CopyEntry {
    pub id: String,
    pub summary: Result<CopySummary, CopyError>,
}
/// The changed project is available for recovery selection; this enum never
/// authorizes overwriting either candidate or silently discarding local edits.
pub enum NamedStatus {
    Unchanged,
    Changed(Project),
    Missing,
    Unreadable(String),
}

#[derive(Clone)]
pub struct WorkingCopies {
    root: PathBuf,
}
pub struct WorkingCopy {
    path: PathBuf,
    id: String,
    _lock: fs::File,
    record: Record,
    preserved: Option<PathBuf>,
}
fn read_bounded(path: &Path, limit: usize) -> io::Result<Vec<u8>> {
    let mut bytes = Vec::new();
    fs::File::open(path)?
        .take(limit as u64 + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() > limit {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "File exceeds the supported size limit",
        ));
    }
    Ok(bytes)
}
fn record_bytes(record: &Record) -> Result<Vec<u8>, CopyError> {
    record.project.encode().map_err(CopyError::Invalid)?;
    let bytes = serde_json::to_vec(record).map_err(|e| CopyError::Invalid(e.to_string()))?;
    if bytes.len() > MAX_RECORD_BYTES {
        return Err(CopyError::Invalid(
            "Working copy exceeds its size limit".into(),
        ));
    }
    Ok(bytes)
}
fn local_changes(record: &Record) -> Result<bool, CopyError> {
    Ok(match &record.named {
        None => true,
        Some(named) => {
            Some(fingerprint(
                &record.project.encode().map_err(CopyError::Invalid)?,
            )) != named.base_content
        }
    })
}
fn write_record(path: &Path, record: &Record) -> Result<(), CopyError> {
    write_atomic(path, &record_bytes(record)?).map_err(CopyError::Io)
}
fn decode_record(path: &Path) -> Result<Record, CopyError> {
    if !fs::symlink_metadata(path)?.is_file() {
        return Err(CopyError::Invalid(
            "Working copy is not a regular file".into(),
        ));
    }
    let bytes = read_bounded(path, MAX_RECORD_BYTES).map_err(|e| {
        if e.kind() == io::ErrorKind::InvalidData {
            CopyError::Invalid(e.to_string())
        } else {
            e.into()
        }
    })?;
    // Check version before decoding fields so future formats are never treated
    // as corrupt and replaced by an older application.
    #[derive(Deserialize)]
    struct Header {
        version: u32,
    }
    let header: Header =
        serde_json::from_slice(&bytes).map_err(|e| CopyError::Invalid(e.to_string()))?;
    if header.version != VERSION {
        return Err(CopyError::UnsupportedVersion(header.version));
    }
    let record: Record =
        serde_json::from_slice(&bytes).map_err(|e| CopyError::Invalid(e.to_string()))?;
    record.project.encode().map_err(CopyError::Invalid)?;
    Ok(record)
}
fn read_record(path: &Path, id: &str) -> Result<Record, CopyError> {
    let record = decode_record(path)?;
    match &record.named {
        Some(n) if n.path.key() == id => {
            if n.path.restore().is_ok_and(|p| !p.is_absolute()) {
                return Err(CopyError::Invalid(
                    "Saved-file locations must be absolute".into(),
                ));
            }
        }
        None if id.starts_with("untitled-") => {}
        _ => {
            return Err(CopyError::Invalid(
                "Working copy identity does not match its saved-file location".into(),
            ));
        }
    }
    Ok(record)
}
fn valid_id(id: &str) -> bool {
    if let Some(hash) = id.strip_prefix("named-") {
        return hash.len() == 64
            && hash
                .bytes()
                .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase());
    }
    id.strip_prefix("untitled-").is_some_and(|s| {
        !s.is_empty() && s.len() <= 64 && s.bytes().all(|b| b.is_ascii_alphanumeric())
    })
}
fn archive_name(name: &str) -> bool {
    (name.starts_with("previous-working-") || name.starts_with("unreadable-working-"))
        && name.ends_with(".json")
        && name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_' || b == b'.')
        && !name.contains("..")
}
fn normalize_named(path: &Path) -> Result<PathBuf, CopyError> {
    match fs::canonicalize(path) {
        Ok(p) => Ok(p),
        Err(e) if e.kind() == io::ErrorKind::NotFound => {
            let parent = path
                .parent()
                .filter(|p| !p.as_os_str().is_empty())
                .unwrap_or(Path::new("."));
            let name = path
                .file_name()
                .ok_or_else(|| CopyError::Invalid("Missing filename".into()))?;
            Ok(fs::canonicalize(parent)?.join(name))
        }
        Err(e) => Err(e.into()),
    }
}
impl WorkingCopies {
    pub fn new(root: impl AsRef<Path>) -> Result<Self, CopyError> {
        fs::create_dir_all(root.as_ref())?;
        Ok(Self {
            root: fs::canonicalize(root.as_ref())?,
        })
    }
    fn acquire(&self, id: &str) -> Result<(PathBuf, fs::File), CopyError> {
        if !valid_id(id) {
            return Err(CopyError::Invalid("Invalid working copy identifier".into()));
        }
        let lock = fs::OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(self.root.join(format!("{id}.lock")))?;
        lock.try_lock().map_err(|_| CopyError::Busy)?;
        Ok((self.root.join(format!("{id}.json")), lock))
    }
    pub fn create_unnamed(&self, project: &Project) -> Result<WorkingCopy, CopyError> {
        for _ in 0..32 {
            let reservation = tempfile::Builder::new()
                .prefix(".reserve-")
                .tempfile_in(&self.root)?;
            let suffix = reservation
                .path()
                .file_name()
                .and_then(|n| n.to_str())
                .expect("generated filename")
                .strip_prefix(".reserve-")
                .unwrap();
            let id = format!("untitled-{suffix}");
            if let Some(copy) = self.create_unnamed_at(&id, project)? {
                return Ok(copy);
            }
        }
        Err(CopyError::Io(
            "Could not reserve a new working copy identifier".into(),
        ))
    }
    fn create_unnamed_at(
        &self,
        id: &str,
        project: &Project,
    ) -> Result<Option<WorkingCopy>, CopyError> {
        let (path, lock) = match self.acquire(id) {
            Ok(owned) => owned,
            Err(CopyError::Busy) => return Ok(None),
            Err(e) => return Err(e),
        };
        // Random names are candidates, not proof that an older record is absent.
        // Keep the ownership lock through the check and first atomic write.
        match fs::symlink_metadata(&path) {
            Ok(_) => return Ok(None),
            Err(e) if e.kind() == io::ErrorKind::NotFound => {}
            Err(e) => return Err(e.into()),
        }
        let record = Record {
            version: VERSION,
            named: None,
            project: project.clone(),
        };
        write_record(&path, &record)?;
        Ok(Some(WorkingCopy {
            path,
            id: id.into(),
            _lock: lock,
            record,
            preserved: None,
        }))
    }
    /// Existing local edits stay available even when the named file is missing
    /// or unreadable. Use named_status to choose between changed candidates.
    /// An unreadable local record is preserved before falling back to a valid
    /// named file. Future-version records are left untouched and return an error.
    pub fn open_named(&self, named: &Path) -> Result<WorkingCopy, CopyError> {
        let named = normalize_named(named)?;
        let native = NativePath::capture(&named);
        let id = native.key();
        let (path, lock) = self.acquire(&id)?;
        let mut preserved = None;
        let mut had_record = false;
        let existing = match fs::symlink_metadata(&path) {
            Ok(_) => {
                had_record = true;
                match read_record(&path, &id) {
                    Ok(record) => Some(record),
                    Err(CopyError::Invalid(_)) => None,
                    Err(e) => return Err(e),
                }
            }
            Err(e) if e.kind() == io::ErrorKind::NotFound => None,
            Err(e) => return Err(e.into()),
        };
        let record = if let Some(record) = existing {
            record
        } else {
            let bytes = read_bounded(&named, document::MAX_FILE_BYTES)?;
            let project = document::decode(&bytes).map_err(CopyError::Invalid)?;
            let record = Record {
                version: VERSION,
                named: Some(NamedFile {
                    path: native,
                    base: Some(fingerprint(&bytes)),
                    base_content: Some(fingerprint(&project.encode().map_err(CopyError::Invalid)?)),
                    pending: None,
                }),
                project,
            };
            if had_record {
                if !fs::symlink_metadata(&path)?.is_file() {
                    return Err(CopyError::Invalid(
                        "Cannot preserve a non-regular working copy".into(),
                    ));
                }
                let mut archive = tempfile::Builder::new()
                    .prefix("unreadable-working-")
                    .suffix(".json")
                    .tempfile_in(&self.root)?
                    .into_temp_path();
                fs::rename(&path, &archive)?;
                archive.disable_cleanup(true);
                preserved = Some(archive.to_path_buf());
            }
            write_record(&path, &record).map_err(|e| match &preserved {
                Some(archive) => CopyError::Io(format!(
                    "{e}. The original working copy remains preserved at {}",
                    archive.display()
                )),
                None => e,
            })?;
            record
        };
        let mut copy = WorkingCopy {
            path,
            id,
            _lock: lock,
            record,
            preserved,
        };
        copy.reconcile_pending()?;
        Ok(copy)
    }
    /// Reserve an explicitly selected Save As destination before any named write.
    /// Existing local records are archived, never silently discarded. The caller
    /// keeps its source copy until save_named succeeds on this returned owner.
    pub fn prepare_save_as(
        &self,
        destination: &Path,
        project: &Project,
    ) -> Result<WorkingCopy, CopyError> {
        project.encode().map_err(CopyError::Invalid)?;
        let named_path = normalize_named(destination)?;
        let native = NativePath::capture(&named_path);
        let id = native.key();
        let (path, lock) = self.acquire(&id)?;
        let original = match read_bounded(&named_path, document::MAX_FILE_BYTES) {
            Ok(bytes) => Some(bytes),
            Err(e) if e.kind() == io::ErrorKind::NotFound => None,
            Err(e) => return Err(e.into()),
        };
        let base = original.as_ref().map(|b| fingerprint(b));
        let base_content = original
            .as_ref()
            .and_then(|b| document::decode(b).ok())
            .and_then(|p| p.encode().ok())
            .map(|b| fingerprint(&b));
        let record = Record {
            version: VERSION,
            named: Some(NamedFile {
                path: native,
                base,
                base_content,
                pending: None,
            }),
            project: project.clone(),
        };
        let mut preserved = None;
        match fs::symlink_metadata(&path) {
            Ok(meta) => {
                if !meta.is_file() {
                    return Err(CopyError::Invalid(
                        "Cannot replace a non-regular working copy".into(),
                    ));
                }
                if let Err(CopyError::UnsupportedVersion(v)) = read_record(&path, &id) {
                    return Err(CopyError::UnsupportedVersion(v));
                }
                let mut archive = tempfile::Builder::new()
                    .prefix("previous-working-")
                    .suffix(".json")
                    .tempfile_in(&self.root)?
                    .into_temp_path();
                fs::rename(&path, &archive)?;
                archive.disable_cleanup(true);
                preserved = Some(archive.to_path_buf());
            }
            Err(e) if e.kind() == io::ErrorKind::NotFound => {}
            Err(e) => return Err(e.into()),
        }
        write_record(&path, &record).map_err(|e| match &preserved {
            Some(archive) => CopyError::Io(format!(
                "{e}. The previous working copy remains at {}",
                archive.display()
            )),
            None => e,
        })?;
        Ok(WorkingCopy {
            path,
            id,
            _lock: lock,
            record,
            preserved,
        })
    }
    /// Archives are immutable recovery sources. Restore creates a fresh unnamed
    /// owner and keeps the archive, never trusts its old saved-file association.
    pub fn restore_archive(&self, name: &str) -> Result<WorkingCopy, CopyError> {
        if !archive_name(name) {
            return Err(CopyError::Invalid("Invalid recovery archive name".into()));
        }
        let record = decode_record(&self.root.join(name))?;
        self.create_unnamed(&record.project)
    }
    pub fn list_archives(&self) -> Result<Vec<CopyEntry>, CopyError> {
        let mut entries = vec![];
        for item in fs::read_dir(&self.root)? {
            let item = item?;
            let Some(name) = item
                .file_name()
                .to_str()
                .map(str::to_owned)
                .filter(|n| archive_name(n))
            else {
                continue;
            };
            let id = format!("archive:{name}");
            let summary = decode_record(&item.path()).map(|r| CopySummary {
                id: id.clone(),
                title: format!("{} (previous copy)", r.project.title),
                named_path: None,
                has_local_changes: true,
            });
            entries.push(CopyEntry { id, summary });
        }
        entries.sort_by(|a, b| a.id.cmp(&b.id));
        Ok(entries)
    }
    pub fn resume(&self, id: &str) -> Result<WorkingCopy, CopyError> {
        let (path, lock) = self.acquire(id)?;
        let record = read_record(&path, id)?;
        let mut copy = WorkingCopy {
            path,
            id: id.into(),
            _lock: lock,
            record,
            preserved: None,
        };
        copy.reconcile_pending()?;
        Ok(copy)
    }
    /// Corrupt entries remain visible as errors. Archives and lock files are
    /// excluded; listing never repairs, removes or changes a record.
    pub fn list(&self) -> Result<Vec<CopyEntry>, CopyError> {
        let mut entries = Vec::new();
        for item in fs::read_dir(&self.root)? {
            let item = item?;
            let path = item.path();
            if path.extension().is_none_or(|x| x != "json") {
                continue;
            }
            let Some(id) = path
                .file_stem()
                .and_then(|s| s.to_str())
                .filter(|s| valid_id(s))
            else {
                continue;
            };
            let summary = read_record(&path, id).and_then(|r| {
                Ok(CopySummary {
                    id: id.into(),
                    has_local_changes: local_changes(&r)?,
                    title: r.project.title,
                    named_path: r.named.map(|n| n.path.restore()).transpose()?,
                })
            });
            entries.push(CopyEntry {
                id: id.into(),
                summary,
            });
        }
        entries.sort_by(|a, b| a.id.cmp(&b.id));
        Ok(entries)
    }
}
impl WorkingCopy {
    pub fn id(&self) -> &str {
        &self.id
    }
    pub fn local_path(&self) -> &Path {
        &self.path
    }
    pub fn project(&self) -> &Project {
        &self.record.project
    }
    pub fn preserved_copy(&self) -> Option<&Path> {
        self.preserved.as_deref()
    }
    pub fn named_path(&self) -> Result<Option<PathBuf>, CopyError> {
        self.record
            .named
            .as_ref()
            .map(|n| n.path.restore())
            .transpose()
    }
    pub fn owns_destination(&self, path: &Path) -> Result<bool, CopyError> {
        Ok(self
            .named_path()?
            .is_some_and(|p| normalize_named(path).is_ok_and(|candidate| candidate == p)))
    }
    pub fn has_local_changes(&self) -> Result<bool, CopyError> {
        local_changes(&self.record)
    }
    /// Explicit Save As consent refreshes the destination baseline once. A
    /// retry must reuse this captured baseline, never recapture outside changes.
    pub fn prepare_current_save_as(&mut self) -> Result<(), CopyError> {
        let path = self.named_path()?.ok_or(CopyError::NotNamed)?;
        let original = match read_bounded(&path, document::MAX_FILE_BYTES) {
            Ok(bytes) => Some(bytes),
            Err(e) if e.kind() == io::ErrorKind::NotFound => None,
            Err(e) => return Err(e.into()),
        };
        let mut record = self.record.clone();
        let named = record.named.as_mut().unwrap();
        named.base = original.as_ref().map(|b| fingerprint(b));
        named.base_content = original
            .as_ref()
            .and_then(|b| document::decode(b).ok())
            .and_then(|p| p.encode().ok())
            .map(|b| fingerprint(&b));
        named.pending = None;
        write_record(&self.path, &record)?;
        self.record = record;
        Ok(())
    }

    /// Open the current named file after retaining any different local document
    /// as an ordinary recoverable unnamed copy. Read before changing either.
    pub fn reload_named(&mut self, copies: &WorkingCopies) -> Result<(), CopyError> {
        let path = self.named_path()?.ok_or(CopyError::NotNamed)?;
        let bytes = read_bounded(&path, document::MAX_FILE_BYTES)?;
        let project = document::decode(&bytes).map_err(CopyError::Invalid)?;
        let content = fingerprint(&project.encode().map_err(CopyError::Invalid)?);
        let mut record = self.record.clone();
        if self.has_local_changes()? && self.project() != &project {
            let retained = copies.create_unnamed(self.project())?;
            self.preserved = Some(retained.local_path().to_owned());
        }
        record.project = project;
        let named = record.named.as_mut().unwrap();
        named.base = Some(fingerprint(&bytes));
        named.base_content = Some(content);
        named.pending = None;
        write_record(&self.path, &record)?;
        self.record = record;
        Ok(())
    }
    pub fn checkpoint(&mut self, project: &Project) -> Result<(), CopyError> {
        let mut record = self.record.clone();
        record.project = project.clone();
        write_record(&self.path, &record)?;
        self.record = record;
        Ok(())
    }
    fn reconcile_pending(&mut self) -> Result<(), CopyError> {
        let Some(named) = &self.record.named else {
            return Ok(());
        };
        let Some(pending) = named.pending else {
            return Ok(());
        };
        let path = named.path.restore()?;
        // Missing, unavailable or externally modified named files never justify
        // forgetting the local document or its write journal.
        let Ok(bytes) = read_bounded(&path, document::MAX_FILE_BYTES) else {
            return Ok(());
        };
        if fingerprint(&bytes) == pending {
            let mut record = self.record.clone();
            let named = record.named.as_mut().unwrap();
            named.base = Some(pending);
            named.base_content = Some(pending);
            named.pending = None;
            write_record(&self.path, &record)?;
            self.record = record;
        }
        Ok(())
    }
    pub fn named_status(&self) -> Result<NamedStatus, CopyError> {
        let named = self.record.named.as_ref().ok_or(CopyError::NotNamed)?;
        let path = named.path.restore()?;
        let bytes = match read_bounded(&path, document::MAX_FILE_BYTES) {
            Ok(bytes) => bytes,
            Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(NamedStatus::Missing),
            Err(e) => return Ok(NamedStatus::Unreadable(e.to_string())),
        };
        let actual = fingerprint(&bytes);
        if Some(actual) == named.base || Some(actual) == named.pending {
            return Ok(NamedStatus::Unchanged);
        }
        Ok(match document::decode(&bytes) {
            Ok(p) => NamedStatus::Changed(p),
            Err(e) => NamedStatus::Unreadable(e),
        })
    }
    /// Write the caller's immutable snapshot, which can be older than the latest
    /// local checkpoint. Never replace local project data with this snapshot.
    /// A journal makes a successful named write recognizable after interrupted
    /// acknowledgement. The caller may retry the same snapshot after any error.
    pub fn save_named(&mut self, snapshot: &Project) -> Result<(), CopyError> {
        self.save_named_using(snapshot, write_atomic)
    }
    fn save_named_using(
        &mut self,
        snapshot: &Project,
        mut write: impl FnMut(&Path, &[u8]) -> Result<(), String>,
    ) -> Result<(), CopyError> {
        self.reconcile_pending()?;
        let named = self.record.named.as_ref().ok_or(CopyError::NotNamed)?;
        let path = named.path.restore()?;
        let read_current = || match read_bounded(&path, document::MAX_FILE_BYTES) {
            Ok(bytes) => Ok::<_, CopyError>(Some(fingerprint(&bytes))),
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(e.into()),
        };
        let actual = read_current()?;
        if actual.is_none() && named.base.is_some() {
            return Err(CopyError::NamedMissing);
        }
        if actual != named.base {
            return Err(CopyError::NamedChanged);
        }
        let bytes = snapshot.encode().map_err(CopyError::Invalid)?;
        let target = fingerprint(&bytes);
        if Some(target) == actual {
            return Ok(());
        }
        let mut staged = self.record.clone();
        staged.named.as_mut().unwrap().pending = Some(target);
        write(&self.path, &record_bytes(&staged)?).map_err(CopyError::Io)?;
        self.record = staged;
        if read_current()? != actual {
            return Err(CopyError::NamedChanged);
        }
        write(&path, &bytes).map_err(CopyError::Io)?;
        let mut completed = self.record.clone();
        let named = completed.named.as_mut().unwrap();
        named.base = Some(target);
        named.base_content = Some(target);
        named.pending = None;
        write(&self.path, &record_bytes(&completed)?).map_err(CopyError::Io)?;
        self.record = completed;
        Ok(())
    }
    /// Explicitly discard only this local record. Keep its stable lock file so
    /// another process cannot acquire a different lock inode during close.
    pub fn discard(self) -> Result<(), CopyError> {
        match fs::remove_file(&self.path) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(e.into()),
        }
    }
}

#[cfg(test)]
mod crash_tests;
#[cfg(test)]
mod tests;
