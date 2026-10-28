# STREAMING.md — torrent + FFmpeg + HLS

## Цель
Начать playback до полной загрузки файла. Torrent engine может использовать ограниченный временный cache; строгий режим без записи на диск требует отдельного PoC.

## Обязательный PoC
На собственном/разрешённом тестовом материале проверить:
- torrent metadata и выбор файла;
- буферизацию нужных частей;
- random access/seek;
- MP4 и MKV отдельно;
- FFmpeg remux to HLS;
- transcode fallback;
- time-to-first-frame;
- остановку и очистку;
- поведение при медленных/недоступных peers.

Не считать stdin pipe универсальным решением: некоторые контейнеры требуют seek. Выбранная стратегия должна подтверждаться тестами.

## Torrent engine ADR
Проверить зрелость, Rust API, random access/seek, caching, license, multi-arch, DHT/tracker/peer support, resource limits, cancellation. Если зрелой Rust-библиотеки не хватает, рассмотреть изолированный компонент только с обоснованием.

## FFmpeg
Запуск через `tokio::process::Command` или проверенную обёртку. Аргументы отдельными элементами, не shell string. Не принимать произвольные flags/URLs от клиента. Ограничить startup timeout, stderr size, число процессов; реализовать cancellation, graceful termination, forced kill after grace period, wait/reap.

## Режимы
- Remux/copy при совместимых кодеках/контейнере.
- Transcode только при необходимости.
- HLS как основной web playback candidate.
- Проверить кодеки Android TV отдельно.

## Sessions
Состояния: `resolving → buffering → ready/playing → stopped`, либо `failed`.
Операции: create/status/stop, idle timeout, process cleanup, temp cleanup, shutdown cleanup.

## Ресурсы
Лимиты активных сессий, транскодирований, буфера, cache bytes, TTL, resolve timeout, соединения/скорость при поддержке engine. Метрики: startup latency, time to first frame, buffering, active sessions, provider timeout, FFmpeg exit code, cache bytes. Не логировать секретные URL.
