#!/usr/bin/env python3
"""Check the pinned runtime downloads against the release they name.

The app installs the speech engine by URL, size and SHA-256 taken from
`apps/or_app/lib/updates/resource_center.dart`. A wrong constant is silent
until a user on that platform clicks Install, so this script reads the same
constants out of the source and verifies each archive against the GitHub
release it points at: HTTP status, byte count and SHA-256.

    python3 scripts/verify_pinned_assets.py            # every host
    python3 scripts/verify_pinned_assets.py --only linux-x64
    python3 scripts/verify_pinned_assets.py --check    # digest only, no download

Exit code 0 means every selected asset matched its recorded size and digest.
"""

from __future__ import annotations

import argparse
import hashlib
import re
import sys
import urllib.request
from dataclasses import dataclass
from pathlib import Path

SOURCE = Path("apps/or_app/lib/updates/resource_center.dart")
USER_AGENT = "Opencut-Reinforced asset check"


@dataclass
class Asset:
    host: str
    url: str
    size: int
    sha256: str


def parse_assets(source: Path) -> list[Asset]:
    text = source.read_text()
    release = re.search(r"const _whisperRelease =\s*'([^']+)'", text)
    if release is None:
        raise SystemExit(f"{source}: cannot find _whisperRelease")
    base = release.group(1)
    assets: list[Asset] = []
    # Every asset is a const SpeechRuntimeAsset(...) block with a file name
    # that identifies its host and architecture.
    for block in re.finditer(r"const SpeechRuntimeAsset\((.*?)\n          \)", text, re.S):
        body = block.group(1)
        url = re.search(r"url: '([^']+)'", body)
        name = re.search(r"fileName: '([^']+)'", body)
        size = re.search(r"sizeBytes: (\d+)", body)
        digest = re.search(r"sha256: '([0-9a-f]{64})'", body)
        if not (url and name and size and digest):
            continue
        assets.append(
            Asset(
                host=name.group(1),
                url=url.group(1).replace("$_whisperRelease", base),
                size=int(size.group(1)),
                sha256=digest.group(1),
            )
        )
    return assets


def check(asset: Asset, timeout: int) -> str:
    request = urllib.request.Request(asset.url, headers={"User-Agent": USER_AGENT})
    digest = hashlib.sha256()
    received = 0
    with urllib.request.urlopen(request, timeout=timeout) as response:
        status = response.status
        while True:
            chunk = response.read(1 << 16)
            if not chunk:
                break
            received += len(chunk)
            digest.update(chunk)
    if status != 200:
        return f"FAIL {asset.host}: HTTP {status}"
    if received != asset.size:
        return f"FAIL {asset.host}: recorded {asset.size} bytes, the release serves {received}"
    if digest.hexdigest() != asset.sha256:
        return f"FAIL {asset.host}: recorded {asset.sha256}, the release hashes to {digest.hexdigest()}"
    return f"OK   {asset.host}: {received} bytes, sha256 {digest.hexdigest()[:16]}…"


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--source", type=Path, default=SOURCE)
    parser.add_argument("--only", help="substring of the archive name to check")
    parser.add_argument("--check", action="store_true", help="parse and report only")
    parser.add_argument("--timeout", type=int, default=120)
    args = parser.parse_args()

    assets = parse_assets(args.source)
    if not assets:
        raise SystemExit(f"{args.source}: no SpeechRuntimeAsset blocks found")
    if args.only:
        assets = [a for a in assets if args.only in a.host]
        if not assets:
            raise SystemExit(f"no asset matches {args.only!r}")

    if args.check:
        for asset in assets:
            print(f"     {asset.host}: {asset.size} bytes, sha256 {asset.sha256[:16]}… ({asset.url})")
        return 0

    failures = 0
    for asset in assets:
        try:
            line = check(asset, args.timeout)
        except Exception as error:  # noqa: BLE001 - the report is the point
            line = f"FAIL {asset.host}: {error}"
        print(line)
        if line.startswith("FAIL"):
            failures += 1
    print(f"PINNED_ASSETS={'PASS' if failures == 0 else 'FAIL'} ({len(assets)} asset(s))")
    return 0 if failures == 0 else 1


if __name__ == "__main__":
    sys.exit(main())
