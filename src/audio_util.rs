use crate::loader::RtmPackage;
use std::{fs, path::PathBuf};

fn normalize_zip_path(p: &str) -> String {
    p.replace('\\', "/")
}

fn find_audio_entry_name(pkg: &RtmPackage) -> Option<String> {
    let wanted = normalize_zip_path(pkg.meta.audioFile.trim());
    if wanted.is_empty() {
        return None;
    }

    if pkg.other_files.contains_key(&wanted) {
        return Some(wanted);
    }

    let wanted_base = wanted.split('/').last().unwrap_or(&wanted);
    for name in pkg.other_files.keys() {
        let n = normalize_zip_path(name);
        if n.split('/').last().unwrap_or(&n) == wanted_base {
            return Some(name.clone());
        }
    }
    None
}

pub fn ensure_audio_extracted(pkg: &RtmPackage, rtm_path: &PathBuf) -> anyhow::Result<Option<String>> {
    let Some(entry_name) = find_audio_entry_name(pkg) else {
        return Ok(None);
    };
    let bytes = pkg
        .other_files
        .get(&entry_name)
        .ok_or_else(|| anyhow::anyhow!("audio entry not found in package: {}", entry_name))?;

    let stem = rtm_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("map");
    let out_path = PathBuf::from("target")
        .join("rtm_cache")
        .join(stem)
        .join(normalize_zip_path(&entry_name));

    if let Some(parent) = out_path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&out_path, bytes)?;

    let rel = out_path.to_string_lossy().replace('\\', "/");
    Ok(Some(rel))
}
