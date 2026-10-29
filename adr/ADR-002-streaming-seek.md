# ADR-002: Streaming — seekable loopback HTTP range + FFmpeg HLS

- Status: Proposed
- Date: 2026-10-29
- Owners: ведущий инженер (Discovery)

## Context

MP4/MKV могут требовать произвольного доступа к данным (moov/cluster-структуры).
FFmpeg запускается отдельным процессом и не может напрямую потреблять in-memory
Rust-stream. Нужен способ подать seekable поток в FFmpeg без полной загрузки файла
и с backpressure при отсутствии peers.

## Options

1. **Seekable loopback HTTP range endpoint**: Rust-обработчик на 127.0.0.1 отдаёт
   выбранный файл с `Accept-Ranges: bytes`, `206`/`Content-Range`; чтение из
   `FileStream` (AsyncRead+AsyncSeek). FFmpeg читает через протокол `http:` с `-seekable 1`.
2. Локальный sparse/cache-файл, читаемый FFmpeg напрямую — нет backpressure, FFmpeg
   «обгонит» загрузку и прочитает нули в дырах; неприемлемо.
3. `pipe:` в stdin — **не seekable**: у `pipe:` нет seek-callback, `avio_seek` падает
   `EPIPE` (подтверждено `aviobuf.c` n9.0.2).
4. `pipe:` + обёртка `cache:` — добавляет seek, но кеширует в неограниченный temp-файл.
5. Строгий режим без записи на диск — требует отдельного PoC, не гарантирован.

## Decision

**Модель 1 (seekable loopback HTTP range) как основная.**

- librqbit `FileStream` (AsyncRead+AsyncSeek) → наш HTTP-обработчик диапазонов
  (loopback, авторизованный по сессии) → FFmpeg `http://127.0.0.1:PORT/...` с `-seekable 1`.
- `Content-Length` берётся из metadata торрента (длина файла известна до загрузки).
- Backpressure естественный: `AsyncRead::poll_read` возвращает `Poll::Pending`, пока
  piece не скачан.
- HLS: FFmpeg 9.0.2 muxer, MPEG-TS (по умолчанию) или fMP4 (`init.mp4`, HLS v7+),
  инкрементальный live-style вывод (`omit_endlist`, `append_list`, `event`, `hls_list_size=0`)
  в директорию сессии; публикация сегментов атомарно (temp + rename).
- **Seek**: live-style HLS; seek в уже опубликованные сегменты — по готовому плейлисту.
  Seek в ещё не загруженную область — новый generation FFmpeg с `-ss <ts>` (перезапуск
  с актуальной позиции), старый generation отменяется и не публикует сегменты.
  `pipe:` не используется.

## Consequences

- Нужен тонкий HTTP-слой диапазонов внутри нашего процесса (не раскрывать наружу;
  привязан к сессии и авторизации).
- `FileStream` держит semaphore permit (см. ADR-001); параллелизм сегментов ограничен
  `runtime_worker_threads`.
- FFmpeg `http:` demuxer имеет собственные таймауты — при медленных/отсутствующих peers
  нужен явный buffering timeout и различимая ошибка.
- Временное хранилище: HLS-сегменты + source cache ограничены (TTL + лимит размера),
  очищаются по stop/expiry/shutdown.

## Validation

- librqbit: `FileStream: AsyncRead + AsyncSeek` (`streaming.rs:167,246`); собственный
  Range-handler rqbit (`http_api/handlers/streaming.rs`, `Accept-Ranges: bytes`, `206`).
- FFmpeg: `pipe:` не seekable (`aviobuf.c` `AVERROR(EPIPE)`); `http:` поддерживает Range/seek;
  `cache:` protocol «brings seeking capability to live streams» (ffmpeg-protocols.html).
- FFmpeg 9.0.2 (2026-09-18), LGPL-2.1-or-later по умолчанию.
