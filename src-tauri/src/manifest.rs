use serde::{Deserialize, Serialize};

pub const MANIFEST_URL: &str =
    "https://raw.githubusercontent.com/kaish1ro/SimulationModPack/main/manifest.json";

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ModEntry {
    pub name: String,
    pub url: String,
    pub sha256: String,
    pub size: u64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Manifest {
    pub version: String,
    pub forge: String,
    pub java_min: u32,
    pub config_repo: String,
    pub config_branch: String,
    pub mods: Vec<ModEntry>,
}

/// Скачивает и парсит manifest.json с GitHub
pub async fn fetch_remote() -> anyhow::Result<Manifest> {
    let client = reqwest::Client::new();
    let manifest = client
        .get(MANIFEST_URL)
        .send()
        .await?
        .json::<Manifest>()
        .await?;
    Ok(manifest)
}

/// Читает локальный manifest из директории установки
pub fn read_local(install_dir: &std::path::Path) -> anyhow::Result<Manifest> {
    let path = install_dir.join("manifest.json");
    let content = std::fs::read_to_string(path)?;
    Ok(serde_json::from_str(&content)?)
}

/// Сохраняет manifest в директорию установки
pub fn save_local(install_dir: &std::path::Path, manifest: &Manifest) -> anyhow::Result<()> {
    let path = install_dir.join("manifest.json");
    std::fs::write(path, serde_json::to_string_pretty(manifest)?)?;
    Ok(())
}
