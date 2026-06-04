use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use crate::icon_b64::PROFILE_ICON;

const PROFILE_ID: &str = "SimulationModPack";

/// Ищет launcher_profiles.json. Возвращает (path, log) для диагностики.
pub fn find_launcher_profiles() -> Option<PathBuf> {
    let mut candidates = vec![
        // %APPDATA%\.minecraft (самый распространённый путь)
        dirs::data_dir().unwrap_or_default().join(".minecraft"),
    ];

    // Также пробуем явный путь через APPDATA env
    if let Ok(appdata) = std::env::var("APPDATA") {
        candidates.push(PathBuf::from(&appdata).join(".minecraft"));
    }

    // config_dir на Windows = AppData\Roaming тоже
    candidates.push(dirs::config_dir().unwrap_or_default().join(".minecraft"));

    for dir in &candidates {
        let profiles = dir.join("launcher_profiles.json");
        if profiles.exists() {
            return Some(profiles);
        }
    }
    None
}

/// Возвращает путь к launcher_profiles.json для отображения в UI.
pub fn find_launcher_profiles_path() -> String {
    find_launcher_profiles()
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_else(|| {
            // Показываем ожидаемый путь даже если файл не найден
            dirs::data_dir()
                .unwrap_or_default()
                .join(".minecraft")
                .join("launcher_profiles.json")
                .to_string_lossy()
                .into_owned()
        })
}

/// Добавляет или обновляет профиль SimulationModPack.
/// `ram_mb` — выделенная память в МБ.
pub fn upsert_profile(
    profiles_path: &Path,
    game_dir: &Path,
    forge_version: &str,
    ram_mb: u32,
) -> anyhow::Result<()> {
    let content = std::fs::read_to_string(profiles_path)?;
    let mut root: Value = serde_json::from_str(&content)?;

    let profiles = root
        .get_mut("profiles")
        .and_then(|v| v.as_object_mut())
        .ok_or_else(|| anyhow::anyhow!("profiles key not found in launcher_profiles.json"))?;

    let game_dir_str = game_dir.to_str().unwrap_or("").replace('\\', "/");
    let java_args = build_java_args(ram_mb, "");

    profiles.insert(
        PROFILE_ID.to_string(),
        json!({
            "name": "SimulationModPack",
            "type": "custom",
            "gameDir": game_dir_str,
            "lastVersionId": forge_version,
            "javaArgs": java_args,
            "icon": PROFILE_ICON
        }),
    );

    std::fs::write(profiles_path, serde_json::to_string_pretty(&root)?)?;
    Ok(())
}

/// Обновляет только javaArgs у существующего профиля (кнопка "Сохранить" в настройках).
pub fn update_jvm_args(
    profiles_path: &Path,
    ram_mb: u32,
    extra_args: &str,
) -> anyhow::Result<()> {
    let content = std::fs::read_to_string(profiles_path)?;
    let mut root: Value = serde_json::from_str(&content)?;

    let profile = root
        .get_mut("profiles")
        .and_then(|v| v.get_mut(PROFILE_ID))
        .ok_or_else(|| anyhow::anyhow!(
            "Профиль SimulationModPack не найден — сначала установите сборку"
        ))?;

    profile["javaArgs"] = json!(build_java_args(ram_mb, extra_args));

    std::fs::write(profiles_path, serde_json::to_string_pretty(&root)?)?;
    Ok(())
}

/// Формирует строку JVM-аргументов.
fn build_java_args(ram_mb: u32, extra: &str) -> String {
    let ram = ram_mb.clamp(2048, 16384);
    let xmx = format!("-Xmx{}M", ram);
    let xms = format!("-Xms{}M", (ram / 4).max(512));
    let base = format!(
        "{} {} -XX:+UseG1GC -XX:+ParallelRefProcEnabled -XX:MaxGCPauseMillis=200 \
         -XX:G1HeapRegionSize=32M -Dfml.ignoreInvalidMinecraftCertificates=true",
        xms, xmx
    );
    if extra.trim().is_empty() {
        base
    } else {
        format!("{} {}", base, extra.trim())
    }
}
