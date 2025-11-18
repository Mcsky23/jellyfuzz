use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use swc_ecma_visit::{VisitWith, swc_ecma_ast::*};

use anyhow::{Context, Result};
use rand::Rng;
use serde::{Deserialize, Serialize};
use tokio::fs;

const METADATA_FILE: &str = "metadata.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CorpusEntry {
    pub id: u64,
    /// Path relative to the corpus root directory.
    pub path: PathBuf,
    pub fingerprint: u64,
    pub edge_hits: Vec<u32>,
    pub size_bytes: usize,
    pub total_reward: f64,
    pub last_reward: f64,
    pub exec_time_ms: Duration,
    pub num_mutations: u64,
    pub last_selected_ts: Option<u64>,
}

#[derive(Debug, Serialize, Deserialize)]
struct CorpusMetadata {
    next_id: u64,
    entries: Vec<CorpusEntry>,
}

#[derive(Debug, Clone)]
pub struct CorpusSelection {
    pub id: u64,
    pub path: PathBuf,
}

pub struct CorpusManager {
    root: PathBuf,
    metadata_path: PathBuf,
    entries: Vec<CorpusEntry>,
    next_id: u64,
}

impl CorpusManager {
    pub async fn load(root: PathBuf) -> Result<Self> {
        if fs::metadata(&root).await.is_err() {
            fs::create_dir_all(&root)
                .await
                .with_context(|| format!("failed to create corpus directory {:?}", root))?;
        }

        let metadata_path = root.join(METADATA_FILE);
        let (entries, next_id) = if fs::metadata(&metadata_path).await.is_ok() {
            let blob = fs::read(&metadata_path)
                .await
                .with_context(|| format!("failed to read metadata {:?}", metadata_path))?;
            if blob.is_empty() {
                (Vec::new(), 0)
            } else {
                let meta: CorpusMetadata = serde_json::from_slice(&blob)
                    .with_context(|| "failed to deserialize corpus metadata".to_string())?;
                let max_id = meta.entries.iter().map(|e| e.id).max().unwrap_or(0);
                let next = meta.next_id.max(max_id.saturating_add(1));
                (meta.entries, next)
            }
        } else {
            (Vec::new(), 0)
        };

        Ok(Self {
            root,
            metadata_path,
            entries,
            next_id,
        })
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    #[allow(dead_code)]
    pub fn entries(&self) -> &[CorpusEntry] {
        &self.entries
    }

    pub fn contains_fingerprint(&self, fingerprint: u64) -> bool {
        self.entries
            .iter()
            .any(|entry| entry.fingerprint == fingerprint)
    }

    pub fn pick_random(&mut self) -> Option<CorpusSelection> {
        if self.entries.is_empty() {
            return None;
        }
        let mut rng = rand::rng();
        let idx = rng.random_range(0..self.entries.len());
        let entry = &mut self.entries[idx];
        entry.num_mutations = entry.num_mutations.saturating_add(1);
        entry.last_selected_ts = Some(current_timestamp());
        Some(CorpusSelection {
            id: entry.id,
            path: self.root.join(&entry.path),
        })
    }

    pub async fn record_result(&mut self, id: u64, reward: f64, exec_time_ms: Duration) -> Result<()> {
        if let Some(entry) = self.entries.iter_mut().find(|entry| entry.id == id) {
            entry.last_reward = reward;
            entry.total_reward += reward;
            entry.exec_time_ms = exec_time_ms;
            self.persist().await?; // TODO: optimize by only calling this function after a number of updates
        }
        Ok(())
    }

    pub async fn add_entry(
        &mut self,
        script_bytes: &[u8],
        edge_hits: Vec<u32>,
        reward: f64,
        exec_time_ms: Duration,
        is_timeout: bool,
    ) -> Result<Option<CorpusEntry>> {
        let fingerprint = compute_fingerprint(script_bytes, &edge_hits);
        if self.contains_fingerprint(fingerprint) {
            return Ok(None);
        }

        let id = self.next_id;
        self.next_id += 1;

        let file_name = format!("seed_{id}.js");
        let relative_path = PathBuf::from(&file_name);
        let absolute_path = self.root.join(&relative_path);

        if is_timeout {
            println!("Storing timeout corpus entry {:?}", file_name);
            // For timeouts, we store the script in a separate directory
            let timeout_dir = self.root.join("timeouts");
            if fs::metadata(&timeout_dir).await.is_err() {
                fs::create_dir_all(&timeout_dir).await.with_context(|| {
                    format!("failed to create timeout directory {:?}", timeout_dir)
                })?;
            }
            let timeout_path = timeout_dir.join(&file_name);
            fs::write(&timeout_path, script_bytes)
                .await
                .with_context(|| {
                    format!("failed to write timeout corpus entry {:?}", timeout_path)
                })?;
            return Ok(None);
        }

        fs::write(&absolute_path, script_bytes)
            .await
            .with_context(|| format!("failed to write corpus entry {:?}", absolute_path))?;

        let entry = CorpusEntry {
            id,
            path: relative_path,
            fingerprint,
            edge_hits,
            size_bytes: script_bytes.len(),
            total_reward: reward.max(0.0),
            last_reward: reward,
            exec_time_ms,
            num_mutations: 0,
            last_selected_ts: None,
        };
        self.entries.push(entry.clone());
        self.persist().await?; // TODO: optimize by only calling this function after a number of additions
        Ok(Some(entry))
    }

    pub async fn remove_entry(&mut self, id: u64) -> Result<()> {
        if let Some(pos) = self.entries.iter().position(|entry| entry.id == id) {
            let entry = self.entries.remove(pos);
            let absolute_path = self.root.join(&entry.path);
            if fs::metadata(&absolute_path).await.is_ok() {
                fs::remove_file(&absolute_path)
                    .await
                    .with_context(|| format!("failed to remove corpus entry {:?}", absolute_path))?;
            }
            self.persist().await?;
        }
        Ok(())
    }

    async fn persist(&self) -> Result<()> {
        let data = CorpusMetadata {
            next_id: self.next_id,
            entries: self.entries.clone(),
        };
        let blob = serde_json::to_vec_pretty(&data)
            .with_context(|| "failed to serialize corpus metadata".to_string())?;
        let temp_path = self.metadata_path.with_extension("json.tmp");
        fs::write(&temp_path, blob)
            .await
            .with_context(|| format!("failed to write temp metadata {:?}", temp_path))?;
        fs::rename(&temp_path, &self.metadata_path)
            .await
            .with_context(|| "failed to atomically update metadata file".to_string())?;
        Ok(())
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub async fn get_random_script(&self) -> Result<Option<Script>> {
        if self.entries.is_empty() {
            return Ok(None);
        }
        let mut rng = rand::rng();
        let idx = rng.random_range(0..self.entries.len());
        let entry = &self.entries[idx];
        let absolute_path = self.root.join(&entry.path);
        let script_bytes = fs::read(&absolute_path)
            .await
            .with_context(|| format!("failed to read corpus entry {:?}", absolute_path))?;
        let script = crate::parsing::parser::parse_js(String::from_utf8_lossy(&script_bytes).to_string())
            .with_context(|| format!("failed to parse corpus entry {:?}", absolute_path))?;
        Ok(Some(script))
    }
}

fn compute_fingerprint(script_bytes: &[u8], edge_hits: &[u32]) -> u64 {
    let mut hasher = DefaultHasher::new();
    script_bytes.hash(&mut hasher);
    edge_hits.hash(&mut hasher);
    hasher.finish()
}

fn current_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|dur| dur.as_secs())
        .unwrap_or(0)
}
