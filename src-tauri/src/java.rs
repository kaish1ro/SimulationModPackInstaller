use std::path::PathBuf;
use std::process::Command;

#[cfg(target_os = "windows")]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

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

fn check_version(exe: &PathBuf) -> bool {
    match java_version_output(exe) {
        Some(s) => is_java17_or_higher(&s),
        None => false,
    }
}

pub fn get_java_major(exe: &PathBuf) -> Option<u32> {
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

fn is_java17_or_higher(version_str: &str) -> bool {
    for line in version_str.lines() {
        if line.contains("version") {
            if let Some(v) = line.split('"').nth(1) {
                let major: u32 = v.split('.').next()
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(0);
                return major >= 17;
            }
        }
    }
    false
}

/// Читает PATH из реестра Windows (актуальный, не из текущего процесса).
#[cfg(target_os = "windows")]
fn get_system_path_dirs() -> Vec<PathBuf> {
    use winreg::enums::*;
    use winreg::RegKey;

    let mut dirs = vec![];

    // System PATH
    if let Ok(key) = RegKey::predef(HKEY_LOCAL_MACHINE)
        .open_subkey(r"SYSTEM\CurrentControlSet\Control\Session Manager\Environment") {
        if let Ok(val) = key.get_value::<String, _>("Path") {
            for p in val.split(';') {
                let pb = PathBuf::from(p.trim());
                if pb.is_dir() { dirs.push(pb); }
            }
        }
    }

    // User PATH
    if let Ok(key) = RegKey::predef(HKEY_CURRENT_USER)
        .open_subkey(r"Environment") {
        if let Ok(val) = key.get_value::<String, _>("Path") {
            for p in val.split(';') {
                let pb = PathBuf::from(p.trim());
                if pb.is_dir() { dirs.push(pb); }
            }
        }
    }

    dirs
}

/// Ищет Java 17+ из реестра Windows (JavaSoft, Adoptium, Corretto и т.д.).
#[cfg(target_os = "windows")]
fn find_java_in_registry() -> Option<PathBuf> {
    use winreg::enums::*;
    use winreg::RegKey;

    let registry_paths = [
        r"SOFTWARE\JavaSoft\JDK",
        r"SOFTWARE\JavaSoft\Java Development Kit",
        r"SOFTWARE\Eclipse Adoptium\JDK",
        r"SOFTWARE\Eclipse Foundation\JDK",
        r"SOFTWARE\Microsoft\JDK",
        r"SOFTWARE\Amazon Corretto\JDK",
        r"SOFTWARE\Azul Systems\Zulu",
        r"SOFTWARE\BellSoft\Liberica",
    ];

    for path in &registry_paths {
        for hive in &[HKEY_LOCAL_MACHINE, HKEY_CURRENT_USER] {
            if let Ok(key) = RegKey::predef(*hive).open_subkey(path) {
                for version_name in key.enum_keys().flatten() {
                    if let Ok(ver_key) = key.open_subkey(&version_name) {
                        if let Ok(home) = ver_key.get_value::<String, _>("JavaHome") {
                            let exe = PathBuf::from(&home).join("bin").join("java.exe");
                            if exe.exists() && check_version(&exe) {
                                return Some(exe);
                            }
                        }
                    }
                }
            }
        }
    }
    None
}

/// Ищет Java 17+ — включая свежеустановленные через реестр и PATH.
pub fn find_java() -> Option<PathBuf> {
    // 1. JAVA_HOME
    if let Ok(java_home) = std::env::var("JAVA_HOME") {
        let exe = PathBuf::from(&java_home).join("bin").join("java.exe");
        if exe.exists() && check_version(&exe) {
            return Some(exe);
        }
    }

    // 2. Minecraft bundled JRE
    let appdata = dirs::data_dir().unwrap_or_default();
    for runtime in ["java-runtime-gamma", "java-runtime-delta", "java-runtime-beta"] {
        let exe = appdata
            .join(".minecraft").join("runtime").join(runtime)
            .join("windows").join(runtime).join("bin").join("java.exe");
        if exe.exists() && check_version(&exe) {
            return Some(exe);
        }
    }

    // 3. Реестр Windows — находит Java установленную ПОСЛЕ старта процесса
    #[cfg(target_os = "windows")]
    if let Some(exe) = find_java_in_registry() {
        return Some(exe);
    }

    // 4. Стандартные пути (статические)
    let static_dirs = [
        r"C:\Program Files\Eclipse Adoptium",
        r"C:\Program Files\Microsoft",
        r"C:\Program Files\Java",
        r"C:\Program Files\OpenJDK",
        r"C:\Program Files\Zulu",
        r"C:\Program Files\Amazon Corretto",
        r"C:\Program Files\BellSoft",
    ];
    for base in &static_dirs {
        if let Ok(entries) = std::fs::read_dir(base) {
            for entry in entries.flatten() {
                let exe = entry.path().join("bin").join("java.exe");
                if exe.exists() && check_version(&exe) {
                    return Some(exe);
                }
            }
        }
    }

    // 5. Свежий PATH из реестра (актуальнее текущего процесса)
    #[cfg(target_os = "windows")]
    for dir in get_system_path_dirs() {
        let exe = dir.join("java.exe");
        if exe.exists() && check_version(&exe) {
            return Some(exe);
        }
    }

    // 6. PATH текущего процесса
    if check_version(&PathBuf::from("java")) {
        return Some(PathBuf::from("java"));
    }

    None
}

/// Ищет предпочтительную версию Java (например Java 17 для Forge).
pub fn find_preferred_java(preferred_major: u32) -> Option<PathBuf> {
    let candidates_dirs = [
        r"C:\Program Files\Eclipse Adoptium",
        r"C:\Program Files\Microsoft",
        r"C:\Program Files\Java",
        r"C:\Program Files\OpenJDK",
        r"C:\Program Files\Zulu",
        r"C:\Program Files\Amazon Corretto",
        r"C:\Program Files\BellSoft",
    ];

    let mut best: Option<(u32, PathBuf)> = None;

    for base in &candidates_dirs {
        if let Ok(entries) = std::fs::read_dir(base) {
            for entry in entries.flatten() {
                let exe = entry.path().join("bin").join("java.exe");
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
        }
    }

    // Также проверяем через реестр
    #[cfg(target_os = "windows")]
    if let Some(exe) = find_java_in_registry() {
        if let Some(v) = get_java_major(&exe) {
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

    // Minecraft bundled
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
