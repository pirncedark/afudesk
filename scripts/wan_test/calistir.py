"""AfuDesk WAN testi: host + izleyiciyi ayrı süreçlerde çalıştırır, kanıtı toplar,
PASS/FAIL kararını gerekçesiyle yazar.

    python calistir.py --test 2                      # doğrudan yol kapalı -> relay
    python calistir.py --test 1 --bagla-ip auto      # izleyici telefon hattından
    python calistir.py --test 0                      # aynı ağ, duman testi

TEST-1: izleyicinin UDP'si telefon paylaşımı IP'sine bağlanır, relay (HTTPS) trafiği
de o IP'ye bağlı vekilden çıkar. İki tarafta da yerel/özel adresli yollar seçilmez
(AFUDESK_TEST_GENEL_YOL=1): LAN kısayolu değil gerçek internet kullanılır.
TEST-2: izleyicide AFUDESK_SADECE_RELAY=1 — doğrudan UDP tamamen kapalı.

Çıktı: reports/wan/TEST<n>_<zaman>/ (host_rapor.json, izle_rapor.json, ilk_kare.png,
son_kare.png, vekil.log, sonuc.json, sonuc.md)
"""
import argparse
import ipaddress
import json
import os
import subprocess
import sys
import time
from pathlib import Path

KOK = Path(__file__).resolve().parents[2]
VARSAYILAN_EXE = Path(r"D:\afudesk_derleme\wan\release\examples\wan_test.exe")
PENCERESIZ = 0x08000000  # CREATE_NO_WINDOW
VEKIL_PORT = 18080


def genel_mi(ip: str) -> bool:
    try:
        a = ipaddress.ip_address(ip)
    except ValueError:
        return False
    if isinstance(a, ipaddress.IPv4Address) and a in ipaddress.ip_network("100.64.0.0/10"):
        return False
    return a.is_global


def adres_ip(adres: str) -> str | None:
    """'1.2.3.4:5' / '[2a02::1]:5' -> ip; relay adresi -> None."""
    if not adres or adres.startswith("relay") or "://" in adres:
        return None
    if adres.startswith("ip:"):
        adres = adres[3:]
    if adres.startswith("["):
        return adres[1:].split("]")[0]
    return adres.rsplit(":", 1)[0]


def telefon_ipsi_bul() -> str | None:
    komut = (
        "Get-NetIPConfiguration | Where-Object { $_.IPv4DefaultGateway -and "
        "$_.NetAdapter.Status -eq 'Up' } | ForEach-Object { [pscustomobject]@{ ad = $_.InterfaceAlias; "
        "aciklama = $_.InterfaceDescription; ip = ($_.IPv4Address | Select-Object -First 1).IPAddress } } "
        "| ConvertTo-Json -Compress"
    )
    cikti = subprocess.run(
        ["powershell", "-NoProfile", "-Command", komut],
        capture_output=True, text=True, creationflags=PENCERESIZ,
    ).stdout.strip()
    if not cikti:
        return None
    liste = json.loads(cikti)
    if isinstance(liste, dict):
        liste = [liste]
    for a in liste:
        metin = f"{a.get('ad', '')} {a.get('aciklama', '')}".lower()
        if any(k in metin for k in ("ndis", "rndis", "bluetooth", "wi-fi", "wireless", "wlan", "mobile", "android", "iphone")):
            return a.get("ip")
    return None


def calistir(args) -> Path:
    zaman = time.strftime("%Y%m%d_%H%M%S")
    klasor = KOK / "reports" / "wan" / f"TEST{args.test}_{zaman}"
    klasor.mkdir(parents=True, exist_ok=True)
    ortak = {**os.environ, "AFUDESK_TEST_GENEL_YOL": "1" if args.test in (1, 2) else "0"}
    host_env = dict(ortak)
    izle_env = dict(ortak)
    vekil = None
    bilgi = {"test": args.test, "zaman": zaman}
    if args.test == 1:
        ip = telefon_ipsi_bul() if args.bagla_ip == "auto" else args.bagla_ip
        if not ip:
            raise SystemExit("Telefon paylaşımı arayüzü bulunamadı (USB/Bluetooth tethering açık mı?)")
        bilgi["izleyici_bagla_ip"] = ip
        izle_env["AFUDESK_BAGLA_IP"] = ip
        izle_env["AFUDESK_PROXY"] = f"http://127.0.0.1:{VEKIL_PORT}"
        vekil = subprocess.Popen(
            [sys.executable, str(Path(__file__).with_name("vekil.py")), ip, str(VEKIL_PORT), str(klasor / "vekil.log")],
            creationflags=PENCERESIZ,
        )
        time.sleep(1)
    elif args.test == 2:
        izle_env["AFUDESK_SADECE_RELAY"] = "1"
    for k in ("AFUDESK_SADECE_RELAY", "AFUDESK_BAGLA_IP", "AFUDESK_PROXY"):
        host_env.pop(k, None)
    (klasor / "bilgi.json").write_text(json.dumps(bilgi, ensure_ascii=False, indent=2), encoding="utf-8")

    exe = str(args.exe)
    with open(klasor / "host.log", "w", encoding="utf-8") as hl, open(klasor / "izle.log", "w", encoding="utf-8") as il:
        host = subprocess.Popen([exe, "host", str(klasor)], env=host_env, stdout=hl, stderr=subprocess.STDOUT,
                                creationflags=PENCERESIZ)
        try:
            izle = subprocess.Popen([exe, "izle", str(klasor), str(args.sure)], env=izle_env, stdout=il,
                                    stderr=subprocess.STDOUT, creationflags=PENCERESIZ)
            try:
                izle.wait(timeout=args.sure + 240)
            except subprocess.TimeoutExpired:
                izle.kill()
        finally:
            (klasor / "dur").write_text("1")
            try:
                host.wait(timeout=15)
            except subprocess.TimeoutExpired:
                host.kill()
            if vekil:
                vekil.kill()
    return klasor


def degerlendir(klasor: Path, test: int) -> dict:
    sebepler, notlar = [], []

    def oku(ad):
        p = klasor / ad
        return json.loads(p.read_text(encoding="utf-8")) if p.exists() else None

    h, iz, bilgi = oku("host_rapor.json"), oku("izle_rapor.json"), oku("bilgi.json") or {}
    if not iz:
        return {"sonuc": "FAIL", "sebepler": ["izle_rapor.json yok (izleyici çalışmadı)"], "notlar": []}
    if "hata" in iz:
        return {"sonuc": "FAIL", "sebepler": [f"bağlanamadı: {iz['hata']}"], "notlar": []}
    if not h:
        return {"sonuc": "FAIL", "sebepler": ["host_rapor.json yok"], "notlar": []}

    def kontrol(kosul, mesaj):
        (notlar if kosul else sebepler).append(("✅ " if kosul else "❌ ") + mesaj)

    kontrol(iz.get("kabul_sayisi", 0) >= 2, f"bağlandı ve kopuştan sonra geri döndü (kabul={iz.get('kabul_sayisi')})")
    kontrol(iz.get("kare", 0) >= 50, f"gerçek video akışı: {iz.get('kare')} kare, ort {iz.get('ort_fps', 0):.1f} fps")
    sapma = iz.get("ilk_kare_parlaklik_sapmasi") or 0
    kontrol(sapma >= 3, f"kare gerçek ekran içeriği (parlaklık sapması {sapma:.1f}; tek renk değil)")
    kontrol(iz.get("kare_donus_sonrasi", 0) >= 10,
            f"yeniden bağlandıktan sonra video sürdü ({iz.get('kare_donus_sonrasi')} kare, "
            f"dönüş {iz.get('yeniden_baglanma_ms')} ms)")
    kontrol(iz.get("koptu") is None, f"oturum beklenmedik kopmadı ({iz.get('koptu')})")

    donus_t = next((o["t"] for o in iz.get("olaylar", []) if o["tur"] == "yeniden_baglan_istendi"), None)
    fareler = [g for g in h.get("girdiler", []) if g.get("tur") == "fare"]
    dogru = [g for g in fareler if g.get("uygulandi")
             and abs(g["gercek"][0] - g["beklenen"][0]) <= 3 and abs(g["gercek"][1] - g["beklenen"][1]) <= 3]
    kontrol(len(dogru) >= 3, f"gerçek fare: {len(dogru)}/{len(fareler)} hareket işletim sisteminde doğrulandı "
            + "; ".join(f"{g['beklenen']}→{g['gercek']}" for g in fareler))
    kontrol(donus_t is not None and any(g["t"] > donus_t for g in dogru), "dönüşten sonra da fare hareketi uygulandı")
    tuslar = [g for g in h.get("girdiler", []) if g.get("tur") == "tus" and g.get("ad") == "Shift"]
    basma = any(g["basili"] and g["os_shift_basili"] for g in tuslar)
    birakma = any((not g["basili"]) and (not g["os_shift_basili"]) for g in tuslar)
    kontrol(basma and birakma, f"gerçek klavye: Shift basıldı/bırakıldı, işletim sistemi durumu doğrulandı ({len(tuslar)} olay)")

    ist = iz.get("istatistik", [])
    yollar = [s["yol"] for s in ist]
    host_yol = [o for o in h.get("olaylar", []) if o["tur"] == "yol"]
    notlar.append(f"📌 izleyici yol örnekleri: {len(ist)} (doğrudan={yollar.count('doğrudan')}, relay={yollar.count('relay')})")
    notlar.append("📌 host'un gördüğü yollar: " + ", ".join(f"{o['yol']}@{o['adres']}" for o in host_yol))
    if test == 2:
        kontrol(len(ist) >= 5 and all(y == "relay" for y in yollar),
                "doğrudan yol kapalıyken tüm trafik relay üzerinden aktı")
        kontrol(host_yol and all(o["yol"] == "relay" for o in host_yol), "host tarafı da yalnız relay gördü")
        kontrol(iz.get("ortam", {}).get("AFUDESK_SADECE_RELAY") == "1", "izleyicide doğrudan UDP kapalıydı")
    uzak_ip_dosyasi = klasor / "izleyici_ip.txt"
    if test == 1 and uzak_ip_dosyasi.exists():
        # Uzak makine kipi: izleyici başka bir bilgisayarda, başka bir ağda (GitHub Actions).
        ozel = [s["adres"] for s in ist if (ip := adres_ip(s["adres"])) and not genel_mi(ip)]
        ozel += [o["adres"] for o in host_yol if (ip := adres_ip(o["adres"])) and not genel_mi(ip)]
        kontrol(not ozel, "hiçbir veri yolu yerel/özel adres kullanmadı" + (f" (bulunan: {ozel})" if ozel else ""))
        uzak_ip = uzak_ip_dosyasi.read_text(encoding="utf-8").strip()
        hazir = next((o for o in h.get("olaylar", []) if o["tur"] == "hazir"), {})
        ev_ip = {adres_ip(a) for a in hazir.get("adresler", []) if adres_ip(a) and genel_mi(adres_ip(a))}
        kontrol(genel_mi(uzak_ip) and uzak_ip not in ev_ip,
                f"izleyici farklı internet bağlantısında: {uzak_ip} (host'un genel IP'si: {', '.join(sorted(ev_ip)) or '?'})")
        gorulen = {adres_ip(o["adres"]) for o in host_yol if o["yol"] == "doğrudan" and adres_ip(o["adres"])}
        if gorulen:
            kontrol(gorulen <= {uzak_ip} or all(genel_mi(g) and g not in ev_ip for g in gorulen),
                    f"host izleyiciyi doğrudan internetten gördü ({', '.join(sorted(gorulen))})")
            notlar.append("📌 doğrudan (NAT delme) yol KURULDU")
        else:
            notlar.append("📌 doğrudan yol kurulamadı; bağlantı relay üzerinden (izin verilen yedek yol)")
    elif test == 1:
        ozel = [s["adres"] for s in ist if (ip := adres_ip(s["adres"])) and not genel_mi(ip)]
        ozel += [o["adres"] for o in host_yol if (ip := adres_ip(o["adres"])) and not genel_mi(ip)]
        kontrol(not ozel, "hiçbir veri yolu yerel/özel adres kullanmadı" + (f" (bulunan: {ozel})" if ozel else ""))
        kontrol(iz.get("ortam", {}).get("AFUDESK_BAGLA_IP") == bilgi.get("izleyici_bagla_ip"),
                f"izleyici UDP'si telefon hattına bağlıydı ({bilgi.get('izleyici_bagla_ip')})")
        vekil = (klasor / "vekil.log").read_text(encoding="utf-8") if (klasor / "vekil.log").exists() else ""
        kontrol("CONNECT" in vekil, "izleyicinin relay bağlantısı telefon hattından çıktı (vekil.log)")
        hazir = next((o for o in h.get("olaylar", []) if o["tur"] == "hazir"), {})
        ev_ip = {adres_ip(a) for a in hazir.get("adresler", []) if adres_ip(a) and genel_mi(adres_ip(a))}
        gorulen = {adres_ip(o["adres"]) for o in host_yol if o["yol"] == "doğrudan" and adres_ip(o["adres"])}
        if gorulen:
            kontrol(not (gorulen & ev_ip),
                    f"host izleyiciyi farklı bir genel IP'den gördü ({', '.join(sorted(gorulen))}; ev: {', '.join(sorted(ev_ip))})")
            notlar.append("📌 doğrudan (hole punching) yol KURULDU")
        else:
            notlar.append("📌 doğrudan yol kurulamadı; bağlantı relay üzerinden (izin verilen yedek yol)")
    return {"sonuc": "PASS" if not sebepler else "FAIL", "sebepler": sebepler, "notlar": notlar}


def main() -> None:
    p = argparse.ArgumentParser()
    p.add_argument("--test", type=int, choices=(0, 1, 2), required=True)
    p.add_argument("--bagla-ip", default="auto")
    p.add_argument("--sure", type=int, default=25)
    p.add_argument("--exe", default=str(VARSAYILAN_EXE))
    p.add_argument("--yalniz-degerlendir", help="var olan klasörü yeniden değerlendir")
    args = p.parse_args()
    klasor = Path(args.yalniz_degerlendir) if args.yalniz_degerlendir else calistir(args)
    s = degerlendir(klasor, args.test)
    (klasor / "sonuc.json").write_text(json.dumps(s, ensure_ascii=False, indent=2), encoding="utf-8")
    md = [f"# TEST-{args.test}: {s['sonuc']}", "", f"Klasör: `{klasor}`", ""] + [f"- {x}" for x in s["sebepler"] + s["notlar"]]
    (klasor / "sonuc.md").write_text("\n".join(md) + "\n", encoding="utf-8")
    print("\n".join(md))
    sys.exit(0 if s["sonuc"] == "PASS" else 1)


if __name__ == "__main__":
    main()
