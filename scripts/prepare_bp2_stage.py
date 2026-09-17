#!/usr/bin/env python3
"""Verify and reconstruct the B.P2 staged dependency tree from `.crate` bytes.

The bridge may consume only the retained B.P2 archive set.  This script refuses
an arbitrary source checkout, verifies the recorded manifest and archive hashes,
then extracts those bytes into the ignored workspace-local Cargo paths.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import shutil
import tarfile
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
PACKAGES = {"sc-observability", "sc-observability-types"}
MANIFEST_SHA256 = "822ff4494dcfbf92b3dcf47fa3fceac8df00b7e28baf7777b6a5aa1147078077"


def digest(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--stage", type=Path, required=True)
    args = parser.parse_args()
    stage = args.stage.resolve()
    if not (stage / "stage-manifest.json").is_file() and (stage / "bp2-candidate-stage").is_dir():
        stage = stage / "bp2-candidate-stage"
    manifest_path = stage / "stage-manifest.json"
    if digest(manifest_path) != MANIFEST_SHA256:
        raise SystemExit("B.P2 stage manifest checksum did not match the final handoff")
    manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
    if manifest["candidate_version"] != "1.3.0" or manifest["source_commit"] != "561f89923c7f4fdfa9cd0fafa929a5d6dc94dfe5":
        raise SystemExit("unexpected B.P2 candidate identity")
    destination = ROOT / ".bp2-stage"
    if destination.exists():
        shutil.rmtree(destination)
    destination.mkdir()
    extracted = destination / "extracted"
    extracted.mkdir()
    for package in manifest["packages"]:
        if package["name"] not in PACKAGES:
            continue
        archive = (stage / package["archive"]).resolve()
        if stage not in archive.parents or digest(archive) != package["archive_sha256"]:
            raise SystemExit(f"invalid staged archive: {package['name']}")
        with tarfile.open(archive, "r:gz") as bundle:
            roots = {member.name.split("/", 1)[0] for member in bundle.getmembers()}
            if roots != {package["archive_root"]}:
                raise SystemExit(f"unexpected archive root: {package['name']}")
            names = sorted(member.name for member in bundle.getmembers() if member.isfile())
            if names != sorted(package["checked_contents"]):
                raise SystemExit(f"unexpected archive contents: {package['name']}")
            bundle.extractall(extracted, filter="data")
    print(f"verified B.P2 stage: {manifest_path} ({digest(manifest_path)})")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
