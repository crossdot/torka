# SECURITY.md

## Threat model
Недоверенные данные: запросы, HTML провайдеров, magnet URI, torrent metadata, media files и сетевые соединения.

## Обязательные требования
- Не принимать произвольные URLs, filesystem paths или FFmpeg args от клиента.
- Валидация magnet URI и лимиты полей.
- SSRF protection: запрет localhost/link-local/private/internal targets и опасных схем для server-side fetch; redirects валидировать повторно.
- Ограничивать размер response, время обработки и concurrency.
- Rate limit и max active sessions.
- Bearer token или иной явный auth; сетевой периметр не единственная защита.
- Секреты вне git; redaction cookies/tokens/magnets.
- Не запускать скачанные бинарники.
- HLS segments доступны только в контексте session.
- Не раскрывать paths, peer IPs, credentials или stack traces.
- Cache, memory, process и CPU limits.
- CORS allowlist, не wildcard с credentials.
- Документировать retention и очистку временных данных.

## Abuse/legal
Не обходить CAPTCHA, paywall, авторизацию, антибот-защиту или иные технические ограничения. Учитывать применимое законодательство и условия источников. Использовать контент, на который есть права/законное основание.
