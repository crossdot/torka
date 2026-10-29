# ADR-006: Docker и Synology (целевые архитектуры, base image, VA-API)

- Status: Proposed
- Date: 2026-10-29
- Owners: ведущий инженер (Discovery)

## Context

Docker-образ должен работать на Synology DSM через Container Manager, в multi-arch,
с опциональным Intel VA-API и гарантированным software fallback.

## Findings (проверено 2026-10-29)

- DSM последний: **7.4.1-90080 Update 1**; Container Manager **24.0.2-1706** (требует DSM 7.4+).
- CM 24.0.2-1706 поддерживает **91 × x86_64 + 7 × armv8 + 0 × ARMv7** моделей
  (кросс-проверка `appliedModels` пакета и CPU KB).
- Модели начального уровня: 512 МБ–2 ГБ RAM (DS120j/DS220j/DS124/DS223j/DS420j) — для
  FFmpeg + source cache это неработоспособно.
- VA-API: официально Synology документирует GPU-проброс только для NVIDIA; `/dev/dri`
  не документирован. Intel iGPU (VA-API) возможен только на моделях с Celeron
  J4025/J4125/J3455/J3355 и т.п.; нет на AMD (Ryzen), ARM (Realtek/Marvell), Atom-C/Xeon-D.
  Наличие `/dev/dri` не гарантирует рабочий VA-API (для DS225+/DS425+ есть community-отчёт
  о деградированном i915 — неофициально). **Реальная валидация VA-API требует физического host.**

## Decision

1. **Целевая платформа: x86_64** (`linux/amd64`) с vendor-agnostic GPU: Intel (VA-API),
   NVIDIA (NVENC/NVDEC), AMD (VA-API/Mesa). Запуск/тест сейчас на Intel. См. ADR-008.
2. **Runtime base `debian:trixie-slim`** (version-tagged): FFmpeg 7.1.5, `libva2`,
   `vainfo`; при VA-API — `intel-media-va-driver-non-free` (полная сборка, из `non-free`).
   Rust-бинарь собирается **glibc в том же trixie** (builder `rust:1.99-slim` + runtime
   `debian:trixie-slim`). Deviation от исходного плана (static musl): `librqbit` →
   `openssl-sys` требует vendored OpenSSL для `*-musl`, поэтому остановились на
   согласованной glibc-версии builder==runtime (зафиксировано в `Dockerfile`).
3. **Сборка в CI** (buildx + QEMU, `--push`), не на NAS; на 512 МБ–2 ГБ ARM сборка невозможна.
4. **Runtime-детект VA-API**: probe `/dev/dri/renderD*`, затем реальная инициализация и
   тестовый encode; `HWACCEL_MODE=auto` → software fallback. 512 МБ–1 ГБ модели отметить
   unsupported (не «медленные»).
5. Compose: `no-new-privileges`, проброс GPU через `devices: ["/dev/dri:/dev/dri"]` +
   `group_add` (после проверки на железе); `deploy:` лимиты задаются в Container Manager.

## Consequences

- Лицензия FFmpeg: **владелец выбрал GPL-сборку** (Debian-пакет, с libx264) → software H.264
  доступен; чисто-LGPL не требуется (решение 2026-10-29).
- GPU-ускорение зависит от фактически установленного GPU, а не только от Intel:
  Intel iGPU и дискретные AMD — через VA-API, NVIDIA — через NVENC. На Synology-моделях
  без iGPU/dGPU (AMD Ryzen NAS, ARM, Atom-C/Xeon-D) доступен только software fallback.

## Validation

- Synology releaseNote API; ContainerManager `appliedModels` (117 моделей); CPU KB (213 моделей);
  Docker multi-platform docs (buildx/QEMU).
