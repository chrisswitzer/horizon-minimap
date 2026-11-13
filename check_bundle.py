"""Validate and optionally package Paper Minimap. Python 3.11+, standard library only."""
import argparse
import hashlib
import json
from pathlib import Path
import tomllib
import zipfile

ROOT = Path(__file__).resolve().parent
BUNDLE = ROOT / "mixins" / "paper_minimap"

def check():
    compat = json.loads((ROOT / "compatibility.json").read_text(encoding="utf-8"))
    cargo = tomllib.loads((ROOT / "Cargo.toml").read_text(encoding="utf-8"))
    manifest = tomllib.loads((BUNDLE / "horizon-mixin.toml").read_text(encoding="utf-8"))
    assert manifest["mod"]["id"] == "paper_minimap"
    assert manifest["mod"]["version"] == cargo["package"]["version"]
    assert manifest["compatibility"]["horizon_revision"] == compat["horizon_revision"]
    for section, key in (("module", "source"), ("injection", "helper")):
        ids = set()
        for item in manifest[section]:
            assert item["id"] not in ids, f"duplicate {section} ID"
            ids.add(item["id"])
            path = (BUNDLE / item[key]).resolve()
            assert path.is_relative_to(BUNDLE.resolve()) and path.is_file(), item[key]
            assert path.read_text(encoding="utf-8").strip(), f"empty source: {path}"
    expected = {"schema":1, "id":manifest["mod"]["id"], "version":manifest["mod"]["version"],
                "game":compat["game"], "horizon":compat["horizon_revision"],
                "mixins":compat["horizon_mixins_revision"], "hooks":compat["hooks"]}
    files = {"horizon-native.json":(json.dumps(expected, indent=2, sort_keys=True)+"\n").encode()}
    for path in sorted(BUNDLE.iterdir()):
        if path.suffix in (".rs", ".toml"):
            files[f"mixins/paper_minimap/{path.name}"] = path.read_bytes()
    files["mixins/paper_minimap/LICENSE"] = (ROOT / "LICENSE").read_bytes()
    return expected, files

def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--package", action="store_true")
    args = parser.parse_args()
    meta, files = check()
    print(f"Validated {len(files)} archive entries; source references and pinned metadata agree.")
    if args.package:
        out = ROOT / "dist" / f"paper_minimap-{meta['version']}.hnmod"
        out.parent.mkdir(exist_ok=True)
        with zipfile.ZipFile(out, "w") as archive:
            for name, data in sorted(files.items()):
                info = zipfile.ZipInfo(name, (2026,1,1,0,0,0))
                info.compress_type = zipfile.ZIP_DEFLATED
                info.external_attr = 0o100644 << 16
                archive.writestr(info, data)
        with zipfile.ZipFile(out) as archive:
            assert archive.testzip() is None
            assert sorted(archive.namelist()) == sorted(files)
            for name, data in files.items():
                assert archive.read(name) == data, name
        print(f"{out.name}: {hashlib.sha256(out.read_bytes()).hexdigest()}")
        print("Source bundle only; full Horizon compilation and in-game tests remain separate.")

if __name__ == "__main__":
    main()
