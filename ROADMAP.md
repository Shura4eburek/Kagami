# Kagami — план форка UxPlay

## Сделано
- [x] Сборка UxPlay 1.74 (MSYS2 UCRT64, GStreamer, D3D12-декод)
- [x] Обход сломанного Bonjour: `reflector.py` (python-zeroconf, анонс прибит к LAN)
- [x] Файрвол-правило, лаунчеры `uxplay.bat` / `uxplay-dual.bat`
- [x] Латенси-тюнинг: `-vsync no -fps 60`
- [x] Две одновременные сессии (2 инстанса, `-m` для уникального MAC)
- [x] Выяснен потолок качества: legacy AirPlay = 1080p (источник кодирует 1080p
      даже при запросе 1440p — проверено логом `source 1920x1080`)
- [x] **Десктоп-приложение (Tauri v2 + Rust)** в `desktop/`: тёмный Apple-UI,
      фикс-окно 720×617, вкладки Устройства/Настройки, кастомный трей-флайаут
      (прижат к иконке), своя иконка-зеркало, старт/стоп из UI спавнит
      uxplay.exe с GStreamer-PATH и авто-рефлектор
- [x] Windows Miracast-приёмник настроен (фича «Wireless Display» + Wi-Fi Direct)
      для будущего Android-теста — ждёт локальный девайс

## Backlog форка

### P1 — ядро
- [ ] **RTSP TEARDOWN при закрытии окна** — сейчас закрытие окна не дисконнектит
      клиента (известное поведение UxPlay; слать teardown из обработчика окна)
- [ ] **Нативный mDNS вместо Bonjour** — убрать зависимость от Apple Bonjour и
      костыля-рефлектора: DnsServiceRegister (dnsapi.dll, Win10+) или свой
      mDNS-стек в dns_sd.c
- [ ] **Мультисессия в одном процессе** — N клиентов без dual-bat: несколько
      raop-инстансов, плитка окон, имена Kagami-1..N

### P2 — качество
- [ ] **AirPlay 2 buffered video → настоящие 2K/4K** — БОЛЬШОЙ ЭТАП:
      HomeKit-пейринг (SRP), FairPlay v3, buffered-протокол, HEVC.
      Без этого 1080p — жёсткий потолок отправителя
- [ ] Разобраться с 16:9 на Mac — смена разрешения дисплея при зеркалировании
      (display-параметры в plist ответа /info)
- [ ] Проверить iPhone-фриз после `-vsync no` (iOS 27 beta)
- [x] **iPhone: нет окна при `-s WxH@60`** — `@60` кладёт `refreshRate=60` в
      /info-plist, iOS 27 beta от этого ломает поток (окно не создаётся).
      Фикс: убран `-s` (дефолт и так 1920x1080). Разобраться глубже при
      работе над AirPlay 2

### Android receiver (отдельная ветка, не AirPlay)
- [ ] **Приём экрана с Android** — Android не умеет AirPlay. Пути:
      - **Miracast** (предпочт.): Wi-Fi Direct + RTSP + H264/MPEG2-TS, близко к
        текущему пайплайну; в Windows уже есть встроенный приёмник
        (`ms-settings:project`). Работает на Samsung/Xiaomi/LG, но стоковый
        Android/Pixel Miracast выпилил с Android 6
      - **Google Cast**: проприетарный (`_googlecast._tcp`, CASTV2 protobuf),
        реверс тяжёлый, по-хорошему нужна сертификация — почти неподъёмно
      - ⏸ ОТЛОЖЕНО: нет Android-устройства для теста. Выбор протокола зависит
        от модели (проверить шторку: «Трансляция»/«Smart View» = Miracast)

### P3 — продукт
- [ ] Свой UI: трей-иконка, старт/стоп, список подключённых устройств
- [ ] Автостарт с Windows, единый инсталлятор (uxplay + reflector + правила)
- [ ] Пин-код подключения (`-pin`), настройки качества из UI

## Архитектура (текущая)
```
iPhone/Mac ──mDNS──> reflector.py (zeroconf @ 192.168.1.102)
           ──RTSP/RTP──> uxplay.exe (порт эфемерный) ──> GStreamer ──> D3D12 окно
```
Bonjour (iTunes) сломан: анонсирует только в Tailscale (if 8), LAN не видит.
Рефлектор снимает живые записи через `dns-sd -Z` (IPC) и переанонсирует в LAN
с SRV-целью `kagami.local`.
