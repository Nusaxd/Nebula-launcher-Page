//! Verified, parallel file downloads with progress reporting.

use crate::bail;
use crate::error::{Error, Result};
use crate::models::Progress;
use futures_util::{stream, StreamExt};
use sha1::{Digest, Sha1};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tauri::{AppHandle, Emitter};
use tokio::io::AsyncWriteExt;

#[derive(Clone, Debug)]
pub struct Job {
    pub url: String,
    pub path: PathBuf,
    pub sha1: Option<String>,
    pub size: u64,
}

pub fn emit_progress(app: &AppHandle, p: &Progress) {
    let _ = app.emit("install-progress", p.clone());
}

pub fn emit_stage(app: &AppHandle, id: &str, stage: &str) {
    emit_progress(
        app,
        &Progress {
            id: id.to_string(),
            stage: stage.to_string(),
            ..Default::default()
        },
    );
}

/// A file is considered fine if it exists and has the expected size.
/// (Freshly downloaded files are always verified against their SHA-1.)
async fn file_ok(job: &Job) -> bool {
    match tokio::fs::metadata(&job.path).await {
        Ok(m) if m.is_file() => job.size == 0 || m.len() == job.size,
        _ => false,
    }
}

async fn attempt(client: &reqwest::Client, job: &Job, counter: &AtomicU64, got: &mut u64) -> Result<()> {
    if let Some(parent) = job.path.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }
    let resp = client.get(&job.url).send().await?;
    let status = resp.status();
    if !status.is_success() {
        bail!("HTTP {} while downloading {}", status.as_u16(), job.url);
    }

    let mut tmp_name = job.path.file_name().map(|n| n.to_os_string()).unwrap_or_default();
    tmp_name.push(".part");
    let tmp = job.path.with_file_name(tmp_name);

    let mut file = tokio::fs::File::create(&tmp).await?;
    let mut hasher = Sha1::new();
    let mut stream = Box::pin(resp.bytes_stream());
    loop {
        let next = tokio::time::timeout(Duration::from_secs(30), stream.next())
            .await
            .map_err(|_| Error::msg(format!("Timed out downloading {}", job.url)))?;
        match next {
            Some(chunk) => {
                let chunk = chunk?;
                hasher.update(&chunk);
                file.write_all(&chunk).await?;
                let n = chunk.len() as u64;
                *got += n;
                counter.fetch_add(n, Ordering::Relaxed);
            }
            None => break,
        }
    }
    file.flush().await?;
    drop(file);

    if let Some(expected) = &job.sha1 {
        let actual = hex::encode(hasher.finalize());
        if !actual.eq_ignore_ascii_case(expected) {
            let _ = tokio::fs::remove_file(&tmp).await;
            bail!("Checksum mismatch for {} (expected {}, got {})", job.url, expected, actual);
        }
    }
    tokio::fs::rename(&tmp, &job.path).await?;
    Ok(())
}

/// Download a single file with retries. Adds downloaded bytes to `counter`.
pub async fn download_file(client: &reqwest::Client, job: &Job, counter: &AtomicU64) -> Result<()> {
    let mut last_err = Error::msg("download failed");
    for n in 0..3u64 {
        let mut got = 0u64;
        match attempt(client, job, counter, &mut got).await {
            Ok(()) => return Ok(()),
            Err(e) => {
                counter.fetch_sub(got, Ordering::Relaxed);
                last_err = e;
                tokio::time::sleep(Duration::from_millis(400 * (n + 1))).await;
            }
        }
    }
    Err(last_err)
}

/// Download `jobs` in parallel, skipping the ones that are already present.
pub async fn download_all(
    app: &AppHandle,
    client: &reqwest::Client,
    id: &str,
    stage: &str,
    jobs: Vec<Job>,
    concurrency: usize,
) -> Result<()> {
    let mut pending = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for j in jobs {
        if !seen.insert(j.path.clone()) {
            continue;
        }
        if !file_ok(&j).await {
            pending.push(j);
        }
    }
    if pending.is_empty() {
        return Ok(());
    }

    let total_files = pending.len() as u64;
    let total_bytes: u64 = pending.iter().map(|j| j.size).sum();
    let bytes = Arc::new(AtomicU64::new(0));
    let files = Arc::new(AtomicU64::new(0));
    let done = Arc::new(AtomicBool::new(false));

    let make_progress = {
        let id = id.to_string();
        let stage = stage.to_string();
        let bytes = bytes.clone();
        let files = files.clone();
        move || Progress {
            id: id.clone(),
            stage: stage.clone(),
            done_bytes: bytes.load(Ordering::Relaxed),
            total_bytes,
            done_files: files.load(Ordering::Relaxed),
            total_files,
        }
    };

    // Ticker that reports progress a few times per second.
    {
        let app = app.clone();
        let done = done.clone();
        let make_progress = make_progress.clone();
        tauri::async_runtime::spawn(async move {
            while !done.load(Ordering::Relaxed) {
                emit_progress(&app, &make_progress());
                tokio::time::sleep(Duration::from_millis(150)).await;
            }
        });
    }

    let mut results = Box::pin(
        stream::iter(pending)
            .map(|job| {
                let client = client.clone();
                let bytes = bytes.clone();
                let files = files.clone();
                async move {
                    let r = download_file(&client, &job, &bytes).await;
                    if r.is_ok() {
                        files.fetch_add(1, Ordering::Relaxed);
                    }
                    r
                }
            })
            .buffer_unordered(concurrency.max(1)),
    );

    let mut first_err: Option<Error> = None;
    while let Some(r) = results.next().await {
        if let Err(e) = r {
            first_err = Some(e);
            break;
        }
    }
    drop(results);
    done.store(true, Ordering::Relaxed);

    match first_err {
        Some(e) => Err(e),
        None => {
            emit_progress(app, &make_progress());
            Ok(())
        }
    }
}

/// Download a small JSON/text document into memory.
pub async fn get_json<T: serde::de::DeserializeOwned>(client: &reqwest::Client, url: &str) -> Result<T> {
    let resp = client.get(url).send().await?;
    let status = resp.status();
    if !status.is_success() {
        bail!("HTTP {} while fetching {}", status.as_u16(), url);
    }
    let bytes = resp.bytes().await?;
    Ok(serde_json::from_slice(&bytes)?)
}

pub async fn read_json_file<T: serde::de::DeserializeOwned>(path: &Path) -> Result<T> {
    let bytes = tokio::fs::read(path).await?;
    Ok(serde_json::from_slice(&bytes)?)
}
