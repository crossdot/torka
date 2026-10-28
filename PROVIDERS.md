# PROVIDERS.md — поисковые провайдеры

## Модель
Provider возвращает нормализованный `TorrentResult`: id (opaque result id), title, year, provider_id, size_bytes, seeders/leechers, info_hash, magnet_available, quality/languages, source_url при наличии. Не все поля доступны; неизвестное остаётся null. Поле `id` результата передаётся как `source_id` при создании playback-сессии.

## Первые кандидаты
- `rutracker`
- `rutor`

Перед реализацией проверить доступность и допустимый способ взаимодействия. Не считать интеграцию гарантированно работающей. Не обходить CAPTCHA, авторизацию или антибот-защиту. Не хранить credentials без согласованного безопасного механизма. Если допустимый путь отсутствует, provider disabled/unavailable с диагностикой.

## Trait и registry
Статический registry в MVP; провайдеры включаются конфигурацией. Пример концептуального интерфейса:
- `id()`
- async `search(query) -> Result<Vec<TorrentResult>, ProviderError>`
- optional lightweight health info.

Выбрать async trait подход с учётом текущей версии Rust и зависимостей, решение зафиксировать.

## SearchService
- параллельный запуск с общим deadline и per-provider timeout;
- ограниченная конкурентность;
- частичные результаты;
- query normalization;
- dedupe по валидированному info hash;
- не объединять разные раздачи только по похожему названию;
- status каждого провайдера;
- rate limit/backoff.

## Парсеры
Тестировать на локальных HTML fixtures. Не использовать live sites в unit tests. Ограничивать response size и parsing time. Валидировать ссылки и redirects. Не исполнять скачанные файлы. Изменение HTML не должно падать всем сервисом.

## Ручной magnet
Можно добавить отдельный flow. Проверять `magnet:` и допустимый `xt=urn:btih:` формат; ограничивать длину. Не принимать `file://`, произвольные HTTP URL или filesystem path как magnet.
