use crate::{authorize, AppState};
use axum::{extract::State, http::{HeaderMap, StatusCode}, routing::get, Json, Router};
use serde::Serialize;
use std::{env, fs, path::PathBuf, process::Command};

const LOGICAL_CAPACITY_BYTES: u64 = 999_000_000_000_000;
const DIRECTORIES: &[&str] = &["vta2/cache", "vta2/ram-spill", "vta2/nvme", "vta2/archive", "vta2/models", "vta2/datasets", "vta2/checkpoints"];

#[derive(Serialize)]
struct StorageStatus {
    product: &'static str,
    version: &'static str,
    logical_capacity_bytes: u64,
    logical_capacity_tb: f64,
    physical_filesystem_total_bytes: Option<u64>,
    physical_filesystem_available_bytes: Option<u64>,
    physical_filesystem_used_bytes: Option<u64>,
    mount_path: String,
    mount_exists: bool,
    directories_ready: bool,
    tiers: Vec<Tier>,
    warning: &'static str,
}
#[derive(Serialize)]
struct Tier { name: &'static str, path: String, purpose: &'static str }

fn storage_root() -> PathBuf {
    PathBuf::from(env::var("APOLO_STORAGE_DIR").unwrap_or_else(|_| "/data/apolo".into()))
}
fn filesystem_capacity(path: &str) -> (Option<u64>, Option<u64>) {
    let output = Command::new("df").args(["-B1", "--output=size,avail", path]).output().ok();
    let Some(output) = output.filter(|o| o.status.success()) else { return (None, None) };
    let text = String::from_utf8_lossy(&output.stdout);
    let values: Vec<u64> = text.lines().skip(1).flat_map(|line| line.split_whitespace()).filter_map(|v| v.parse().ok()).collect();
    (values.first().copied(), values.get(1).copied())
}
async fn status(State(state): State<AppState>, headers: HeaderMap) -> Result<Json<StorageStatus>, StatusCode> {
    authorize(&state, &headers).await?;
    let root = storage_root();
    let exists = root.is_dir();
    let path = root.to_string_lossy().to_string();
    let (total, available) = if exists { filesystem_capacity(&path) } else { (None, None) };
    let used = total.zip(available).map(|(t,a)| t.saturating_sub(a));
    let tiers = vec![
        Tier { name: "VTA Cache", path: root.join("vta2/cache").display().to_string(), purpose: "Cache de blocos quentes; limitado pela RAM/volume real" },
        Tier { name: "VTA RAM Spill", path: root.join("vta2/ram-spill").display().to_string(), purpose: "Área de spill em disco; não é RAM física" },
        Tier { name: "VTA NVMe", path: root.join("vta2/nvme").display().to_string(), purpose: "Pesos, shards e arquivos de trabalho no volume conectado" },
        Tier { name: "VTA Archive", path: root.join("vta2/archive").display().to_string(), purpose: "Arquivamento de datasets e checkpoints no mesmo volume" },
        Tier { name: "VTA Models", path: root.join("vta2/models").display().to_string(), purpose: "Artefatos e shards de modelos" },
        Tier { name: "VTA Datasets", path: root.join("vta2/datasets").display().to_string(), purpose: "Datasets e manifests" },
        Tier { name: "VTA Checkpoints", path: root.join("vta2/checkpoints").display().to_string(), purpose: "Checkpoints recuperáveis" },
    ];
    Ok(Json(StorageStatus {
        product: "APOLO VTA 2.0", version: "2.0.0",
        logical_capacity_bytes: LOGICAL_CAPACITY_BYTES,
        logical_capacity_tb: 999.0,
        physical_filesystem_total_bytes: total,
        physical_filesystem_available_bytes: available,
        physical_filesystem_used_bytes: used,
        mount_path: path, mount_exists: exists, directories_ready: DIRECTORIES.iter().all(|d| root.join(d).is_dir()),
        tiers,
        warning: "999 TB é quota lógica configurada, não espaço físico provisionado. O armazenamento utilizável é limitado pela capacidade real do volume montado.",
    }))
}
async fn initialize(State(state): State<AppState>, headers: HeaderMap) -> Result<Json<StorageStatus>, StatusCode> {
    authorize(&state, &headers).await?;
    let root = storage_root();
    for dir in DIRECTORIES {
        fs::create_dir_all(root.join(dir)).map_err(|_| StatusCode::INSUFFICIENT_STORAGE)?;
    }
    status(State(state), headers).await
}
pub(crate) fn router() -> Router<AppState> {
    Router::new()
        .route("/api/v1/vta2/storage", get(status))
        .route("/api/v1/vta2/initialize", axum::routing::post(initialize))
}
