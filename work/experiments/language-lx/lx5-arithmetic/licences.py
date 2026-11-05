"""Capture digests of installed licence notices, without network or installation."""
from pathlib import Path
import shutil
import sys

sys.dont_write_bytecode = True
from common import ROOT, cleanup, json_bytes, run, sha, versions, write


def capture():
    tools = versions()
    sdk_line = run(["dotnet", "--list-sdks"]).stdout.decode().strip().splitlines()[-1]
    dotnet_root = Path(sdk_line.split(" [", 1)[1].rstrip("]")).parent
    rust_root = Path(run(["rustc", "--print", "sysroot"]).stdout.decode().strip())
    node_root = Path(shutil.which("node")).parent
    docs = [("dotnet distribution LICENSE.txt", dotnet_root / "LICENSE.txt"),
            ("dotnet distribution ThirdPartyNotices.txt", dotnet_root / "ThirdPartyNotices.txt"),
            ("Rust standard library COPYRIGHT-library.html", rust_root / "share/doc/rust/COPYRIGHT-library.html"),
            ("Rust compiler COPYRIGHT.html", rust_root / "share/doc/rust/COPYRIGHT.html")]
    for name in ("LICENSE", "LICENSE.txt", "LICENSE.md"):
        if (node_root / name).is_file():
            docs.append(("Node distribution " + name, node_root / name))
    evidence = []
    for label, path in docs:
        if path.is_file():
            content = path.read_bytes()
            evidence.append(dict(document=label, sha256=sha(content), bytes=len(content), status="inspected locally"))
        else:
            evidence.append(dict(document=label, status="not present in inspected installation"))
    return {"format": "lx5-licence-evidence/1", "tools": tools, "documents": evidence,
            "node_core_notice": "no standalone core licence in inspected Node installation root" if not any(d[0].startswith("Node") for d in docs) else "document listed",
            "unfetched_crates": ["num-bigint", "dashu", "ibig", "malachite", "rug"],
            "decision": "open", "decider": "owner"}


if __name__ == "__main__":
    try:
        write(ROOT / "LICENCE-EVIDENCE.json", json_bytes(capture()))
        print("Local licence document digests recorded; no dependency selected")
    finally:
        cleanup()
