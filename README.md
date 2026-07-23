<div align="center">

# 🪞 Kagami

**Бесплатный AirPlay-приёмник для Windows с упором на качество и низкую задержку.**

Зеркаль экран iPhone / iPad / Mac прямо в окно на ПК — без подписок, без облака, без рекламы.
Аналог LetsView / AirServer, только open-source.

![platform](https://img.shields.io/badge/platform-Windows%2010%2F11-0078D6)
![license](https://img.shields.io/badge/license-GPLv3-blue)
![based on](https://img.shields.io/badge/fork%20of-UxPlay-lightgrey)
![decode](https://img.shields.io/badge/decode-D3D12%20HW-76B900)

</div>

---

## Что это

Kagami — форк [UxPlay](https://github.com/FDH2/UxPlay), допиленный под Windows и реальное
использование как продукт. AirPlay-приёмник ловит зеркалированный поток с устройств Apple,
декодирует его аппаратно (Direct3D12) и показывает в окне.

**Отличия от голого UxPlay:**
- 🚀 Тюнинг задержки из коробки (`-vsync no -fps 60`)
- 📡 Обход сломанного Apple Bonjour — свой mDNS-рефлектор, чтобы устройства видели ПК в LAN
- 👥 Две одновременные сессии (два устройства на экране разом)
- 🔧 Готовые лаунчеры `.bat`, не нужно возиться с флагами

## Возможности

| | |
|---|---|
| **Источники** | iPhone, iPad, Mac (AirPlay mirroring) |
| **Качество** | 1080p60 (потолок legacy-AirPlay; 2K/4K — в планах через AirPlay 2) |
| **Декод** | аппаратный, Direct3D12 (проверено на RTX 4070 Ti) |
| **Мультиэкран** | до 2 устройств одновременно |
| **Стоимость** | бесплатно, GPLv3 |

## Требования

- Windows 10/11
- [MSYS2](https://www.msys2.org/) (UCRT64) с GStreamer — для сборки
- Python 3.13 + `zeroconf` — для mDNS-рефлектора
- Apple Bonjour (обычно уже стоит с iTunes/Apple-софтом)
- ПК и устройство в одной Wi-Fi/LAN-сети

## Быстрый старт

```bat
:: одно устройство
uxplay.bat

:: два устройства одновременно
uxplay-dual.bat
```

Затем на iPhone/Mac открой **Пункт управления → Повтор экрана** и выбери **Kagami**.

## Как это устроено

```
iPhone/Mac ──mDNS──▶ reflector.py (zeroconf, прибит к LAN IP)
           ──RTSP/RTP──▶ uxplay.exe ──▶ GStreamer ──▶ D3D12-окно
```

Штатный Bonjour на этой машине анонсировал сервис только в Tailscale-интерфейс, и LAN его
не видел. `reflector.py` снимает живые записи через `dns-sd -Z` (IPC) и переанонсирует их
в локальную сеть с корректным адресом.

## Дорожная карта

- **P1** — teardown при закрытии окна, нативный mDNS (без Bonjour), мультисессия в одном процессе
- **P2** — AirPlay 2 buffered video → настоящие 2K/4K (HomeKit-пейринг, FairPlay v3, HEVC)
- **Android** — приём экрана с Android через Miracast (отдельная ветка, отложено)
- **P3** — трей-UI, инсталлятор, пин-код подключения

Подробности — в [`ROADMAP.md`](ROADMAP.md).

## Лицензия

GPLv3 — как и оригинальный UxPlay. См. [`LICENSE`](LICENSE).

## Благодарности

Основано на [UxPlay](https://github.com/FDH2/UxPlay) (F. Duncanh и др.) и его предшественниках
(RPiPlay, AirplayServer). Kagami — надстройка над их работой.
