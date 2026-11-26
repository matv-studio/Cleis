//! Updates (D-74): the newest version published on GitHub, downloaded, checked and put in place
//! of the running copy, then Cleis starts again.
//!
//! The portable folder is replaced file by file: a running program cannot be overwritten on
//! Windows but can be renamed, so each old file becomes `<name>.old` (erased at the next start)
//! before the new one takes its name. Network and files: call off the interface thread.

use std::path::{Path, PathBuf};
use std::time::Duration;

/// Where versions are published (public repository).
const RELEASES: &str = "https://api.github.com/repos/matv-studio/Cleis/releases?per_page=30";
/// Looked again after this long while Cleis runs.
pub const EVERY: Duration = Duration::from_secs(6 * 3600);
/// Largest download accepted.
const MAX_BYTES: u64 = 300 * 1024 * 1024;

/// A version newer than the running one.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Release {
    /// `0.2.1`, without the `v`.
    pub version: String,
    pub prerelease: bool,
    /// The release's page.
    pub page: String,
    /// The package for this computer and its checksum, when one is published.
    pub package: Option<(String, String)>,
}

/// The running version.
pub fn current() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

/// The end of the package name for this computer (`None`: Cleis cannot replace itself here,
/// the release page opens instead).
fn package_suffix() -> Option<&'static str> {
    if cfg!(all(windows, target_arch = "x86_64")) {
        Some("-windows-x64-portable.zip")
    } else {
        None
    }
}

/// `0.2.1-beta.1` → comparable; `None` if it is not a version.
fn parse(v: &str) -> Option<semver::Version> {
    semver::Version::parse(v.trim().trim_start_matches('v')).ok()
}

/// The newest version above `current` in GitHub's answer (`releases`: its JSON list). Drafts
/// never count; pre-releases only with `pre`.
pub fn pick(
    releases: &serde_json::Value,
    current: &str,
    pre: bool,
    suffix: Option<&str>,
) -> Option<Release> {
    let now = parse(current)?;
    releases
        .as_array()?
        .iter()
        .filter(|r| !r["draft"].as_bool().unwrap_or(true))
        .filter(|r| pre || !r["prerelease"].as_bool().unwrap_or(false))
        .filter_map(|r| {
            let tag = r["tag_name"].as_str()?;
            let v = parse(tag)?;
            (v > now).then_some((v, r))
        })
        .max_by(|a, b| a.0.cmp(&b.0))
        .map(|(v, r)| {
            let assets = r["assets"].as_array().cloned().unwrap_or_default();
            let url_of = |name: &str| {
                assets
                    .iter()
                    .find(|a| a["name"].as_str() == Some(name))
                    .and_then(|a| a["browser_download_url"].as_str())
                    .map(str::to_owned)
            };
            let package = suffix.and_then(|suffix| {
                let name = assets
                    .iter()
                    .filter_map(|a| a["name"].as_str())
                    .find(|n| n.starts_with(&format!("Cleis-{v}-")) && n.ends_with(suffix))?;
                Some((url_of(name)?, url_of(&format!("{name}.sha256"))?))
            });
            Release {
                version: v.to_string(),
                prerelease: r["prerelease"].as_bool().unwrap_or(false),
                page: r["html_url"].as_str().unwrap_or_default().to_owned(),
                package,
            }
        })
}

fn agent() -> ureq::Agent {
    ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(300)))
        .user_agent(concat!("Cleis/", env!("CARGO_PKG_VERSION")))
        .build()
        .new_agent()
}

/// Asks GitHub for the newest version (network).
pub fn check(pre: bool) -> Result<Option<Release>, String> {
    let list: serde_json::Value = agent()
        .get(RELEASES)
        .header("Accept", "application/vnd.github+json")
        .call()
        .map_err(|e| format!("GitHub: {e}"))?
        .body_mut()
        .read_json()
        .map_err(|e| format!("GitHub: {e}"))?;
    Ok(pick(&list, current(), pre, package_suffix()))
}

fn download(url: &str) -> Result<Vec<u8>, String> {
    agent()
        .get(url)
        .call()
        .map_err(|e| format!("download: {e}"))?
        .body_mut()
        .with_config()
        .limit(MAX_BYTES)
        .read_to_vec()
        .map_err(|e| format!("download: {e}"))
}

/// The folder of the running copy.
fn folder() -> Result<PathBuf, String> {
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    exe.parent()
        .map(Path::to_path_buf)
        .ok_or_else(|| "no folder".to_owned())
}

/// Checks the package against its published checksum (`<hex>  <name>`).
pub fn verify(package: &[u8], sums: &str) -> Result<(), String> {
    let want = sums
        .split_whitespace()
        .next()
        .ok_or("empty checksum file")?
        .to_ascii_lowercase();
    let got: String = cleis_crypto::sha256(package)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    if got == want {
        Ok(())
    } else {
        Err("the download does not match its checksum".into())
    }
}

/// Puts the package's files in `dir` (the running copy's folder): an existing file is renamed
/// `<name>.old` first. Only plain files at the package's top level are taken.
pub fn install_into(package: &[u8], dir: &Path) -> Result<Vec<PathBuf>, String> {
    let mut zip = zip::ZipArchive::new(std::io::Cursor::new(package)).map_err(|e| e.to_string())?;
    // Everything read first: nothing is replaced if the package is damaged.
    let mut files = Vec::new();
    for i in 0..zip.len() {
        let mut f = zip.by_index(i).map_err(|e| e.to_string())?;
        if f.is_dir() {
            continue;
        }
        let Some(name) = f.enclosed_name().filter(|p| p.components().count() == 1) else {
            continue;
        };
        let mut bytes = Vec::with_capacity(f.size() as usize);
        std::io::copy(&mut f, &mut bytes).map_err(|e| e.to_string())?;
        files.push((name, bytes));
    }
    if !files
        .iter()
        .any(|(n, _)| n.to_string_lossy().eq_ignore_ascii_case(exe_name()))
    {
        return Err("the package holds no Cleis program".into());
    }
    let mut placed = Vec::new();
    for (name, bytes) in files {
        let target = dir.join(&name);
        let new = dir.join(format!("{}.new", name.display()));
        std::fs::write(&new, &bytes).map_err(|e| format!("{}: {e}", new.display()))?;
        if target.exists() {
            let old = dir.join(format!("{}.old", name.display()));
            let _ = std::fs::remove_file(&old);
            std::fs::rename(&target, &old).map_err(|e| format!("{}: {e}", target.display()))?;
        }
        std::fs::rename(&new, &target).map_err(|e| format!("{}: {e}", target.display()))?;
        placed.push(target);
    }
    Ok(placed)
}

fn exe_name() -> &'static str {
    if cfg!(windows) { "Cleis.exe" } else { "cleis" }
}

/// Downloads, checks and puts the new version in place (network and files).
pub fn install(r: &Release) -> Result<(), String> {
    let (url, sum_url) = r.package.as_ref().ok_or("no package for this computer")?;
    let package = download(url)?;
    let sums = String::from_utf8(download(sum_url)?).map_err(|e| e.to_string())?;
    verify(&package, &sums)?;
    install_into(&package, &folder()?)?;
    Ok(())
}

/// Starts the new copy (the caller then quits).
pub fn restart() -> Result<(), String> {
    let exe = folder()?.join(exe_name());
    std::process::Command::new(exe)
        .spawn()
        .map(|_| ())
        .map_err(|e| e.to_string())
}

/// At start: the files left by the previous update.
pub fn clean_up() {
    let Ok(dir) = folder() else { return };
    let Ok(list) = std::fs::read_dir(&dir) else {
        return;
    };
    for e in list.flatten() {
        let p = e.path();
        if p.extension().is_some_and(|x| x == "old") {
            let _ = std::fs::remove_file(p);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn rel(tag: &str, pre: bool, draft: bool) -> serde_json::Value {
        let v = tag.trim_start_matches('v');
        json!({
            "tag_name": tag, "prerelease": pre, "draft": draft,
            "html_url": format!("https://github.com/matv-studio/Cleis/releases/tag/{tag}"),
            "assets": [
                {"name": format!("Cleis-{v}-abc1234-windows-x64-portable.zip"),
                 "browser_download_url": format!("https://example-dl.com/{v}.zip")},
                {"name": format!("Cleis-{v}-abc1234-windows-x64-portable.zip.sha256"),
                 "browser_download_url": format!("https://example-dl.com/{v}.zip.sha256")},
            ]
        })
    }

    #[test]
    fn picks_the_newest_allowed_version() {
        let list = json!([
            rel("v0.2.0", true, false),
            rel("v0.3.0", true, false),
            rel("v0.2.5", false, false),
            rel("v0.4.0", false, true),
        ]);
        let suffix = Some("-windows-x64-portable.zip");
        let r = pick(&list, "0.2.0", true, suffix).unwrap();
        assert_eq!(r.version, "0.3.0");
        assert!(r.prerelease);
        assert_eq!(
            r.package,
            Some((
                "https://example-dl.com/0.3.0.zip".into(),
                "https://example-dl.com/0.3.0.zip.sha256".into()
            ))
        );
        // Without pre-releases: the stable one; the draft never.
        assert_eq!(
            pick(&list, "0.2.0", false, suffix).unwrap().version,
            "0.2.5"
        );
        assert_eq!(pick(&list, "0.3.0", true, suffix), None);
        // No package for this computer: the page only.
        assert_eq!(pick(&list, "0.2.0", true, None).unwrap().package, None);
    }

    #[test]
    fn checks_the_package() {
        let data = b"package";
        let hex: String = cleis_crypto::sha256(data)
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect();
        assert!(verify(data, &format!("{}  x.zip\r\n", hex.to_uppercase())).is_ok());
        assert!(verify(b"other", &format!("{hex}  x.zip")).is_err());
    }

    #[test]
    fn replaces_the_files_and_keeps_the_old_ones() {
        use std::io::Write;
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join(exe_name()), b"old program").unwrap();
        std::fs::write(dir.path().join("keep.txt"), b"mine").unwrap();
        let mut buf = std::io::Cursor::new(Vec::new());
        {
            let mut z = zip::ZipWriter::new(&mut buf);
            let o = zip::write::SimpleFileOptions::default();
            z.start_file(exe_name(), o).unwrap();
            z.write_all(b"new program").unwrap();
            z.start_file("README.md", o).unwrap();
            z.write_all(b"readme").unwrap();
            z.start_file("../escape.txt", o).unwrap();
            z.write_all(b"no").unwrap();
            z.finish().unwrap();
        }
        let placed = install_into(buf.get_ref(), dir.path()).unwrap();
        assert_eq!(placed.len(), 2);
        let read = |n: &str| std::fs::read(dir.path().join(n)).unwrap();
        assert_eq!(read(exe_name()), b"new program");
        assert_eq!(read(&format!("{}.old", exe_name())), b"old program");
        assert_eq!(read("README.md"), b"readme");
        assert_eq!(read("keep.txt"), b"mine");
        assert!(!dir.path().parent().unwrap().join("escape.txt").exists());
        // A package without the program changes nothing.
        let mut bad = std::io::Cursor::new(Vec::new());
        {
            let mut z = zip::ZipWriter::new(&mut bad);
            z.start_file("README.md", zip::write::SimpleFileOptions::default())
                .unwrap();
            z.write_all(b"x").unwrap();
            z.finish().unwrap();
        }
        assert!(install_into(bad.get_ref(), dir.path()).is_err());
        assert_eq!(read("README.md"), b"readme");
    }
}
