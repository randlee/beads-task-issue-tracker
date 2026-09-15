#!/usr/bin/env python3
"""Version / toolchain single-source-of-truth check (and bump helper).

Sources of truth:
  * App version  -> Cargo.toml (repo root)  [workspace.package].version
  * Rust toolchain -> rust-toolchain.toml  [toolchain].channel

Derived / mirrored values this script verifies:
  * Cargo.toml             every workspace package uses `version.workspace = true`,
                           except INDEPENDENT_VERSION_MEMBERS, which carry one identical
                           explicit version equal to the `=` pin of
                           [workspace.dependencies].sc-observability-log-macros
  * crates/btit-app/tauri.conf.json  has no `version` (Tauri falls back to Cargo.toml)
  * package.json           `version` equals the workspace version (Nuxt reads it)
  * Cargo.lock             workspace package entries carry the workspace version
                           (INDEPENDENT_VERSION_MEMBERS excluded)
  * Cargo.toml             [workspace.package].rust-version equals the toolchain channel
  * .github/workflows/*.yml  every `toolchain:` pin equals the toolchain channel

Usage:
  python3 scripts/check_version_sync.py              # check, exit 1 on mismatch
  python3 scripts/check_version_sync.py --set 1.2.3  # bump every copy, then check

Requires Python 3.11+ (tomllib).
"""
from __future__ import annotations

import argparse
import json
import re
import sys
import tomllib
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
CARGO_TOML = ROOT / "Cargo.toml"
CARGO_LOCK = ROOT / "Cargo.lock"
TAURI_CONF = ROOT / "crates" / "btit-app" / "tauri.conf.json"
PACKAGE_JSON = ROOT / "package.json"
TOOLCHAIN_TOML = ROOT / "rust-toolchain.toml"
WORKFLOWS = ROOT / ".github" / "workflows"
# Workspace members that keep their own version (in-tree phase-a crates; not bumped with the app).
INDEPENDENT_VERSION_MEMBERS = {"sc-observability-log", "sc-observability-log-macros", "sc-observability-log-consumer-check"}
# The `=X.Y.Z` pin the independent members' shared version must equal.
INDEPENDENT_VERSION_PIN_DEPENDENCY = "sc-observability-log-macros"

SEMVER = re.compile(r"^\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?(?:\+[0-9A-Za-z.-]+)?$")


def rel(path: Path) -> str:
    return path.relative_to(ROOT).as_posix()


def load_toml(path: Path) -> dict:
    return tomllib.loads(path.read_text(encoding="utf-8"))


def workspace_package_manifests(cargo: dict) -> list[Path]:
    """Cargo.toml files of all workspace packages (root package + explicit members)."""
    manifests: list[Path] = []
    if "package" in cargo:
        manifests.append(CARGO_TOML)
    for member in cargo.get("workspace", {}).get("members", []):
        for member_dir in sorted(CARGO_TOML.parent.glob(member)):
            if (member_dir / "Cargo.toml").is_file():
                manifests.append(member_dir / "Cargo.toml")
    return manifests


def check() -> list[str]:
    errors: list[str] = []
    cargo = load_toml(CARGO_TOML)
    ws_pkg = cargo.get("workspace", {}).get("package", {})
    version = ws_pkg.get("version")
    if not isinstance(version, str) or not SEMVER.match(version):
        return [f"{rel(CARGO_TOML)}: [workspace.package].version missing or not semver: {version!r}"]

    packages: list[str] = []
    independent: dict[str, str | None] = {}
    for manifest_path in workspace_package_manifests(cargo):
        pkg = load_toml(manifest_path).get("package", {})
        name = pkg.get("name", "?")
        if name in INDEPENDENT_VERSION_MEMBERS:
            member_version = pkg.get("version")
            if isinstance(member_version, str) and SEMVER.match(member_version):
                independent[name] = member_version
            else:
                independent[name] = None
                errors.append(
                    f"{rel(manifest_path)}: [package].version must be an explicit semver string "
                    f"(independently versioned member), got {member_version!r}"
                )
            continue
        packages.append(name)
        if pkg.get("version") != {"workspace": True}:
            errors.append(f"{rel(manifest_path)}: [package] must use `version.workspace = true`")

    missing = sorted(INDEPENDENT_VERSION_MEMBERS - independent.keys())
    if missing:
        errors.append(f"{rel(CARGO_TOML)}: independent members are not workspace members: {', '.join(missing)}")
    pin_dep = cargo.get("workspace", {}).get("dependencies", {}).get(INDEPENDENT_VERSION_PIN_DEPENDENCY, {})
    pin = pin_dep.get("version") if isinstance(pin_dep, dict) else None
    independent_versions = set(independent.values())
    independent_version = next(iter(independent_versions)) if len(independent_versions) == 1 else None
    if independent_version is None or f"={independent_version}" != pin:
        errors.append(
            f"{rel(CARGO_TOML)}: independent members must share one explicit version equal to the {pin!r} pin "
            f"of [workspace.dependencies].{INDEPENDENT_VERSION_PIN_DEPENDENCY}, got {independent!r}"
        )

    tauri_conf = json.loads(TAURI_CONF.read_text(encoding="utf-8"))
    if "version" in tauri_conf:
        errors.append(
            f"{rel(TAURI_CONF)}: remove `version`; Tauri uses the Cargo.toml version when it is absent"
        )

    pkg_json_version = json.loads(PACKAGE_JSON.read_text(encoding="utf-8")).get("version")
    if pkg_json_version != version:
        errors.append(f"{rel(PACKAGE_JSON)}: version {pkg_json_version!r} != workspace version {version!r}")

    lock_versions = {
        p.get("name"): p.get("version")
        for p in load_toml(CARGO_LOCK).get("package", [])
        if "source" not in p  # local (path/workspace) packages only
    }
    for name in packages:
        if lock_versions.get(name) != version:
            errors.append(
                f"{rel(CARGO_LOCK)}: {name} is {lock_versions.get(name)!r}, expected {version!r} "
                "(run a cargo command to refresh the lockfile)"
            )

    channel = load_toml(TOOLCHAIN_TOML).get("toolchain", {}).get("channel")
    if not isinstance(channel, str) or not SEMVER.match(channel):
        errors.append(f"{rel(TOOLCHAIN_TOML)}: [toolchain].channel must be an exact version, got {channel!r}")
    else:
        if ws_pkg.get("rust-version") != channel:
            errors.append(
                f"{rel(CARGO_TOML)}: [workspace.package].rust-version {ws_pkg.get('rust-version')!r} "
                f"!= toolchain channel {channel!r}"
            )
        for wf in sorted(WORKFLOWS.glob("*.y*ml")):
            text = wf.read_text(encoding="utf-8")
            if re.search(r"dtolnay/rust-toolchain@stable", text):
                errors.append(f"{rel(wf)}: use dtolnay/rust-toolchain@master with an explicit `toolchain:` pin")
            for pinned in re.findall(r"^\s*toolchain:\s*['\"]?([^'\"\s]+)['\"]?\s*$", text, re.MULTILINE):
                if pinned != channel:
                    errors.append(f"{rel(wf)}: toolchain pin {pinned!r} != {rel(TOOLCHAIN_TOML)} channel {channel!r}")

    if not errors:
        print(
            f"version sync OK: app {version}, rust toolchain {channel} "
            f"({', '.join(packages)}; independent: {', '.join(independent)} @ {independent_version})"
        )
    return errors


def sub_once(pattern: str, repl: str, text: str, path: Path, flags: int = 0) -> str:
    new_text, count = re.subn(pattern, repl, text, count=1, flags=flags)
    if count != 1:
        raise SystemExit(f"{rel(path)}: could not locate version field to update")
    return new_text


def set_version(new_version: str) -> None:
    if not SEMVER.match(new_version):
        raise SystemExit(f"not a semver version: {new_version!r}")
    cargo = load_toml(CARGO_TOML)
    packages = [
        name
        for name in (load_toml(m)["package"]["name"] for m in workspace_package_manifests(cargo))
        if name not in INDEPENDENT_VERSION_MEMBERS
    ]

    text = CARGO_TOML.read_text(encoding="utf-8")
    text = sub_once(
        r'(^\[workspace\.package\][^\[]*?^version\s*=\s*")[^"]*(")',
        rf"\g<1>{new_version}\g<2>",
        text,
        CARGO_TOML,
        re.MULTILINE | re.DOTALL,
    )
    CARGO_TOML.write_text(text, encoding="utf-8")

    text = PACKAGE_JSON.read_text(encoding="utf-8")
    PACKAGE_JSON.write_text(
        sub_once(r'("version"\s*:\s*")[^"]*(")', rf"\g<1>{new_version}\g<2>", text, PACKAGE_JSON),
        encoding="utf-8",
    )

    text = CARGO_LOCK.read_text(encoding="utf-8")
    for name in packages:
        text = sub_once(
            rf'(\[\[package\]\]\nname = "{re.escape(name)}"\nversion = ")[^"]*(")',
            rf"\g<1>{new_version}\g<2>",
            text,
            CARGO_LOCK,
        )
    CARGO_LOCK.write_text(text, encoding="utf-8")
    print(f"set version {new_version} in {rel(CARGO_TOML)}, {rel(PACKAGE_JSON)}, {rel(CARGO_LOCK)}")


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--set", metavar="X.Y.Z", help="bump the app version everywhere, then check")
    args = parser.parse_args()
    if args.set:
        set_version(args.set)
    errors = check()
    for error in errors:
        print(f"error: {error}", file=sys.stderr)
    return 1 if errors else 0


if __name__ == "__main__":
    sys.exit(main())
