"""TEST-1 (uzak makine): host bu bilgisayarda, izleyici GitHub Actions makinesinde.

    python uzak.py <derle_run_id> [--test 2]     # --test 2: izleyicide doğrudan UDP kapalı

Kod+parola ve sonuç şifreleme anahtarı geçici repo secret'ı olarak yazılır, iş bitince
silinir. Sonuç (ekran görüntüsü içerir) şifreli iner, burada çözülür.
"""
import json
import os
import secrets
import shutil
import subprocess
import sys
import tarfile
import time
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent))
from calistir import KOK, PENCERESIZ, VARSAYILAN_EXE, degerlendir  # noqa: E402

REPO = "pirncedark/afudesk"
DAL = "ozellik/wan-p0"


def gh(*a, girdi=None, kontrol=True):
    r = subprocess.run(["gh", *a], input=girdi, capture_output=True, text=True, creationflags=PENCERESIZ)
    if kontrol and r.returncode != 0:
        raise SystemExit(f"gh {' '.join(a[:3])} başarısız: {r.stderr.strip()}")
    return r.stdout.strip()


def main() -> None:
    derle_run = sys.argv[1]
    test = 2 if sys.argv[2:4] == ["--test", "2"] else 1
    zaman = time.strftime("%Y%m%d_%H%M%S")
    klasor = KOK / "reports" / "wan" / f"TEST{test}_uzak_{zaman}"
    klasor.mkdir(parents=True, exist_ok=True)
    (klasor / "bilgi.json").write_text(json.dumps({"test": test, "uzak": True, "derle_run": derle_run}), encoding="utf-8")
    env = {**os.environ, "AFUDESK_TEST_GENEL_YOL": "1"}
    for k in ("AFUDESK_SADECE_RELAY", "AFUDESK_BAGLA_IP", "AFUDESK_PROXY"):
        env.pop(k, None)
    anahtar = secrets.token_urlsafe(32)
    with open(klasor / "host.log", "w", encoding="utf-8") as hl:
        host = subprocess.Popen([str(VARSAYILAN_EXE), "host", str(klasor)], env=env, stdout=hl,
                                stderr=subprocess.STDOUT, creationflags=PENCERESIZ)
        try:
            kod_yolu = klasor / "kod.txt"
            son = time.time() + 60
            while not kod_yolu.exists():
                if time.time() > son:
                    raise SystemExit("host kod üretmedi")
                time.sleep(0.3)
            gh("secret", "set", "WAN_KOD", "--repo", REPO, girdi=kod_yolu.read_text(encoding="utf-8").strip())
            gh("secret", "set", "WAN_ANAHTAR", "--repo", REPO, girdi=anahtar)
            once = {r["databaseId"] for r in json.loads(gh("run", "list", "--repo", REPO, "--workflow", "ci.yml",
                                                             "--limit", "20", "--json", "databaseId"))}
            gh("workflow", "run", "ci.yml", "--repo", REPO, "--ref", DAL, "-f", "wan=baglan", "-f", f"derleme_run={derle_run}",
               "-f", f"sadece_relay={'1' if test == 2 else ''}")
            run_id = None
            for _ in range(60):
                time.sleep(3)
                yeni = [r["databaseId"] for r in json.loads(gh("run", "list", "--repo", REPO, "--workflow", "ci.yml",
                                                                "--limit", "20", "--json", "databaseId"))
                        if r["databaseId"] not in once]
                if yeni:
                    run_id = str(max(yeni))
                    break
            if not run_id:
                raise SystemExit("bağlanma çalışması başlamadı")
            print(f"bağlanma çalışması: https://github.com/{REPO}/actions/runs/{run_id}", flush=True)
            (klasor / "bilgi.json").write_text(json.dumps({"test": test, "uzak": True, "derle_run": derle_run,
                                                           "baglan_run": run_id}), encoding="utf-8")
            gh("run", "watch", run_id, "--repo", REPO, "--interval", "10", kontrol=False)
            sonuc = json.loads(gh("run", "view", run_id, "--repo", REPO, "--json", "conclusion"))
            print("çalışma sonucu:", sonuc.get("conclusion"), flush=True)
            indir = klasor / "indir"
            gh("run", "download", run_id, "--repo", REPO, "-n", "wan_sonuc", "-D", str(indir))
            subprocess.run(["openssl", "enc", "-d", "-aes-256-cbc", "-pbkdf2", "-pass", f"pass:{anahtar}",
                            "-in", str(indir / "sonuc.enc"), "-out", str(indir / "s.tgz")],
                           check=True, creationflags=PENCERESIZ)
            with tarfile.open(indir / "s.tgz") as t:
                for m in t.getmembers():
                    if m.name.startswith(("/", "..")) or ".." in Path(m.name).parts or not (m.isfile() or m.isdir()):
                        raise SystemExit(f"şüpheli arşiv üyesi: {m.name}")
                t.extractall(indir)
            for f in (indir / "sonuc").iterdir():
                shutil.move(str(f), str(klasor / f.name))
            shutil.rmtree(indir)
        finally:
            (klasor / "dur").write_text("1")
            try:
                host.wait(timeout=15)
            except subprocess.TimeoutExpired:
                host.kill()
            for s in ("WAN_KOD", "WAN_ANAHTAR"):
                gh("secret", "delete", s, "--repo", REPO, kontrol=False)
            (klasor / "kod.txt").unlink(missing_ok=True)
    s = degerlendir(klasor, test)
    (klasor / "sonuc.json").write_text(json.dumps(s, ensure_ascii=False, indent=2), encoding="utf-8")
    md = [f"# TEST-{test} (uzak makine): {s['sonuc']}", "", f"Klasör: `{klasor}`", ""] + [f"- {x}" for x in s["sebepler"] + s["notlar"]]
    (klasor / "sonuc.md").write_text("\n".join(md) + "\n", encoding="utf-8")
    print("\n".join(md))
    sys.exit(0 if s["sonuc"] == "PASS" else 1)


if __name__ == "__main__":
    main()
