"""HeavenOS status — generated, never hand-edited.

The ICM rule is that status is whatever exists on disk: a stage is COMPLETE when
its `output/` holds files other than `.gitkeep`. This script reads that state
rather than any hand-maintained summary, so it cannot drift.

Run:  python _system/status.py
Also: python _system/status.py --check     (exit 1 if the workspace is inconsistent)
"""

import re
import subprocess
import sys
import pathlib

# Windows consoles often default to a codepage (cp1252) that cannot encode the
# math symbols this report prints (⊗, ξ, ...), crashing mid-print with
# UnicodeEncodeError. UTF-8 can encode all of them, so force it rather than
# requiring callers to set PYTHONIOENCODING themselves.
if sys.stdout.encoding and sys.stdout.encoding.lower() != "utf-8":
    sys.stdout.reconfigure(encoding="utf-8")
    sys.stderr.reconfigure(encoding="utf-8")

ROOT = pathlib.Path(__file__).resolve().parent.parent
STAGES = ["01_derive", "02_design", "03_tests", "04_implement"]


def records():
    return sorted(p for p in (ROOT / "subsystems").iterdir() if p.is_dir())


def stage_marks(rec):
    marks = ""
    for s in STAGES:
        out = rec / s / "output"
        done = out.is_dir() and any(
            p.name != ".gitkeep" for p in out.iterdir() if p.is_file()
        )
        marks += "#" if done else "."
    return marks


def frontmatter(rec, key):
    text = (rec / "CONTEXT.md").read_text(encoding="utf-8", errors="replace")
    m = re.search(rf"^{key}:\s*(.+)$", text, re.M)
    return m.group(1).strip().strip('"') if m else "?"


def test_counts():
    """Per-suite passing counts from cargo, or None if the toolchain is absent."""
    # stderr must be merged into stdout, not concatenated after it: cargo emits
    # "Running <suite>" on stderr and "test result:" on stdout, so appending one
    # to the other loses the interleaving that pairs a suite with its result.
    try:
        r = subprocess.run(
            ["cargo", "test", "--workspace"],
            cwd=ROOT / "neos",
            stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT,
            text=True,
            timeout=600,
        )
    except (FileNotFoundError, subprocess.TimeoutExpired):
        return None
    out = r.stdout
    suites, current = {}, None
    for line in out.splitlines():
        m = re.search(r"Running .*[\\/]([\w]+)\.rs", line) or re.search(
            r"Running .*deps[\\/](\w+?)-[0-9a-f]{16}", line
        )
        if m:
            current = m.group(1)
        m = re.search(r"test result: (ok|FAILED)\. (\d+) passed; (\d+) failed", line)
        if m and current:
            passed, failed = int(m.group(2)), int(m.group(3))
            if passed or failed:
                suites[current] = (passed, failed)
    return suites


def ledger_rows():
    text = (ROOT / "_mkb" / "reconciliation.md").read_text(
        encoding="utf-8", errors="replace"
    )
    rows = re.findall(r"^\| (R\d\w?) \| ([^|]+)\|([^|]+)\|", text, re.M)
    return [(r[0], r[1].strip(), "resolved" if "✅" in r[2] else r[2].strip()) for r in rows]


def dead_wikilinks():
    live = {p.name for p in records()} | {
        p.stem for p in (ROOT / "_mkb").glob("*.md")
    }
    dead = []
    for f in ROOT.rglob("*.md"):
        if "_archive" in f.parts or "target" in f.parts:
            continue
        for i, line in enumerate(
            f.read_text(encoding="utf-8", errors="replace").splitlines(), 1
        ):
            for t in re.findall(r"\[\[([^\]]+)\]\]", line):
                if t not in live and f"`[[{t}]]`" not in line:
                    dead.append(f"{f.relative_to(ROOT)}:{i} [[{t}]]")
    return dead


def broken_links():
    bad = []
    for f in ROOT.rglob("*.md"):
        if "_archive" in f.parts or "target" in f.parts or "papers" in f.parts:
            continue
        for link in re.findall(
            r"\]\(([^)#:]+)\)", f.read_text(encoding="utf-8", errors="replace")
        ):
            if link.startswith(("http", "mailto")):
                continue
            if not (f.parent / link.split("#")[0]).exists():
                bad.append(f"{f.relative_to(ROOT)} -> {link}")
    return bad


def encoding_faults():
    faults = []
    for f in list(ROOT.rglob("*.json")) + list(ROOT.rglob("*.rs")):
        if "_archive" in f.parts or "target" in f.parts:
            continue
        raw = f.read_bytes()
        if raw.startswith(b"\xef\xbb\xbf"):
            faults.append(f"{f.relative_to(ROOT)} has a BOM")
    return faults


def main():
    check_only = "--check" in sys.argv

    print("HeavenOS — build status")
    print("=" * 66)

    recs = records()
    counts = test_counts()

    print("\nRECORDS   (# = stage output exists; the filesystem is the truth)")
    print(f"  {'record':<18} {'derive/design/tests/impl':<26} status")
    for r in recs:
        print(f"  {r.name:<18} [{stage_marks(r)}]{'':<21} {frontmatter(r, 'status')}")

    print("\nTESTS")
    if counts is None:
        print("  cargo unavailable — skipped")
    else:
        total_p = sum(p for p, _ in counts.values())
        total_f = sum(f for _, f in counts.values())
        for name, (p, f) in sorted(counts.items()):
            flag = "" if not f else f"  <-- {f} FAILING"
            print(f"  {name:<26} {p:>3} passing{flag}")
        print(f"  {'TOTAL':<26} {total_p:>3} passing, {total_f} failing")

    rows = ledger_rows()
    unresolved = [r for r in rows if r[2] != "resolved"]
    print(f"\nLAW  ({len(rows)} ledger rows, {len(unresolved)} unresolved)")
    for rid, what, state in rows:
        mark = "ok " if state == "resolved" else "OPEN"
        print(f"  {mark} {rid:<4} {what}")

    dead, broken, enc = dead_wikilinks(), broken_links(), encoding_faults()
    print("\nINTEGRITY")
    print(f"  dead wikilinks   {len(dead)}")
    for d in dead:
        print(f"      {d}")
    print(f"  broken links     {len(broken)}")
    for b in broken:
        print(f"      {b}")
    print(f"  encoding faults  {len(enc)}")
    for e in enc:
        print(f"      {e}")

    nxt = [r.name for r in recs if stage_marks(r) == "...." and frontmatter(r, "status") == "not-started"]
    print(f"\nNEXT  buildable now: {', '.join(nxt) if nxt else 'none'}")

    failing = counts and any(f for _, f in counts.values())
    if check_only and (dead or broken or enc or unresolved or failing):
        print("\nFAIL: workspace is inconsistent")
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
