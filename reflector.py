"""mDNS-рефлектор: переанонсирует AirPlay-сервисы uxplay на LAN-интерфейс.

Древний Bonjour (iTunes) анонсирует только в Tailscale — этот скрипт снимает
живые записи через dns-sd (IPC к mDNSResponder) и регистрирует их заново
через python-zeroconf, прибитый к LAN IP. SRV-цель подменяется на свой
хостнейм, чтобы iPhone резолвил ровно наш LAN-адрес.
"""
import re
import socket
import subprocess
import sys
import time

from zeroconf import Zeroconf, ServiceInfo

INSTANCE_FILTER = "MamoruScreen"      # часть имени наших сервисов
HOST_ALIAS = "mamoru-screen.local."   # SRV-цель, резолвится только в LAN IP
ROUTER_IP = "192.168.1.1"             # для определения LAN-адреса


def lan_ip() -> str:
    s = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
    s.connect((ROUTER_IP, 80))
    ip = s.getsockname()[0]
    s.close()
    return ip


def snapshot(service_type: str) -> str:
    """Снять zone-дамп через dns-sd -Z (работает через IPC, не через сеть)."""
    p = subprocess.Popen(
        ["dns-sd", "-Z", service_type, "local."],
        stdout=subprocess.PIPE, text=True, encoding="utf-8", errors="replace",
    )
    time.sleep(4)
    p.kill()
    return p.stdout.read()


def parse(dump: str, service_type: str):
    """Достать (instance, port, txt_dict) наших сервисов из zone-дампа."""
    srv_re = re.compile(rf"^(\S+)\.{re.escape(service_type)}\s+SRV\s+\d+\s+\d+\s+(\d+)\s+(\S+)", re.M)
    txt_re = re.compile(rf"^(\S+)\.{re.escape(service_type)}\s+TXT\s+(.*)$", re.M)
    ports, txts = {}, {}
    for m in srv_re.finditer(dump):
        ports[m.group(1)] = int(m.group(2))
    for m in txt_re.finditer(dump):
        txts[m.group(1)] = dict(
            kv.split("=", 1) for kv in re.findall(r'"((?:[^"\\]|\\.)*)"', m.group(2)) if "=" in kv
        )
    for inst, port in ports.items():
        if INSTANCE_FILTER in inst:
            yield inst, port, txts.get(inst, {})


def main():
    ip = lan_ip()
    addr = socket.inet_aton(ip)
    zc = Zeroconf(interfaces=[ip])
    registered = []

    for stype in ("_airplay._tcp", "_raop._tcp"):
        dump = snapshot(stype)
        for inst, port, txt in parse(dump, stype):
            info = ServiceInfo(
                type_=f"{stype}.local.",
                name=f"{inst}.{stype}.local.",
                port=port,
                properties={k: v.encode() for k, v in txt.items()},
                server=HOST_ALIAS,
                addresses=[addr],
            )
            zc.register_service(info, allow_name_change=False)
            registered.append(info)
            print(f"[reflector] {inst}.{stype} -> {ip}:{port} ({len(txt)} TXT)")

    if not registered:
        print("[reflector] ОШИБКА: сервисы uxplay не найдены — uxplay запущен?")
        sys.exit(1)

    print("[reflector] Анонс активен, Ctrl+C для остановки")
    try:
        while True:
            time.sleep(3600)
    except KeyboardInterrupt:
        pass
    finally:
        for info in registered:
            zc.unregister_service(info)
        zc.close()


if __name__ == "__main__":
    main()
