use std::path::PathBuf;
use std::process::Command;

#[cfg(target_os = "windows")]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

/// Запускает `<exe> -version` без вспышки консоли и возвращает stderr-строку.
fn java_version_output(exe: &PathBuf) -> Option<String> {
    let mut cmd = Command::new(exe);
    cmd.arg("-version");

    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }

    let output = cmd.output().ok()?;
    Some(String::from_utf8_lossy(&output.stderr).into_owned())
}

/// Ищет Java конкретной минимальной версии (предпочитает точное совпадение major).
/// Используется для запуска Forge installer (нужна Java 17, не 22+).
pub fn find_preferred_java(preferred_major: u32) -> Option<PathBuf> {
    let candidates_dirs = [
        r"C:\Program Files\Eclipse Adoptium",
        r"C:\Program Files\Microsoft",
        r"C:\Program Files\Java",
        r"C:\Program Files\OpenJDK",
        r"C:\Program Files\Zulu",
        r"C:\Program Files\Amazon Corretto",
    ];

    let mut best: Option<(u32, PathBuf)> = None;

    for base in &candidates_dirs {
        if let Ok(entries) = std::fs::read_dir(base) {
            for entry in entries.flatten() {
                let exe = entry.path().join("bin").join("java.exe");
                if !exe.exists() { continue; }
                if let Some(v) = get_java_major(&exe) {
                    if v >= 17 {
                        // Предпочитаем preferred_major; иначе — наименьшую >= 17
                        let better = match best {
                            None => true,
                            Some((prev, _)) => {
                                if prev == preferred_major { false }
                                else if v == preferred_major { true }
                                else { v < prev }
                            }
                        };
                        if better { best = Some((v, exe)); }
                    }
                }
            }
        }
    }

    // Также проверяем Minecraft bundled Java
    let appdata = dirs::data_dir().unwrap_or_default();
    for runtime in ["java-runtime-gamma", "java-runtime-delta", "java-runtime-beta"] {
        let exe = appdata
            .join(".minecraft").join("runtime").join(runtime)
            .join("windows").join(runtime).join("bin").join("java.exe");
        if !exe.exists() { continue; }
        if let Some(v) = get_java_major(&exe) {
            if v >= 17 {
                let better = match best {
                    None => true,
                    Some((prev, _)) => {
                        if prev == preferred_major { false }
                        else if v == preferred_major { true }
                        else { v < prev }
                    }
                };
                if better { best = Some((v, exe)); }
            }
        }
    }

    best.map(|(_, p)| p)
}

fn get_java_major(exe: &PathBuf) -> Option<u32> {
    let s = java_version_output(exe)?;
    for line in s.lines() {
        if line.contains("version") {
            if let Some(v) = line.split('"').nth(1) {
                return v.split('.').next()?.parse().ok();
            }
        }
    }
    None
}

/// Ищет Java 17+ и возвращает путь к исполняемому файлу.
pub fn find_java() -> Option<PathBuf> {
    // 1. JAVA_HOME
    if let Ok(java_home) = std::env::var("JAVA_HOME") {
        let exe = PathBuf::from(&java_home).join("bin").join("java.exe");
        if exe.exists() && check_version(&exe) {
            return Some(exe);
        }
    }

    // 2. Minecraft bundled JRE (официальный лаунчер)
    let appdata = dirs::data_dir().unwrap_or_default();
    for runtime in [
        "java-runtime-gamma",
        "java-runtime-delta",
        "java-runtime-beta",
    ] {
        let mc_java = appdata
            .join(".minecraft")
            .join("runtime")
            .join(runtime)
            .join("windows")
            .join(runtime)
            .join("bin")
            .join("java.exe");
        if mc_java.exists() && check_version(&mc_java) {
            return Some(mc_java);
        }
    }

    // 3. Стандартные пути Windows
    let common = [
        r"C:\Program Files\Eclipse Adoptium",
        r"C:\Program Files\Microsoft",
        r"C:\Program Files\Java",
        r"C:\Program Files\OpenJDK",
        r"C:\Program Files\Zulu",
        r"C:\Program Files\Amazon Corretto",
    ];
    for base in &common {
        if let Ok(entries) = std::fs::read_dir(base) {
            for entry in entries.flatten() {
                let exe = entry.path().join("bin").join("java.exe");
                if exe.exists() && check_version(&exe) {
                    return Some(exe);
                }
            }
        }
    }

    // 4. PATH (java в системных переменных)
    let path_java = PathBuf::from("java");
    if check_version(&path_java) {
        return Some(path_java);
    }

    None
}

fn check_version(exe: &PathBuf) -> bool {
    println!("Checking {:?}", exe);
    match java_version_output(exe) {
        Some(s) => is_java17_or_higher(&s),
        None => false,
    }
}

fn is_java17_or_higher(version_str: &str) -> bool {
    for line in version_str.lines() {
        if line.contains("version") {
            if let Some(v) = line.split('"').nth(1) {
                // "17.0.1" / "21" / "1.8.0_xxx"
                let major: u32 = v
                    .split('.')
                    .next()
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(0);
                return major >= 17;
            }
        }
    }
    false
}
