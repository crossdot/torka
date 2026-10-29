# Discovery — отчёт и следующий этап

Дата: 2026-10-29. Статус: Phase 0 завершён (без кода; только проверенные факты и ADR).

## Подтверждённое (первичные источники)

### Стек Rust
| Компонент | Версия | Лицензия (SPDX) |
|---|---|---|
| Rust stable | 1.99.1 (2026-10-15) | — |
| edition | 2024 (stable c 1.85) | — |
| axum | 0.8.9 | MIT |
| tokio | 1.54.1 | MIT |
| serde / serde_json | 1.0.229 / 1.0.154 | MIT OR Apache-2.0 |
| tracing / tracing-subscriber | 0.1.44 / 0.3.23 | MIT |
| tower / tower-http | 0.5.3 / 0.7.1 | MIT |
| hyper / http | 1.12.0 / 1.5.0 | MIT / MIT OR Apache-2.0 |

- Эффективный MSRV = **1.85** (tokio). AFIT стабилен с 1.75, но **не dyn-compatible** и
  без гарантии `Send` → provider-trait: нативный AFIT + `#[trait_variant::make(...: Send)]`;
  `async-trait` только если нужен `dyn`-registry.
- axum 0.8: пути `/{param}` (старый `/:param` паникует), нет regex-сегментов (валидация
  вручную), handler'ы `Sync`, state `Clone+Send+Sync` (→ `Arc<…>`).

### Torrent engine
- **librqbit 9.0.1, Apache-2.0**, активен (push 2026-10-27, 1962★). Единственный кандидат
  с `FileStream: AsyncRead + AsyncSeek` (random access + seek-aware приоритизация, окно 32 MiB).
- metadata через `list_only`, выбор файла `only_files`/`only_files_regex`, отмена `CancellationToken`.
- ⚠️ `FileStream` не реэкспортирован (бокс в `dyn AsyncRead + AsyncSeek`); ~8 streams по умолчанию.

### FFmpeg и потоковая передача
- FFmpeg **9.0.2** (2026-09-18); LGPL-2.1-or-later по умолчанию, GPL при `--enable-gpl`
  (Debian-пакет — GPL). HLS: MPEG-TS + fMP4 (`init.mp4`, v7+), инкрементальный live.
- **Надёжный seek-путь**: seekable loopback HTTP-range (backed by `FileStream`) + `http:`
  протокол + `-seekable 1` + перезапуск `-ss` при seek. `pipe:` **не** seekable (EPIPE).

### Провайдеры
- **RuTracker**: официальный Atom-feed `https://feed.rutracker.cc/atom/f/<id>.atom` (magnet,
  без auth/челленджа) — **только лента раздач, не поиск**. Поиск за Cloudflare → исключён.
- **Rutor**: HTML-поиск + RSS, без логина, `robots.txt` разрешает (`Crawl-delay: 3`).
  ТоS отсутствует (серая зона); возможна недоступность/блокировки.

### Docker/Synology
- DSM 7.4.1-90080 U1, Container Manager 24.0.2-1706. CM: 91 x86_64 + 7 armv8 + 0 armv7.
- Base: `debian:trixie-slim` (FFmpeg 7.1.5); Rust static musl. `linux/amd64`.
- VA-API только Intel-iGPU (Celeron J4xxx…), нет на AMD/ARM/Atom/Xeon; требует физической валидации.

## ADR (см. `adr/`)
- ADR-001 torrent engine → librqbit (embedded).
- ADR-002 streaming/seek → seekable loopback HTTP-range + FFmpeg HLS, seek через `-ss` restart.
- ADR-003 providers → RuTracker feed-only, Rutor HTML-поиск (robots-compliant).
- ADR-004 frontend → Axum раздаёт SPA.
- ADR-005 auth → один Bearer token (env), закрыто по умолчанию.
- ADR-006 Synology → amd64, debian:trixie-slim, CI-сборка, VA-API Intel/AMD.
- ADR-007 metadata → без внешнего каталога в MVP.

## Минимальный вертикальный прототип (spike, до Phase 1/5)

Цель — проверить рискованную гипотезу «torrent → seekable поток → FFmpeg HLS» до затрат
на полную реализацию. Самый тонкий сквозной срез:

1. **Контролируемый тестовый контент**: собственный короткий MP4 + собственный `.torrent`
   (создан локально), засеянный локальным peer (или `list_only` + заранее доступный файл).
2. librqbit embedded: `Session` → `add_torrent(list_only)` → выбрать файл → `api_stream(file_id)`.
3. Loopback HTTP-range handler: `Content-Length` из metadata, `Accept-Ranges`, `206`, чтение
   из `FileStream` (backpressure = `Poll::Pending`).
4. `ffprobe http://127.0.0.1:PORT/...` (с `-seekable 1`) → подтвердить инспекцию контейнера.
5. `ffmpeg -i http://... -c copy` → HLS (MPEG-TS, live-style) в директорию сессии;
   публикация сегментов атомарно.
6. Axum: `GET .../master.m3u8` и `.../segments/{n}`; критерии: time-to-first-playable-segment,
   seek через `-ss` restart, «файл не загружен целиком», stop→cleanup.
7. **Gate**: если seek/remux/HLS не подтверждаются на этом срезе — зафиксировать ограничения
   и согласовать изменение требований (как требует STREAMING.md §18).

Результат среза: подтверждение ADR-001/002 на реальном engine+FFmpeg до Phase 5.

## Решения владельца (2026-10-29) — блокеры закрыты

1. **Целевая платформа**: x86_64, vendor-agnostic GPU — Intel (VA-API), NVIDIA (NVENC),
   AMD (VA-API/Mesa); запуск/тест сейчас на Intel. Модель/RAM далее не уточняем.
2. **Лицензии**: непринципиальны → допустима GPL-сборка FFmpeg (с libx264 → software H.264).
3. **Легальность контента/метаданных**: не блокер.
4. **Провайдеры**: Rutor включён (HTML-поиск); RuTracker — feed-only (поиск технически за
   Cloudflare, обход не выполняется).
5. **Тестовый контент**: любой воспроизводимый torrent для PoC.
6. Техническое ограничение (не «легальность»): CAPTCHA/Cloudflare/антибот не обходим —
   RuTracker-поиск остаётся недоступным.
7. «Без записи медиа на диск» не гарантировано (STREAMING.md §11.2); временный cache ограничен.

## Предлагаемый следующий этап (утверждено)

**Phase 0.5 (spike) + Phase 1 (Foundation)**: Rust-проект (edition 2024, MSRV 1.85), config/tracing,
Axum health/readiness, модель ошибок, provider trait (AFIT + trait-variant), Vue scaffold,
OpenAPI, Docker skeleton; spike закрывает риск streaming (ADR-001/002) до Phase 5. Сборка —
`linux/amd64` с Intel VA-API (software fallback обязателен).
