use crate::{java, manifest, profile};
use anyhow::Context;
use futures_util::StreamExt;
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use tauri::Window;

// ── Tauri-команды ────────────────────────────────────────────────────────────

#[tauri::command]
pub async fn check_java() -> bool {
    println!("check_java called");
    tauri::async_runtime::spawn_blocking(|| java::find_java().is_some())
        .await
        .unwrap_or(false)
}

/// Открывает нативный диалог выбора папки (через Tauri native dialog)
#[tauri::command]
pub async fn pick_directory() -> Option<String> {
    println!("pick_directory called");
    use tauri::api::dialog::blocking::FileDialogBuilder;
    // blocking dialog нельзя звать с главного потока — выносим в blocking-поток
    tauri::async_runtime::spawn_blocking(|| {
        FileDialogBuilder::new()
            .set_title("Выберите папку установки")
            .pick_folder()
            .map(|p| p.to_string_lossy().into_owned())
    })
    .await
    .ok()
    .flatten()
}

/// Запускает Minecraft Launcher (официальный или TLauncher).
/// Если лаунчер не найден — открывает папку установки в проводнике.
#[tauri::command]
pub fn launch_game(install_dir: String) -> Result<String, String> {
    // Известные пути к Minecraft лаунчерам
    let candidates: Vec<std::path::PathBuf> = vec![
        // Microsoft Store / XboxGames (наиболее распространённое место)
        std::path::PathBuf::from(r"C:\XboxGames\Minecraft Launcher\Content\Minecraft.exe"),
        // Официальный лаунчер (AppData)
        dirs::data_dir().unwrap_or_default()
            .join("Minecraft Launcher").join("MinecraftLauncher.exe"),
        // Официальный лаунчер (Program Files)
        std::path::PathBuf::from(r"C:\Program Files (x86)\Minecraft Launcher\MinecraftLauncher.exe"),
        std::path::PathBuf::from(r"C:\Program Files\Minecraft Launcher\MinecraftLauncher.exe"),
        // TLauncher
        dirs::data_dir().unwrap_or_default()
            .join("TLauncher").join("TLauncher.exe"),
        dirs::config_dir().unwrap_or_default()
            .join("TLauncher").join("TLauncher.exe"),
        // Legacy launcher
        dirs::data_dir().unwrap_or_default()
            .join(".minecraft").join("launcher").join("launcher.exe"),
    ];

    for path in &candidates {
        if path.exists() {
            #[cfg(target_os = "windows")]
            {
                use std::os::windows::process::CommandExt;
                std::process::Command::new(path)
                    .creation_flags(0x0000_0008) // DETACHED_PROCESS
                    .spawn()
                    .map_err(|e| e.to_string())?;
            }
            #[cfg(not(target_os = "windows"))]
            {
                std::process::Command::new(path).spawn().map_err(|e| e.to_string())?;
            }
            return Ok(format!("Запущен: {}", path.display()));
        }
    }

    // Лаунчер не найден — открываем папку установки в проводнике
    let dir = std::path::PathBuf::from(&install_dir);
    if dir.exists() {
        #[cfg(target_os = "windows")]
        std::process::Command::new("explorer")
            .arg(&install_dir)
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    Err("Minecraft лаунчер не найден. Открыта папка установки — запусти игру вручную и выбери профиль SimulationModPack.".into())
}

#[tauri::command]
pub fn get_default_install_dir() -> String {
    dirs::data_dir()
        .unwrap_or_default()
        .join("SimulationModPack")
        .to_string_lossy()
        .into_owned()
}

/// Проверяет установлена ли сборка в указанной папке.
/// Возвращает: "ok" | "missing" | "empty"
#[tauri::command]
pub fn check_install_exists(install_dir: String) -> String {
    let dir = std::path::PathBuf::from(&install_dir);
    if !dir.exists() { return "empty".into(); }
    let mods_dir = dir.join("mods");
    if !mods_dir.exists() { return "missing".into(); }
    match std::fs::read_dir(&mods_dir) {
        Ok(mut entries) => if entries.next().is_some() { "ok".into() } else { "missing".into() },
        Err(_) => "missing".into(),
    }
}

/// Возвращает путь к launcher_profiles.json для отображения в UI.
#[tauri::command]
pub fn get_profiles_path() -> String {
    profile::find_launcher_profiles_path()
}

/// Сохраняет RAM и JVM-аргументы в профиль лаунчера.
#[tauri::command]
pub fn save_settings(ram_mb: u32, extra_jvm_args: String) -> Result<String, String> {
    let profiles_path = profile::find_launcher_profiles()
        .ok_or_else(|| "launcher_profiles.json не найден — сначала установите сборку".to_string())?;
    profile::update_jvm_args(&profiles_path, ram_mb, &extra_jvm_args)
        .map(|_| format!("Настройки сохранены → {}", profiles_path.display()))
        .map_err(|e| e.to_string())
}

/// Установка сборки. ram_mb — выделенная память из настроек UI (дефолт 4096 МБ).
#[tauri::command]
pub async fn install(window: Window, install_dir: String, ram_mb: Option<u32>) -> Result<String, String> {
    run_install(window, PathBuf::from(install_dir), ram_mb.unwrap_or(4096))
        .await
        .map_err(|e| e.to_string())
}

/// Проверка обновлений. Возвращает список модов, которые изменились или устарели.
#[tauri::command]
pub async fn check_updates(install_dir: String) -> Result<Vec<String>, String> {
    let dir = PathBuf::from(&install_dir);
    let remote = manifest::fetch_remote().await.map_err(|e| e.to_string())?;

    let manifest_names: std::collections::HashSet<&str> =
        remote.mods.iter().map(|m| m.name.as_str()).collect();

    // Моды, которые нужно скачать/обновить
    let mut changed: Vec<String> = remote
        .mods
        .iter()
        .filter(|m| {
            let path = dir.join("mods").join(&m.name);
            !path.exists() || !verify_hash(&path, &m.sha256)
        })
        .map(|m| m.name.clone())
        .collect();

    // JAR-файлы в папке игрока, которых нет в манифесте — удалённые из сборки
    if let Ok(entries) = std::fs::read_dir(dir.join("mods")) {
        for entry in entries.flatten() {
            let name = entry.file_name();
            let name_str = name.to_string_lossy().into_owned();
            if name_str.ends_with(".jar") && !manifest_names.contains(name_str.as_str()) {
                changed.push(format!("[REMOVED] {}", name_str));
            }
        }
    }

    Ok(changed)
}

/// Применяет обновления (скачивает изменившиеся файлы, удаляет лишние + обновляет конфиги).
#[tauri::command]
pub async fn apply_updates(window: Window, install_dir: String) -> Result<String, String> {
    let dir = PathBuf::from(&install_dir);
    let remote = manifest::fetch_remote().await.map_err(|e| e.to_string())?;

    let manifest_names: std::collections::HashSet<&str> =
        remote.mods.iter().map(|m| m.name.as_str()).collect();

    // Удаляем JAR-файлы, которых нет в манифесте (удалённые из сборки)
    if let Ok(entries) = std::fs::read_dir(dir.join("mods")) {
        for entry in entries.flatten() {
            let name = entry.file_name();
            let name_str = name.to_string_lossy().into_owned();
            if name_str.ends_with(".jar") && !manifest_names.contains(name_str.as_str()) {
                emit_progress(&window, "cleanup", 0.5,
                    &format!("Удаляю устаревший мод: {}", name_str));
                std::fs::remove_file(entry.path()).ok();
            }
        }
    }

    let mods_to_update: Vec<_> = remote
        .mods
        .iter()
        .filter(|m| {
            let path = dir.join("mods").join(&m.name);
            !path.exists() || !verify_hash(&path, &m.sha256)
        })
        .collect();

    let total = mods_to_update.len();
    for (i, entry) in mods_to_update.iter().enumerate() {
        emit_progress(
            &window,
            "mods",
            (i as f32) / (total as f32),
            &format!("Обновление: {}", entry.name),
        );
        let dest = dir.join("mods").join(&entry.name);
        download_file(&entry.url, &dest, &window).await.map_err(|e| e.to_string())?;
    }

    emit_progress(&window, "configs", 0.0, "Обновление конфигов...");
    update_configs(&dir, &remote).await.map_err(|e| e.to_string())?;

    manifest::save_local(&dir, &remote).map_err(|e| e.to_string())?;

    emit_progress(&window, "done", 1.0, "Обновление завершено!");
    Ok("ok".into())
}

// ── Внутренняя логика ────────────────────────────────────────────────────────

async fn run_install(window: Window, install_dir: PathBuf, ram_mb: u32) -> anyhow::Result<String> {
    // 1. Создаём директории
    std::fs::create_dir_all(install_dir.join("mods"))?;
    std::fs::create_dir_all(install_dir.join("config"))?;

    // 2. Скачиваем manifest
    emit_progress(&window, "manifest", 0.0, "Загрузка манифеста...");
    let remote = manifest::fetch_remote().await.context("Не удалось загрузить манифест")?;

    // 3. Проверяем Java
    emit_progress(&window, "java", 0.0, "Поиск Java 17+...");
    let java_exe = java::find_java().context(
        "Java 17+ не найдена. Установи Java 17 с https://adoptium.net/"
    )?;

    // 4. Устанавливаем Forge
    emit_progress(&window, "forge", 0.0, "Установка Forge...");
    install_forge(&java_exe, &install_dir, &remote.forge, &window).await?;

    // 5. Удаляем моды, которых больше нет в сборке
    let manifest_names: std::collections::HashSet<&str> =
        remote.mods.iter().map(|m| m.name.as_str()).collect();
    if let Ok(entries) = std::fs::read_dir(install_dir.join("mods")) {
        for entry in entries.flatten() {
            let name = entry.file_name();
            let name_str = name.to_string_lossy().into_owned();
            if name_str.ends_with(".jar") && !manifest_names.contains(name_str.as_str()) {
                emit_progress(&window, "cleanup", 0.5,
                    &format!("Удаляю устаревший мод: {}", name_str));
                std::fs::remove_file(entry.path()).ok();
            }
        }
    }

    // 6. Скачиваем моды
    let total = remote.mods.len();
    for (i, entry) in remote.mods.iter().enumerate() {
        let dest = install_dir.join("mods").join(&entry.name);
        if dest.exists() && verify_hash(&dest, &entry.sha256) {
            continue; // уже скачан и не изменился
        }
        emit_progress(
            &window,
            "mods",
            (i as f32) / (total as f32),
            &format!("[{}/{}] {}", i + 1, total, entry.name),
        );
        download_file(&entry.url, &dest, &window).await?;
    }
    emit_progress(&window, "mods", 1.0, "Моды загружены");

    // 7. Скачиваем конфиги с GitHub
    emit_progress(&window, "configs", 0.0, "Загрузка конфигов...");
    update_configs(&install_dir, &remote).await?;

    // 8. Сохраняем manifest
    manifest::save_local(&install_dir, &remote)?;

    // 9. Создаём профиль в launcher если возможно
    let forge_version = format!("1.20.1-forge-{}", remote.forge.trim_start_matches("1.20.1-"));
    if let Some(profiles_path) = profile::find_launcher_profiles() {
        if let Err(e) = profile::upsert_profile(&profiles_path, &install_dir, &forge_version, ram_mb) {
            emit_progress(&window, "profile", 1.0, &format!("Профиль: {}", e));
        } else {
            emit_progress(&window, "profile", 1.0, "Профиль добавлен в лаунчер");
        }
    } else {
        emit_progress(
            &window,
            "profile",
            1.0,
            "Minecraft лаунчер не найден — укажи игровую папку вручную",
        );
    }

    emit_progress(&window, "done", 1.0, "Установка завершена!");
    Ok("ok".into())
}

async fn install_forge(
    java_exe: &Path,
    install_dir: &Path,
    forge_version: &str,
    window: &Window,
) -> anyhow::Result<()> {
    let installer_name = format!("forge-{}-installer.jar", forge_version);

    // Ищем JAR в нескольких местах: рядом с exe, рядом с Cargo.toml (dev), в install_dir
    let candidates = vec![
        // Рядом с собранным exe (production)
        std::env::current_exe().ok()
            .and_then(|p| p.parent().map(|d| d.join(&installer_name))),
        // Рядом с Cargo.toml — для dev-режима (src-tauri/../)
        std::env::current_exe().ok()
            .and_then(|p| p.ancestors().nth(4).map(|d| d.join(&installer_name))),
        // В папке назначения
        Some(install_dir.join(&installer_name)),
    ];

    let installer_path = candidates.into_iter()
        .flatten()
        .find(|p| p.exists());

    let installer_path = if let Some(p) = installer_path {
        emit_progress(window, "forge", 0.2, &format!("Forge installer найден: {}", p.display()));
        p
    } else {
        // Скачиваем с официального зеркала
        let url = format!(
            "https://maven.minecraftforge.net/net/minecraftforge/forge/{v}/forge-{v}-installer.jar",
            v = forge_version
        );
        let dest = install_dir.join(&installer_name);
        emit_progress(window, "forge", 0.1, "Скачиваю Forge installer...");
        download_file(&url, &dest, window).await?;
        dest
    };

    // Forge устанавливается в стандартный .minecraft (там уже есть ванильный клиент).
    // Наш install_dir используется только как gameDir в launcher_profiles.json.
    let minecraft_dir = dirs::data_dir()
        .unwrap_or_default()
        .join(".minecraft");
    std::fs::create_dir_all(&minecraft_dir).ok();
    let install_dir_str = minecraft_dir.to_str().unwrap_or_default();

    // Forge требует launcher_profiles.json в .minecraft — создаём минимальный если нет
    let profiles_path = minecraft_dir.join("launcher_profiles.json");
    if !profiles_path.exists() {
        let minimal = serde_json::json!({
            "profiles": {},
            "selectedProfile": null,
            "clientToken": "00000000-0000-0000-0000-000000000000",
            "authenticationDatabase": {},
            "launcherVersion": { "name": "1.0", "format": 21 }
        });
        std::fs::write(&profiles_path, serde_json::to_string_pretty(&minimal)?)?;
        emit_progress(window, "forge", 0.3, "Создан launcher_profiles.json");
    }

    // Forge 1.20.1 требует Java 17 — ищем её, если у пользователя Java 22+
    let forge_java = java::find_preferred_java(17).unwrap_or_else(|| java_exe.to_path_buf());
    emit_progress(window, "forge", 0.5, &format!(
        "Запуск Forge installer через {} ...",
        forge_java.display()
    ));

    #[cfg(target_os = "windows")]
    let output = {
        use std::os::windows::process::CommandExt;
        std::process::Command::new(&forge_java)
            .args([
                "-Djava.awt.headless=true",
                "-jar", installer_path.to_str().unwrap_or_default(),
                "--installClient", install_dir_str,
            ])
            .current_dir(install_dir)
            .creation_flags(0x0800_0000)
            .output()?
    };

    #[cfg(not(target_os = "windows"))]
    let output = std::process::Command::new(&forge_java)
        .args([
            "-Djava.awt.headless=true",
            "-jar", installer_path.to_str().unwrap_or_default(),
            "--installClient", install_dir_str,
        ])
        .current_dir(install_dir)
        .output()?;

    if !output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!(
            "Forge installer error (code {}).\n--- STDOUT ---\n{}\n--- STDERR ---\n{}",
            output.status, stdout, stderr
        );
    }

    emit_progress(window, "forge", 1.0, "Forge установлен");
    Ok(())
}

async fn update_configs(install_dir: &Path, manifest: &manifest::Manifest) -> anyhow::Result<()> {
    // Скачиваем ZIP репозитория с GitHub и распаковываем нужные папки
    let zip_url = format!(
        "{}/archive/refs/heads/{}.zip",
        manifest.config_repo, manifest.config_branch
    );
    let zip_path = install_dir.join("_config_update.zip");

    let client = reqwest::Client::new();
    let bytes = client.get(&zip_url).send().await?.bytes().await?;
    std::fs::write(&zip_path, &bytes)?;

    // Распаковываем только нужные папки (config/, kubejs/, defaultconfigs/, resourcepacks/, patchouli_books/)
    let zip_file = std::fs::File::open(&zip_path)?;
    let mut archive = zip::ZipArchive::new(zip_file)?;

    let target_prefixes = [
        "config/",
        "kubejs/",
        "defaultconfigs/",
        "resourcepacks/",
        "patchouli_books/",
        "visual_keybinder/",
    ];

    for i in 0..archive.len() {
        let mut file = archive.by_index(i)?;
        let name = file.name().to_string();

        // Имя содержит корневую папку репо: "SimulationModPack-main/config/..."
        let relative = if let Some(pos) = name.find('/') {
            &name[pos + 1..]
        } else {
            continue;
        };

        if !target_prefixes.iter().any(|p| relative.starts_with(p)) {
            continue;
        }

        let out_path = install_dir.join(relative);
        if name.ends_with('/') {
            std::fs::create_dir_all(&out_path)?;
        } else {
            if let Some(parent) = out_path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            let mut out_file = std::fs::File::create(&out_path)?;
            std::io::copy(&mut file, &mut out_file)?;
        }
    }

    std::fs::remove_file(&zip_path).ok();
    Ok(())
}

pub async fn download_file(url: &str, dest: &Path, window: &Window) -> anyhow::Result<()> {
    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::limited(10))
        .user_agent("SimulationModPackInstaller/0.1.0")
        .build()?;

    let response = client.get(url).send().await?;

    let status = response.status();
    if !status.is_success() {
        anyhow::bail!(
            "HTTP {} при скачивании {}\n{}",
            status,
            url,
            response.text().await.unwrap_or_default().chars().take(300).collect::<String>()
        );
    }

    let total = response.content_length().unwrap_or(0);
    let mut stream = response.bytes_stream();
    let mut file = std::fs::File::create(dest)?;
    let mut downloaded: u64 = 0;

    use std::io::Write;
    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        file.write_all(&chunk)?;
        downloaded += chunk.len() as u64;
        if total > 0 {
            let _ = window.emit("download_progress", downloaded as f64 / total as f64);
        }
    }

    // Проверяем что скачали не пустышку (LFS pointer = ~130 bytes)
    let file_size = std::fs::metadata(dest)?.len();
    if file_size < 1024 {
        std::fs::remove_file(dest).ok();
        anyhow::bail!(
            "Файл {} слишком мал ({} байт) — возможно LFS pointer или ошибка. URL: {}",
            dest.file_name().unwrap_or_default().to_string_lossy(),
            file_size,
            url
        );
    }

    Ok(())
}

pub fn verify_hash(path: &Path, expected_sha256: &str) -> bool {
    if let Ok(data) = std::fs::read(path) {
        let mut hasher = Sha256::new();
        hasher.update(&data);
        let result = hex::encode(hasher.finalize());
        return result == expected_sha256;
    }
    false
}

fn emit_progress(window: &Window, stage: &str, progress: f32, message: &str) {
    let _ = window.emit(
        "install_progress",
        serde_json::json!({
            "stage": stage,
            "progress": progress,
            "message": message
        }),
    );
}
