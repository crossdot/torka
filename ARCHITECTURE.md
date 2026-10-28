# ARCHITECTURE.md

## Форма
Модульный монолит: один Rust backend в Docker. FFmpeg — дочерний процесс. Torrent engine выбирается после PoC/ADR: Rust library или отдельный локальный компонент, если это обосновано.

## Модули
- `api`: маршруты, DTO, преобразование ошибок.
- `config`: env/TOML и валидация.
- `search`: координация, timeout, нормализация, dedupe.
- `providers`: RuTracker, Rutor и будущие адаптеры.
- `metadata`: необязательный каталог метаданных.
- `torrent`: resolver, metadata и выбор файла.
- `streaming`: sessions, FFmpeg, HLS, cleanup.
- `state`: in-memory sessions и ограниченный cache.

## Границы
`TorrentSearchProvider` только ищет и нормализует результаты. Он не запускает FFmpeg.
`TorrentResolver` валидирует источник и выбирает файл.
`SessionManager` создаёт/останавливает сессии и управляет лимитами.
`FfmpegRunner` управляет процессом, не принимает произвольные аргументы клиента.
`SearchService` объединяет результаты и изолирует сбои.

## Состояние
В MVP нет обязательной БД/Redis. Активные сессии и короткий кэш — в памяти; временный cache directory опционален. После рестарта сессии завершаются и теряются.

## Лимиты
Настроить timeout поиска, max parallel providers, max sessions, idle timeout, resolve timeout, FFmpeg startup timeout, cache size/TTL, max transcode sessions.

## Поток
Search → providers → normalized results → API.
Create playback session → validate source → resolver → select video → buffering/seek strategy → FFmpeg/remux/transcode → HLS → client.
Обязательно проверить seek/random access: последовательный pipe не гарантирует работу для всех MP4/MKV.

## API
Версионировать `/api/v1`, OpenAPI — источник истины. Не раскрывать внутренние пути, команды FFmpeg, credentials или подробности инфраструктуры.
