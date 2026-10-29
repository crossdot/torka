# ADR-003: Интеграция RuTracker и Rutor (без обхода ограничений)

- Status: Proposed
- Date: 2026-10-29
- Owners: ведущий инженер (Discovery)

## Context

Кандидаты — RuTracker и Rutor. Жёсткое ограничение: **не обходить** CAPTCHA, авторизацию,
Cloudflare, антибот-защиту; не хранить credentials без согласованного механизма.
Провайдеры должны быть независимо включаемыми/удаляемыми и давать частичные результаты.

## Findings (проверено 2026-10-29, первичные источники)

**RuTracker.org**
- Документированного API/поиска нет. `/forum/tracker.php` (поиск) и `/forum/viewtopic.php`
  закрыты Cloudflare-челленджем (`403 "Just a moment..."`) → обход исключён.
- **Официальный Atom-feed есть**: `https://feed.rutracker.cc/atom/f/<forum_id>.atom`
  (сайт сам ссылается на него как «Последние раздачи»). Проверен: `200`,
  `application/atom+xml`, содержит `<entry>` с **magnet-enclosure** (`xt=urn:btih:...`),
  отдаётся без челленджа и без авторизации.
- Это **лента последних раздач по категории, НЕ поиск** по произвольному запросу.
- `robots.txt` — Cloudflare-managed: общий обход разрешён, AI-обучение запрещено.

**Rutor.info**
- Документированного API нет. RSS объявлен (`/rss.php?category`), но **без magnet** —
  только ссылки на топики.
- HTML-поиск `/search/<query>` открыт, без логина, без CAPTCHA/Cloudflare (по архивам);
  magnet-ссылки в результатах анонимно.
- `robots.txt`: `User-agent: *` + `Crawl-delay: 3`, без `Disallow` → обход разрешён с
  паузой 3 с.
- Хост с части сетей недоступен и заблокирован в РФ (зеркало `rutor.is`).
- Публичной лицензии на метаданные нет ни у одного сайта.

## Decision

- **RuTracker = feed-only провайдер** («последние раздачи по категории»), парсинг Atom
  + magnet-enclosures. Поиск **не** реализуем без обхода Cloudflare → в UI это не «поиск».
  При недоступности feed — провайдер disabled/unavailable с диагностикой.
- **Rutor = HTML-поиск + RSS**, с `Crawl-delay: 3`, кешем, descriptive UA, без логина.
  Контракт HTML считать нестабильным; парсеры только на локальных фикстурах.
- Оба провайдера включаются конфигурацией (`PROVIDER_*_ENABLED`, по умолчанию `false`),
  за статическим registry. Credentials не хранятся.
- Политика проекта: если требуется строго «документированная интеграция» → RuTracker
  остаётся feed-only (допустимо), Rutor — отключить (серая зона, ToS отсутствует).

## Consequences

- Поиск в MVP фактически обеспечен только Rutor (+ будущие провайдеры); RuTracker
  добавляет ленту свежих раздач, а не полнотекстовый поиск.
- Rutor может быть недоступен (блокировки) — обрабатывать как частичный сбой провайдера.
- Отсутствие лицензии на метаданные — правовой риск, вынести в согласование.

## Validation

- RuTracker feed: `https://feed.rutracker.cc/atom/f/0.atom` (200, 150 `<entry>`, magnet).
- Rutor robots: `https://rutor.info/robots.txt` (архив) — `Crawl-delay: 3`, без `Disallow`.
- RuTracker search: `tracker.php` → `403` Cloudflare (обход исключён).
