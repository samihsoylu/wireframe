// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
//! Early, fail-closed memory-only diagnostic opt-in. No normal storage resolver here.
use super::diagnostics::Trace;
use serde::{Deserialize, Serialize};
use std::{
    ffi::OsString,
    fs::{self, OpenOptions},
    io::Read,
    path::PathBuf,
    sync::Arc,
};
pub(crate) const NORMAL_BUNDLE: &str = "nl.samihsoylu.wireframe";
#[derive(Clone, Default)]
pub(crate) enum AppEnvironment {
    #[default]
    Normal,
    Diagnostic(Arc<Trace>),
}
#[derive(Clone, Debug, Default)]
pub(crate) struct BundleIdentity {
    pub marked: bool,
    pub id: Option<String>,
    pub run: Option<String>,
    pub root: Option<PathBuf>,
}
#[derive(Clone, Debug)]
pub(crate) struct Request {
    pub run: String,
    pub root: PathBuf,
    pub bundle: String,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Marker {
    version: u32,
    run_id: String,
    bundle_id: String,
    root: PathBuf,
}
impl AppEnvironment {
    pub(crate) fn trace(&self) -> Option<&Arc<Trace>> {
        match self {
            Self::Normal => None,
            Self::Diagnostic(t) => Some(t),
        }
    }
    pub(crate) fn normal_only<T>(&self, action: impl FnOnce() -> T) -> Option<T> {
        if self.diagnostic() {
            None
        } else {
            Some(action())
        }
    }
    pub(crate) fn diagnostic(&self) -> bool {
        self.trace().is_some()
    }
    pub(crate) fn allow(&self, effect: &'static str) -> bool {
        if let Some(t) = self.trace() {
            t.record("effect_denied", serde_json::json!({"effect":effect}));
            false
        } else {
            true
        }
    }
    pub(crate) fn install_backend(&self) -> Result<(), String> {
        if let Some(trace) = self.trace() {
            egui_winit::diagnostics::install(egui_winit::diagnostics::Bridge::new(trace.clone()))
                .map_err(str::to_owned)?;
        }
        Ok(())
    }
}
pub(crate) fn parse(
    args: &[OsString],
    debug: bool,
    bundle: &BundleIdentity,
    legacy_trace: bool,
) -> Result<Option<Request>, String> {
    let requested = args
        .iter()
        .any(|a| a.to_string_lossy().starts_with("--diagnostic"));
    if !requested {
        if bundle.marked
            || bundle
                .id
                .as_ref()
                .is_some_and(|id| id.starts_with(&format!("{NORMAL_BUNDLE}.diagnostic.")))
        {
            return Err("Diagnostic bundle requires explicit mode, run and root".into());
        }
        return Ok(None);
    }
    if !debug {
        return Err("Diagnostic mode requires a debug-enabled binary".into());
    }
    if !bundle.marked {
        return Err("Diagnostic mode requires a marked private bundle".into());
    }
    if legacy_trace {
        return Err("Legacy input trace is forbidden in diagnostic mode".into());
    }
    let mut mode = None;
    let mut run = None;
    let mut root = None;
    let mut i = 0;
    while i < args.len() {
        let key = args[i].to_str().ok_or("Invalid diagnostic option")?;
        i += 1;
        let value = args
            .get(i)
            .ok_or("Diagnostic option is missing its value")?;
        i += 1;
        match key {
            "--diagnostic-mode" if mode.is_none() => {
                mode = Some(value.to_str().ok_or("Invalid diagnostic mode")?.to_owned())
            }
            "--diagnostic-run-id" if run.is_none() => {
                run = Some(value.to_str().ok_or("Invalid run ID")?.to_owned())
            }
            "--diagnostic-root" if root.is_none() => root = Some(PathBuf::from(value)),
            _ => return Err("Unknown, duplicate or conflicting diagnostic arguments".into()),
        }
    }
    if mode.as_deref() != Some("no-link") {
        return Err("Only no-link diagnostic mode is supported".into());
    }
    let run = run.ok_or("Missing diagnostic run ID")?;
    if run.is_empty()
        || run.len() > 40
        || !run.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-')
    {
        return Err("Invalid diagnostic run ID".into());
    }
    let root = root.ok_or("Missing diagnostic root")?;
    if !root.is_absolute() || root.to_str().is_none() {
        return Err("Diagnostic root must be absolute UTF-8".into());
    }
    let expected = format!("{NORMAL_BUNDLE}.diagnostic.{run}");
    if bundle.id.as_deref() != Some(expected.as_str())
        || bundle.run.as_deref() != Some(run.as_str())
        || bundle.root.as_ref() != Some(&root)
    {
        return Err("Diagnostic bundle/run/root identity mismatch".into());
    }
    Ok(Some(Request {
        run,
        root,
        bundle: expected,
    }))
}
#[cfg(unix)]
fn owned(metadata: &fs::Metadata) -> bool {
    use std::os::unix::fs::{MetadataExt, PermissionsExt};
    unsafe extern "C" {
        fn geteuid() -> u32;
    }
    metadata.uid() == unsafe { geteuid() } && metadata.permissions().mode() & 0o077 == 0
}
#[cfg(not(unix))]
fn owned(_metadata: &fs::Metadata) -> bool {
    false
}
pub(crate) fn open(request: Request) -> Result<AppEnvironment, String> {
    let root = &request.root;
    let metadata = fs::symlink_metadata(root).map_err(|e| e.to_string())?;
    if !metadata.is_dir()
        || metadata.file_type().is_symlink()
        || !owned(&metadata)
        || fs::canonicalize(root).map_err(|e| e.to_string())? != *root
    {
        return Err("Diagnostic root must be canonical, private and owned".into());
    }
    let marker_path = root.join("diagnostic.json");
    let metadata = fs::symlink_metadata(&marker_path).map_err(|e| e.to_string())?;
    if !metadata.is_file()
        || metadata.file_type().is_symlink()
        || !owned(&metadata)
        || metadata.len() > 4096
    {
        return Err("Invalid diagnostic marker ownership/type/size".into());
    }
    let mut bytes = Vec::new();
    fs::File::open(marker_path)
        .map_err(|e| e.to_string())?
        .take(4097)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    let marker: Marker = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
    if marker.version != 1
        || marker.run_id != request.run
        || marker.bundle_id != request.bundle
        || marker.root != *root
    {
        return Err("Diagnostic marker identity mismatch".into());
    }
    if fs::read_dir(root)
        .map_err(|e| e.to_string())?
        .any(|e| e.map_or(true, |e| e.file_name() != "diagnostic.json"))
    {
        return Err("Diagnostic root is not fresh".into());
    }
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let file = options
        .open(root.join("trace.jsonl"))
        .map_err(|e| e.to_string())?;
    let trace = Arc::new(Trace::new(request.run, file));
    trace.record(
        "launch_validated",
        serde_json::json!({"keyboard_only":true}),
    );
    trace.flush();
    if !trace.healthy() {
        return Err("Owned diagnostic trace cannot be written".into());
    }
    Ok(AppEnvironment::Diagnostic(trace))
}
#[cfg(target_os = "macos")]
pub(crate) fn bundle_identity() -> BundleIdentity {
    use objc2_foundation::{NSBundle, NSString};
    let bundle = NSBundle::mainBundle();
    let string = |key: &str| {
        bundle
            .objectForInfoDictionaryKey(&NSString::from_str(key))
            .and_then(|v| v.downcast_ref::<NSString>().map(ToString::to_string))
    };
    let marked = bundle
        .objectForInfoDictionaryKey(&NSString::from_str("WireframeDiagnostic"))
        .and_then(|v| {
            v.downcast_ref::<objc2_foundation::NSNumber>()
                .map(|n| n.boolValue())
        })
        .unwrap_or(false);
    BundleIdentity {
        marked,
        id: bundle.bundleIdentifier().map(|s| s.to_string()),
        run: string("WireframeDiagnosticRunID"),
        root: string("WireframeDiagnosticRoot").map(PathBuf::from),
    }
}
#[cfg(not(target_os = "macos"))]
pub(crate) fn bundle_identity() -> BundleIdentity {
    BundleIdentity::default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;
    fn args(root: &Path) -> Vec<OsString> {
        vec![
            "--diagnostic-mode".into(),
            "no-link".into(),
            "--diagnostic-run-id".into(),
            "run1".into(),
            "--diagnostic-root".into(),
            root.into(),
        ]
    }
    fn identity(root: &Path) -> BundleIdentity {
        BundleIdentity {
            marked: true,
            id: Some(format!("{NORMAL_BUNDLE}.diagnostic.run1")),
            run: Some("run1".into()),
            root: Some(root.into()),
        }
    }
    #[test]
    fn pure_launch_rejects_before_any_backend_factory() {
        let root = Path::new("/private/owned");
        let good = identity(root);
        let a = args(root);
        assert!(
            parse(&[], true, &BundleIdentity::default(), false)
                .unwrap()
                .is_none()
        );
        for (argv, debug, bundle, legacy) in [
            (vec![], true, good.clone(), false),
            (a.clone(), false, good.clone(), false),
            (a.clone(), true, BundleIdentity::default(), false),
            (a.clone(), true, good.clone(), true),
            (
                vec!["--diagnostic-unknown".into()],
                true,
                good.clone(),
                false,
            ),
        ] {
            assert!(
                parse(&argv, debug, &bundle, legacy)
                    .map(|_| panic!("backend factory must not follow rejected launch"))
                    .is_err()
            );
        }
        let mut mismatch = good.clone();
        mismatch.root = Some("/different".into());
        assert!(parse(&a, true, &mismatch, false).is_err());
        let mut extra = a.clone();
        extra.push("ordinary.wireframe".into());
        assert!(parse(&extra, true, &good, false).is_err());
        let mut duplicate = a.clone();
        duplicate.extend(["--diagnostic-root".into(), root.into()]);
        assert!(parse(&duplicate, true, &good, false).is_err());
        assert!(parse(&a, true, &good, false).unwrap().is_some());
    }
    #[cfg(unix)]
    fn fixture() -> (tempfile::TempDir, Request) {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().canonicalize().unwrap();
        fs::set_permissions(&root, fs::Permissions::from_mode(0o700)).unwrap();
        let b = identity(&root);
        let request = parse(&args(&root), true, &b, false).unwrap().unwrap();
        let marker = Marker {
            version: 1,
            run_id: request.run.clone(),
            bundle_id: request.bundle.clone(),
            root: root.clone(),
        };
        fs::write(
            root.join("diagnostic.json"),
            serde_json::to_vec(&marker).unwrap(),
        )
        .unwrap();
        fs::set_permissions(
            root.join("diagnostic.json"),
            fs::Permissions::from_mode(0o600),
        )
        .unwrap();
        (dir, request)
    }
    #[test]
    #[cfg(unix)]
    fn owned_marker_and_create_new_trace_fail_closed() {
        let (_dir, request) = fixture();
        let root = request.root.clone();
        let env = open(request.clone()).unwrap();
        assert!(env.diagnostic());
        assert!(root.join("trace.jsonl").is_file());
        assert!(open(request).is_err());
        let (_dir, request) = fixture();
        std::os::unix::fs::symlink("/not-owned", request.root.join("trace.jsonl")).unwrap();
        assert!(open(request).is_err());
        let (_dir, request) = fixture();
        fs::write(request.root.join("unrelated"), b"keep").unwrap();
        assert!(open(request.clone()).is_err());
        assert_eq!(fs::read(request.root.join("unrelated")).unwrap(), b"keep");
    }
    #[cfg(unix)]
    pub(crate) fn diagnostic_fixture() -> (tempfile::TempDir, AppEnvironment) {
        let (dir, request) = fixture();
        (dir, open(request).unwrap())
    }
}
#[cfg(all(test, unix))]
pub(super) use tests::diagnostic_fixture;
