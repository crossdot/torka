# ADR-008: Аппаратное ускорение — vendor-agnostic backend

- Status: Proposed
- Date: 2026-10-29
- Owners: ведущий инженер (Discovery)

## Context

Слой аппаратного ускорения не должен завязываться на одного vendor. Целевые GPU:
Intel (VA-API), NVIDIA (NVENC/NVDEC), AMD (VA-API через Mesa). Сейчас запускаем и тестируем
на Intel, но интерфейс должен допускать NVIDIA/AMD без переписывания ядра.

## Options

1. Жёсткая привязка к Intel VA-API (исходный STREAMING.md §6).
2. Абстракция backend'а + рантайм-детект доступности + fallback.

## Decision

**Вариант 2.** Вводится перечисление backend'ов и единая точка выбора FFmpeg-аргументов:

- `HwAccelBackend { Auto, Software, Vaapi, Nvenc }` (`HWACCEL_MODE=auto|software|vaapi|nvenc`).
- `HWACCEL_DEVICE` — опциональный device node (для vaapi обычно `/dev/dri/renderD128`),
  пустое = автоопределение.
- Единый маппинг backend → FFmpeg (`-hwaccel`/`-init_hw_device`/энкодеры: `h264_vaapi`,
  `hevc_vaapi`, `h264_nvenc`, `hevc_nvenc`) + `-filter_hw_device`.
- Рантайм-детект: probe устройства, затем репрезентативный encode-тест; `auto` выбирает
  доступный hardware, иначе software fallback (при `HWACCEL_REQUIRED=false`; при `true`
  — различимая ошибка).

## Consequences

- Intel VA-API — первый реализованный/тестируемый backend (текущая машина).
- NVIDIA (NVENC) и AMD (VA-API) — целевые; детект их устройств (`nvidia-smi`/CUDA presence,
  `/dev/dri/renderD*`) закладывается в интерфейс, но не тестируется сейчас.
- NVIDIA-энкодеры (`h264_nvenc`) есть в LGPL-сборке FFmpeg, но требуют драйвер NVIDIA
  в образе; AMD работает через штатный VA-API (Mesa). Чисто-LGPL не требуется (решение владельца).
- Фактически выбранный backend и причина fallback должны различаться в диагностике
  (STREAMING.md §6.5, §16.1).

## Validation

- FFmpeg 9.0.2: `h264_vaapi`/`h264_nvenc`/`hevc_*`, `-hwaccel vaapi|cuda`
  (ffmpeg.org/ffmpeg-codecs.html). Реальная проверка на конкретном GPU — отдельным шагом.
