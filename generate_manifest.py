"""
Генератор manifest.json для SimulationModPack.

Использование:
  python generate_manifest.py --mods-dir ../clientALLMODS/mods --release-tag v0.1.0

Генерирует manifest.json с SHA256-хешами и URL для скачивания с GitHub Releases.
Готовый файл кладёт в корень репозитория сборки.
"""

import argparse
import hashlib
import json
import os

REPO = "kaish1ro/SimulationModPack"
BRANCH = "main"
FORGE_VERSION = "1.20.1-47.4.10"
JAVA_MIN = 17
CONFIG_REPO = f"https://github.com/{REPO}"


def sha256_file(path: str) -> str:
    h = hashlib.sha256()
    with open(path, "rb") as f:
        for chunk in iter(lambda: f.read(65536), b""):
            h.update(chunk)
    return h.hexdigest()


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--mods-dir", required=True, help="Путь к папке mods/")
    parser.add_argument("--release-tag", default="v0.1.0", help="Тег GitHub Release")
    parser.add_argument("--version", default="0.1.0", help="Версия сборки")
    parser.add_argument("--out", default="../clientALLMODS/manifest.json", help="Куда сохранить")
    args = parser.parse_args()

    base_url = f"https://github.com/{REPO}/releases/download/{args.release_tag}"

    mods = []
    jar_files = sorted(f for f in os.listdir(args.mods_dir) if f.endswith(".jar"))

    print(f"Сканирую {len(jar_files)} JAR-файлов...")
    for i, name in enumerate(jar_files, 1):
        path = os.path.join(args.mods_dir, name)
        size = os.path.getsize(path)
        sha = sha256_file(path)
        mods.append({
            "name": name,
            "url": f"{base_url}/{name}",
            "sha256": sha,
            "size": size,
        })
        print(f"  [{i}/{len(jar_files)}] {name} ({size // 1024} KB) sha256:{sha[:8]}...")

    manifest = {
        "version": args.version,
        "forge": FORGE_VERSION,
        "java_min": JAVA_MIN,
        "config_repo": CONFIG_REPO,
        "config_branch": BRANCH,
        "mods": mods,
    }

    with open(args.out, "w", encoding="utf-8") as f:
        json.dump(manifest, f, indent=2, ensure_ascii=False)

    print(f"\nГотово! manifest.json сохранён в {args.out}")
    print(f"Версия: {args.version}, модов: {len(mods)}")
    print(f"\nДалее:")
    print(f"  1. Загрузи manifest.json на GitHub (git add + commit + push)")
    print(f"  2. Убедись что все JAR-файлы загружены в Release {args.release_tag}")


if __name__ == "__main__":
    main()
