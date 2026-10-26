#!/usr/bin/env python3
"""Install pinned official runtime distributions; never compile or connect to a tenant."""
import base64
import hashlib
import json
import os
import platform
from pathlib import Path
import shutil
import tarfile
import tempfile
import urllib.request
import zipfile

PS = "7.6.9"
EXO = "3.10.1"
EXO_HASH = "aPlz8t/GKd96eRD6Y8euTQFwjKtzdraI323x0Y1oo5xFa8CQQexu76vVJ9UW7FxBxnzVR5YBwIT/Kzj2PnWo6w=="


def fetch(url, target, limit):
    with urllib.request.urlopen(url, timeout=60) as response, target.open("xb") as output:
        if not response.geturl().startswith("https://"):
            raise ValueError("non-HTTPS download rejected")
        total = 0
        while block := response.read(1024 * 1024):
            total += len(block)
            if total > limit:
                raise ValueError("download exceeds size limit")
            output.write(block)


def destination(root, name):
    p = Path(name)
    if p.is_absolute() or ".." in p.parts or "\\" in name:
        raise ValueError("unsafe archive member")
    return root / p


def install(root):
    if platform.system() != "Linux" or platform.machine() not in ("x86_64", "AMD64"):
        raise ValueError("this pinned runtime installer supports Linux x86_64 only")
    root.mkdir(parents=True, exist_ok=True, mode=0o700)
    if root.is_symlink() or root.stat().st_uid != os.getuid() or root.stat().st_mode & 0o022:
        raise ValueError("unsafe runtime directory")
    targets = [root / f"powershell-{PS}", root / f"exchange-online-management-{EXO}"]
    if any(p.exists() for p in targets):
        raise ValueError("runtime already exists; inspect existing receipt instead of overwriting")
    with tempfile.TemporaryDirectory(prefix=".official-", dir=root) as temporary:
        tmp = Path(temporary)
        base = f"https://github.com/PowerShell/PowerShell/releases/download/v{PS}"
        filename = f"powershell-{PS}-linux-x64.tar.gz"
        archive, hashes = tmp / filename, tmp / "hashes.sha256"
        fetch(base + "/hashes.sha256", hashes, 65536)
        fetch(base + "/" + filename, archive, 100 * 1024 * 1024)
        entries = [line.split() for line in (hashes.read_bytes().decode("utf-16") if hashes.read_bytes().startswith(b"\xff\xfe") else hashes.read_text()).splitlines() if line.strip()]
        expected = [h for h, n in entries if n.lstrip("*") == filename]
        actual = hashlib.sha256(archive.read_bytes()).hexdigest()
        if expected != [actual]:
            raise ValueError("official PowerShell checksum mismatch")
        ps = tmp / "powershell"
        ps.mkdir(mode=0o700)
        with tarfile.open(archive) as t:
            members = t.getmembers()
            if len(members) > 10000 or sum(m.size for m in members) > 600 * 1024 * 1024:
                raise ValueError("archive resource limit")
            # Python's data filter confines links and rejects device entries.
            for m in members:
                destination(ps, m.name)
            t.extractall(ps, filter="data")
        module = tmp / "module.zip"
        url = f"https://www.powershellgallery.com/api/v2/package/ExchangeOnlineManagement/{EXO}"
        fetch(url, module, 100 * 1024 * 1024)
        if hashlib.sha512(module.read_bytes()).digest() != base64.b64decode(EXO_HASH):
            raise ValueError("official Exchange package checksum mismatch")
        exo = tmp / "exchange"
        exo.mkdir(mode=0o700)
        with zipfile.ZipFile(module) as z:
            files = z.infolist()
            if len(files) > 10000 or sum(f.file_size for f in files) > 600 * 1024 * 1024:
                raise ValueError("archive resource limit")
            for f in files:
                target = destination(exo, f.filename)
                if f.is_dir():
                    target.mkdir(parents=True, exist_ok=True, mode=0o700)
                else:
                    target.parent.mkdir(parents=True, exist_ok=True, mode=0o700)
                    with z.open(f) as source, target.open("xb") as out:
                        shutil.copyfileobj(source, out, 1024 * 1024)
                    target.chmod(0o600)
        (ps / "pwsh").chmod(0o700)
        if not (exo / "ExchangeOnlineManagement.psd1").is_file():
            raise ValueError("module manifest absent")
        receipt = {"powershell": {"version": PS, "url": base + "/" + filename, "sha256": actual},
                   "exchange": {"version": EXO, "url": url, "sha512_base64": EXO_HASH},
                   "source": "official binary distributions", "tenant_requests": False}
        (ps / "zix-install-receipt.json").write_text(json.dumps(receipt, indent=2) + "\n")
        ps.rename(targets[0])
        exo.rename(targets[1])
        print(json.dumps({"pwsh": str(targets[0] / "pwsh"), "module": str(targets[1] / "ExchangeOnlineManagement.psd1"), "receipt": receipt}))


if __name__ == "__main__":
    install(Path.home() / ".local/share/zix/runtimes")
