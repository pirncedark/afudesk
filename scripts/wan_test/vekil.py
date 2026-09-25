"""HTTP CONNECT vekili: giden her TCP bağlantısını verilen yerel IP'ye bağlar.

TEST-1'de izleyicinin relay (HTTPS) trafiği de telefon paylaşımı arayüzünden çıksın
diye kullanılır. Her bağlantı hedefi vekil.log'a yazılır (kanıt).

    python vekil.py <bagla_ip> <port> <log_dosyasi>
"""
import socket
import sys
import threading
import time

BAGLA_IP, PORT, LOG = sys.argv[1], int(sys.argv[2]), sys.argv[3]
kilit = threading.Lock()


def kaydet(satir: str) -> None:
    with kilit, open(LOG, "a", encoding="utf-8") as f:
        f.write(f"{time.strftime('%H:%M:%S')} {satir}\n")


def aktar(a: socket.socket, b: socket.socket) -> None:
    try:
        while True:
            veri = a.recv(65536)
            if not veri:
                break
            b.sendall(veri)
    except OSError:
        pass
    finally:
        for s in (a, b):
            try:
                s.shutdown(socket.SHUT_RDWR)
            except OSError:
                pass


def isle(istemci: socket.socket) -> None:
    try:
        baslik = b""
        while b"\r\n\r\n" not in baslik:
            parca = istemci.recv(4096)
            if not parca:
                return
            baslik += parca
            if len(baslik) > 16384:
                return
        ilk = baslik.split(b"\r\n", 1)[0].decode("latin-1")
        yontem, hedef, _ = ilk.split(" ", 2)
        if yontem.upper() != "CONNECT":
            istemci.sendall(b"HTTP/1.1 405 Method Not Allowed\r\n\r\n")
            return
        ana, port = hedef.rsplit(":", 1)
        uzak = socket.create_connection((ana, int(port)), timeout=15, source_address=(BAGLA_IP, 0))
        uzak.settimeout(None)
        kaydet(f"CONNECT {hedef} kaynak={uzak.getsockname()[0]}")
        istemci.sendall(b"HTTP/1.1 200 Connection established\r\n\r\n")
        artik = baslik.split(b"\r\n\r\n", 1)[1]
        if artik:
            uzak.sendall(artik)
        t = threading.Thread(target=aktar, args=(uzak, istemci), daemon=True)
        t.start()
        aktar(istemci, uzak)
        t.join()
    except Exception as e:  # noqa: BLE001 - vekil hiçbir bağlantıda çökmemeli
        kaydet(f"HATA {e!r}")
        try:
            istemci.sendall(b"HTTP/1.1 502 Bad Gateway\r\n\r\n")
        except OSError:
            pass
    finally:
        istemci.close()


def main() -> None:
    dinle = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
    dinle.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
    dinle.bind(("127.0.0.1", PORT))
    dinle.listen(64)
    kaydet(f"BASLADI 127.0.0.1:{PORT} -> kaynak {BAGLA_IP}")
    while True:
        c, _ = dinle.accept()
        threading.Thread(target=isle, args=(c,), daemon=True).start()


if __name__ == "__main__":
    main()
