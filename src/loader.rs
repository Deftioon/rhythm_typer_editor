use crate::data::{Beatmap, Meta, MetaDifficulty};
use anyhow::{anyhow, Context, Result};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::{Read, Write};
use std::path::Path;
use zip::write::FileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

#[derive(Debug, Clone)]
pub struct RtmDifficulty {
    pub meta: MetaDifficulty,
    pub beatmap: Beatmap,
}

#[derive(Debug, Clone)]
pub struct RtmPackage {
    pub meta: Meta,
    pub difficulties: Vec<RtmDifficulty>,
    pub other_files: BTreeMap<String, Vec<u8>>,
}

pub fn load_beatmap(path: &Path) -> Result<Beatmap> {
    let content = fs::read_to_string(path)?;
    let beatmap = serde_json::from_str(&content)?;
    Ok(beatmap)
}

pub fn save_beatmap(path: &Path, beatmap: &Beatmap) -> Result<()> {
    let json = serde_json::to_string_pretty(&beatmap)?;
    fs::write(path, json)?;
    Ok(())
}

pub fn load_meta(path: &Path) -> Result<Meta> {
    let content = fs::read_to_string(path)?;
    let meta = serde_json::from_str(&content)?;
    Ok(meta)
}

pub fn save_meta(path: &Path, meta: &Meta) -> Result<()> {
    let json = serde_json::to_string_pretty(&meta)?;
    fs::write(path, json)?;
    Ok(())
}

pub fn load_rtm(path: &Path) -> Result<RtmPackage> {
    let file = fs::File::open(path).with_context(|| format!("open rtm: {}", path.display()))?;
    let mut zip = ZipArchive::new(file).context("read zip")?;

    let mut entries: BTreeMap<String, Vec<u8>> = BTreeMap::new();
    for i in 0..zip.len() {
        let mut f = zip.by_index(i).context("zip entry")?;
        if f.is_dir() {
            continue;
        }
        let name = f.name().to_string();
        let mut buf = Vec::new();
        f.read_to_end(&mut buf)
            .with_context(|| format!("read zip entry: {}", name))?;
        entries.insert(name, buf);
    }

    let meta_bytes = entries
        .get("meta.json")
        .ok_or_else(|| anyhow!("rtm missing meta.json"))?;
    let meta: Meta = serde_json::from_slice(meta_bytes).context("parse meta.json")?;

    let mut difficulties: Vec<RtmDifficulty> = Vec::new();
    let mut diff_names_in_zip: BTreeSet<String> = BTreeSet::new();
    for d in &meta.difficulties {
        let fname = d.filename.clone();
        let bytes = entries.get(&fname).ok_or_else(|| {
            anyhow!(
                "rtm missing difficulty file '{}' referenced by meta.json",
                fname
            )
        })?;
        let beatmap: Beatmap = serde_json::from_slice(bytes)
            .with_context(|| format!("parse difficulty json: {}", fname))?;
        diff_names_in_zip.insert(fname);
        difficulties.push(RtmDifficulty {
            meta: d.clone(),
            beatmap,
        });
    }

    let mut other_files = BTreeMap::new();
    for (name, bytes) in entries {
        if name == "meta.json" {
            continue;
        }
        if diff_names_in_zip.contains(&name) {
            continue;
        }
        other_files.insert(name, bytes);
    }

    Ok(RtmPackage {
        meta,
        difficulties,
        other_files,
    })
}

pub fn save_rtm(path: &Path, package: &RtmPackage) -> Result<()> {
    let file = fs::File::create(path).with_context(|| format!("create rtm: {}", path.display()))?;
    let mut zip = ZipWriter::new(file);
    let options = FileOptions::default().compression_method(CompressionMethod::Deflated);

    // Ensure meta.difficulties matches package.difficulties.
    let mut meta = package.meta.clone();
    meta.difficulties = package
        .difficulties
        .iter()
        .map(|d| d.meta.clone())
        .collect();

    let meta_json = serde_json::to_vec_pretty(&meta).context("serialize meta.json")?;
    zip.start_file("meta.json", options)
        .context("write meta.json")?;
    zip.write_all(&meta_json).context("write meta.json")?;

    let mut diff_names: BTreeSet<String> = BTreeSet::new();
    for d in &package.difficulties {
        let name = d.meta.filename.clone();
        diff_names.insert(name.clone());
        let json = serde_json::to_vec_pretty(&d.beatmap)
            .with_context(|| format!("serialize difficulty: {}", name))?;
        zip.start_file(&name, options)
            .with_context(|| format!("write difficulty file: {}", name))?;
        zip.write_all(&json)
            .with_context(|| format!("write difficulty bytes: {}", name))?;
    }

    for (name, bytes) in &package.other_files {
        if name == "meta.json" || diff_names.contains(name) {
            continue;
        }
        zip.start_file(name, options)
            .with_context(|| format!("write other file: {}", name))?;
        zip.write_all(bytes)
            .with_context(|| format!("write other bytes: {}", name))?;
    }

    zip.finish().context("finalize zip")?;
    Ok(())
}

pub fn import_difficulty_json(path: &Path) -> Result<Beatmap> {
    load_beatmap(path)
}
