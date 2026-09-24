"""AfuDesk görev kapısı doğrulayıcısı.

Durumu beyana göre DEĞİL, kanıta göre hesaplar:
  DONE        = bütün kanıt desenleri bulundu + bütün testler en az 1 test koşup geçti
  IN_PROGRESS = kanıtın ya da testin bir kısmı var
  TODO        = hiçbiri yok
  BLOCKED     = maddede "engel" alanı dolu
Çıktı: reports/test-results.json, reports/AUDIT.md. Hepsi DONE değilse çıkış kodu 1 (NOT_READY).

Kullanım: python scripts/kapi_dogrula.py [--kok DIZIN] [--test-yok]
"""
import argparse
import json
import os
import re
import subprocess
import sys
import time
from pathlib import Path

PENCERESIZ = 0x08000000 if os.name == "nt" else 0  # CREATE_NO_WINDOW


def calistir(komut, cwd, sure=1800):
    t0 = time.time()
    try:
        p = subprocess.run(komut, cwd=cwd, capture_output=True, text=True, encoding="utf-8",
                           errors="replace", timeout=sure, creationflags=PENCERESIZ, shell=isinstance(komut, str))
        return p.returncode, p.stdout + p.stderr, round(time.time() - t0, 1)
    except subprocess.TimeoutExpired:
        return -1, "ZAMAN AŞIMI", round(time.time() - t0, 1)


def cargo_test(kok, filtre, onbellek):
    anahtar = ("cargo", filtre)
    if anahtar in onbellek:
        return onbellek[anahtar]
    kod, cikti, sure = calistir(["cargo", "test", "--release", "-q", filtre], kok / "rust")
    gecen = sum(int(m) for m in re.findall(r"test result: \w+\. (\d+) passed", cikti))
    kalan = sum(int(m) for m in re.findall(r"(\d+) failed", cikti))
    sonuc = {"tur": "cargo", "filtre": filtre, "cikis": kod, "gecen": gecen, "basarisiz": kalan, "sure_sn": sure,
             "gecti": kod == 0 and gecen > 0 and kalan == 0, "son_satirlar": cikti.strip().splitlines()[-4:]}
    onbellek[anahtar] = sonuc
    return sonuc


def flutter_test(kok, filtre, onbellek):
    anahtar = ("flutter", filtre)
    if anahtar in onbellek:
        return onbellek[anahtar]
    flutter = os.path.expanduser(r"~/tools/f344/flutter/bin/flutter.bat") if os.name == "nt" else "flutter"
    if not os.path.exists(flutter):
        flutter = "flutter"
    kod, cikti, sure = calistir(f'"{flutter}" test --plain-name "{filtre}"', kok / "app")
    m = re.findall(r"\+(\d+)(?: -(\d+))?: (?:All tests passed|Some tests failed)", cikti)
    gecen = int(m[-1][0]) if m else 0
    kalan = int(m[-1][1]) if m and m[-1][1] else 0
    sonuc = {"tur": "flutter", "filtre": filtre, "cikis": kod, "gecen": gecen, "basarisiz": kalan, "sure_sn": sure,
             "gecti": kod == 0 and gecen > 0 and kalan == 0, "son_satirlar": cikti.strip().splitlines()[-3:]}
    onbellek[anahtar] = sonuc
    return sonuc


def madde_degerlendir(kok, madde, testleri_calistir, onbellek):
    kanitlar = []
    for dosya, desen in madde.get("kanit", []):
        yol = kok / dosya
        bulundu = yol.exists() and re.search(desen, yol.read_text(encoding="utf-8", errors="replace")) is not None
        kanitlar.append({"dosya": dosya, "desen": desen, "bulundu": bool(bulundu)})
    testler = []
    for tur, filtre in madde.get("testler", []):
        if not testleri_calistir:
            testler.append({"tur": tur, "filtre": filtre, "gecti": False, "calismadi": True})
        elif tur == "cargo":
            testler.append(cargo_test(kok, filtre, onbellek))
        else:
            testler.append(flutter_test(kok, filtre, onbellek))
    kanit_tam = bool(kanitlar) and all(k["bulundu"] for k in kanitlar)
    test_tam = bool(testler) and all(t["gecti"] for t in testler)
    if madde.get("engel"):
        durum = "BLOCKED"
    elif kanit_tam and test_tam:
        durum = "DONE"
    elif any(k["bulundu"] for k in kanitlar) or any(t.get("gecti") for t in testler):
        durum = "IN_PROGRESS"
    else:
        durum = "TODO"
    return {"ad": madde["ad"], "durum": durum, "kanit": kanitlar, "testler": testler,
            "uygulandi": kanit_tam, "test_edildi": any(t.get("gecen", 0) > 0 for t in testler),
            "gecti": test_tam, "basarisiz": any(t.get("basarisiz", 0) > 0 or (t.get("cikis", 0) not in (0, None) and not t.get("calismadi")) for t in testler),
            "engel": madde.get("engel", "")}


def rapor_yaz(kok, sonuc):
    s = ["# AfuDesk Görev Kapısı — Denetim", "", f"Tarih: {sonuc['zaman']}  ·  Commit: `{sonuc['commit']}`", ""]
    toplam = {"İSTENEN": 0, "UYGULANAN": 0, "TEST EDİLEN": 0, "GEÇEN": 0, "BAŞARISIZ": 0, "ENGELLİ": 0, "EKSİK": 0, "TEST EDİLMEYEN": 0}
    for sid, sis in sonuc["sistemler"].items():
        m = sis["maddeler"]
        done = sum(1 for x in m.values() if x["durum"] == "DONE")
        s += [f"## {sid} — {sis['ad']}", "", f"Zorunlu: {len(m)} · Tamam: {done} · Eksik: {len(m) - done}", "",
              "| Madde | Durum | Kod | Test | Not |", "|---|---|---|---|---|"]
        for mid, x in m.items():
            eksik_kanit = [k["desen"] for k in x["kanit"] if not k["bulundu"]]
            kalan_test = [t["filtre"] for t in x["testler"] if not t.get("gecti")]
            not_ = x["engel"] or "; ".join(filter(None, [
                ("kod yok: " + ", ".join(eksik_kanit)) if eksik_kanit else "",
                ("test geçmedi/yok: " + ", ".join(kalan_test)) if kalan_test else ""]))
            simge = {"DONE": "✅", "IN_PROGRESS": "🟡", "TODO": "⬜", "BLOCKED": "⛔"}[x["durum"]]
            s.append(f"| {x['ad']} | {simge} {x['durum']} | {'✅' if x['uygulandi'] else '❌'} | {'✅' if x['gecti'] else '❌'} | {not_} |")
            toplam["İSTENEN"] += 1
            toplam["UYGULANAN"] += x["uygulandi"]
            toplam["TEST EDİLEN"] += x["test_edildi"]
            toplam["GEÇEN"] += x["gecti"]
            toplam["BAŞARISIZ"] += x["basarisiz"]
            toplam["ENGELLİ"] += x["durum"] == "BLOCKED"
            toplam["EKSİK"] += x["durum"] != "DONE"
            toplam["TEST EDİLMEYEN"] += not x["test_edildi"]
        s.append("")
    s += ["## Toplam", "", "| " + " | ".join(toplam) + " |", "|" + "---|" * len(toplam),
          "| " + " | ".join(str(v) for v in toplam.values()) + " |", "",
          f"**SONUÇ: {'✅ READY' if sonuc['sonuc'] == 'READY' else '❌ NOT_READY'}**", ""]
    (kok / "reports").mkdir(exist_ok=True)
    (kok / "reports" / "AUDIT.md").write_text("\n".join(s), encoding="utf-8")


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--kok", default=str(Path(__file__).resolve().parent.parent))
    ap.add_argument("--test-yok", action="store_true", help="testleri çalıştırma (yalnız kanıt tara)")
    a = ap.parse_args()
    kok = Path(a.kok)
    tanim = json.loads((kok / "kapi" / "gorevler.json").read_text(encoding="utf-8"))
    commit = calistir(["git", "rev-parse", "--short", "HEAD"], kok)[1].strip()
    onbellek = {}
    sonuc = {"zaman": time.strftime("%Y-%m-%d %H:%M"), "commit": commit, "sistemler": {}}
    hazir = True
    for sid, sis in tanim["sistemler"].items():
        maddeler = {mid: madde_degerlendir(kok, m, not a.test_yok, onbellek) for mid, m in sis["maddeler"].items()}
        sonuc["sistemler"][sid] = {"ad": sis["ad"], "zorunlu": sis.get("zorunlu", True), "maddeler": maddeler}
        if sis.get("zorunlu", True) and any(x["durum"] != "DONE" for x in maddeler.values()):
            hazir = False
    sonuc["sonuc"] = "READY" if hazir else "NOT_READY"
    (kok / "reports").mkdir(exist_ok=True)
    (kok / "reports" / "test-results.json").write_text(json.dumps(sonuc, ensure_ascii=False, indent=2), encoding="utf-8")
    rapor_yaz(kok, sonuc)
    for sid, sis in sonuc["sistemler"].items():
        m = sis["maddeler"]
        print(f"{sid}: {sum(1 for x in m.values() if x['durum'] == 'DONE')}/{len(m)} DONE")
        for mid, x in m.items():
            if x["durum"] != "DONE":
                print(f"   {x['durum']:<11} {mid}")
    print("SONUÇ:", sonuc["sonuc"])
    return 0 if hazir else 1


if __name__ == "__main__":
    sys.exit(main())
