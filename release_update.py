"""
Обновление файлов сборки в GitHub Release.

Использование:
  python release_update.py                        # обновить только manifest
  python release_update.py simulation-0.1.0.jar  # загрузить мод + обновить manifest
  python release_update.py mod1.jar mod2.jar      # несколько файлов

Требует: gh (GitHub CLI) в PATH и авторизацию `gh auth login`.
"""

import argparse
import hashlib
import json
import os
import subprocess
import sys

# ── Конфиг ────────────────────────────────────────────────────────────────────
SCRIPT_DIR    = os.path.dirname(os.path.abspath(__file__))
RELEASE_TAG   = "PreAlpha"
MODS_DIR      = os.path.join(SCRIPT_DIR, "..", "clientALLMODS", "mods")
MANIFEST_PATH = os.path.join(SCRIPT_DIR, "..", "clientALLMODS", "manifest.json")
# Папка с артефактами нашего мода (gradle build output)
SIM_MOD_BUILD = os.path.join(SCRIPT_DIR, "..", "..", "SimulationMod", "build", "libs")
REPO          = "kaish1ro/SimulationModPack"
CONFIG_REPO   = "https://github.com/kaish1ro/SimulationModPack"
CONFIG_BRANCH = "main"
FORGE_VERSION = "1.20.1-47.4.10"
PACK_VERSION  = "0.1.0"
JAVA_MIN      = 17
# ─────────────────────────────────────────────────────────────────────────────


def sha256(path: str) -> str:
    h = hashlib.sha256()
    with open(path, "rb") as f:
        for chunk in iter(lambda: f.read(65536), b""):
            h.update(chunk)
    return h.hexdigest()


def run(cmd: list, check=True) -> subprocess.CompletedProcess:
    print(f"  $ {' '.join(cmd)}")
    return subprocess.run(cmd, check=check, capture_output=True, text=True)


def upload_files(files: list[str]) -> list[str]:
    """Загружает файлы в GitHub Release. Возвращает список имён загруженных файлов."""
    uploaded = []
    for path in files:
        name = os.path.basename(path)
        if not os.path.exists(path):
            # Ищем в MODS_DIR
            candidate = os.path.join(MODS_DIR, name)
            if os.path.exists(candidate):
                path = candidate
            else:
                print(f"  [!] Файл не найден: {path}")
                continue

        size_mb = os.path.getsize(path) / 1024 / 1024
        print(f"  Загружаю {name} ({size_mb:.1f} МБ)...")
        result = run([
            "gh", "release", "upload", RELEASE_TAG, path,
            "--repo", REPO, "--clobber"
        ], check=False)

        if result.returncode == 0:
            print(f"  ✓ {name} загружен")
            uploaded.append(name)
        else:
            print(f"  ✗ Ошибка загрузки {name}:\n{result.stderr.strip()}")

    return uploaded


def regenerate_manifest() -> dict:
    """Пересчитывает manifest.json по всем JAR в MODS_DIR."""
    base_url = f"https://github.com/{REPO}/releases/download/{RELEASE_TAG}"

    jar_files = sorted(f for f in os.listdir(MODS_DIR) if f.endswith(".jar"))
    print(f"  Сканирую {len(jar_files)} JAR-файлов...")

    mods = []
    for name in jar_files:
        path = os.path.join(MODS_DIR, name)
        mods.append({
            "name":   name,
            "url":    f"{base_url}/{name.replace(' ', '.')}",
            "sha256": sha256(path),
            "size":   os.path.getsize(path),
        })

    manifest = {
        "version":       PACK_VERSION,
        "forge":         FORGE_VERSION,
        "java_min":      JAVA_MIN,
        "config_repo":   CONFIG_REPO,
        "config_branch": CONFIG_BRANCH,
        "mods":          mods,
    }

    new_content = json.dumps(manifest, indent=2, ensure_ascii=False)

    # Проверяем изменился ли файл
    changed = True
    if os.path.exists(MANIFEST_PATH):
        with open(MANIFEST_PATH, "r", encoding="utf-8") as f:
            changed = f.read() != new_content

    with open(MANIFEST_PATH, "w", encoding="utf-8") as f:
        f.write(new_content)

    status = "изменён" if changed else "не изменился (те же хеши)"
    print(f"  ✓ manifest.json {status} ({len(mods)} модов)")
    return manifest, changed


def git(manifest_dir: str, *args) -> int:
    """Запускает git команду напрямую (без перехвата вывода)."""
    cmd = ["git", "-C", manifest_dir] + list(args)
    print(f"  $ {' '.join(cmd)}")
    return subprocess.call(cmd)

def push_manifest():
    """Коммитит и пушит manifest.json."""
    d = os.path.dirname(os.path.abspath(MANIFEST_PATH))
    name = os.path.basename(MANIFEST_PATH)

    print(f"  Путь: {os.path.abspath(MANIFEST_PATH)}")

    # pull --rebase чтобы синхронизироваться с remote
    git(d, "pull", "--rebase", "origin", "main")

    # add + commit + push
    git(d, "add", name)

    code = git(d, "commit", "-m", f"Update manifest ({RELEASE_TAG})")
    if code != 0:
        print("  ✓ Нечего коммитить")
        return

    code = git(d, "push", "origin", "main")
    if code != 0:
        print("  [!] push не удался — запусти git push вручную")
        return

    print("  ✓ manifest.json запушен в GitHub")


def main():
    global RELEASE_TAG

    parser = argparse.ArgumentParser(description="Обновить релиз сборки")
    parser.add_argument("files", nargs="*",
                        help="JAR-файлы для загрузки (без аргументов — только manifest)")
    parser.add_argument("--no-push", action="store_true",
                        help="Не пушить manifest в GitHub")
    parser.add_argument("--tag", default=RELEASE_TAG,
                        help=f"Тег релиза (дефолт: {RELEASE_TAG})")
    args = parser.parse_args()

    RELEASE_TAG = args.tag

    print(f"\n{'='*50}")
    print(f"  SimulationModPack release update")
    print(f"  Релиз: {RELEASE_TAG}  |  Репозиторий: {REPO}")
    print(f"{'='*50}\n")

    # 1. Загружаем файлы если указаны
    if args.files:
        print(f"[1/3] Загрузка файлов в GitHub Release...")
        uploaded = upload_files(args.files)
        if not uploaded:
            print("  Нет загруженных файлов, прерываю.")
            sys.exit(1)
    else:
        print("[1/3] Файлы для загрузки не указаны — пропускаю")

    # 2. Пересчитываем manifest
    print(f"\n[2/3] Генерация manifest.json...")
    _, manifest_changed = regenerate_manifest()

    # 3. Пушим
    if not args.no_push:
        if manifest_changed or args.files:
            print(f"\n[3/3] Публикация manifest.json...")
            push_manifest()
        else:
            print(f"\n[3/3] manifest.json не изменился — пуш не нужен")
    else:
        print("\n[3/3] --no-push: пуш пропущен")

    print(f"\n✓ Готово! Игроки увидят обновление при следующей проверке.\n")


if __name__ == "__main__":
    main()
