# Provider support

Watch queries several third-party catalogs in parallel. Provider search, episode extraction, and stream resolution are separate capabilities: a title can appear in search even when its source does not expose a usable episode list or a directly resolvable player.

## Enabled providers

| Provider | Catalog category | Current behavior and limits |
| --- | --- | --- |
| HiAnime | English anime | Anime search, episodes, and the dedicated stream resolver. |
| Gogoanime | English anime | Anime search, generic episode extraction, and shared HLS/MP4 resolution. |
| AniWatch | English anime | Anime search, generic episode extraction, and shared HLS/MP4 resolution. |
| TioAnime | Spanish anime | Anime search, episode extraction including its numeric episode array, and shared HLS/MP4 resolution. |
| JKAnime | Spanish anime | Anime search and shared episode/stream parsing. |
| Gnula | Spanish movies and series | Movie/series search. LAT identifies Latin-American Spanish audio (es-419); CAS identifies Castilian Spanish audio (es-ES). SUB alone does not identify the subtitle language. A movie detail page is exposed as one playable entry. |
| PelisPedia | Spanish movies and series | Search uses /peliculas and /series; detail pages use /pelicula/... and /serie/.... Movie pages are exposed as one entry. Series episode extraction depends on the page exposing recognizable episode links. |
| PelisPlus | Spanish movies and series | Search uses /inicio/, /nuevas-peliculas/, and /ultimas-series/. Movie details use /peliculas/...; series details and episode links use /serie/.... Episode numbers encode season and episode. Some Watch now pages may require registration; Watch does not bypass registration or other access controls. |
| HDToday | English movie and TV catalog | Movie details use /movie/... and link to /watch-movie/.... TV details use /tv/...; the inspected pages did not expose a usable episode list, so TV titles may be searchable without being playable. |
| Pelis7 | Spanish movies and series | The pelis7.com host could not be independently verified during implementation. The adapter uses fallback search and common detail-path patterns; live search, episode, and playback support remain unverified. |

Provider markup, hostnames, search routes, and player embeds can change without notice. A source being enabled in the catalog is not a guarantee that its website is currently reachable or that every title has a playable stream.

## Audio and subtitle language pairs

A result label uses the form audio/subtitles, for example ja/en or ja/es. Each side is independent. ? means Watch does not have reliable information for that field.

The existing anime providers currently have configured defaults based on their provider category. These defaults are useful catalog labels, but they are not a live probe of the exact tracks selected from the player. Gnula exposes explicit LAT and CAS audio labels; its generic SUB badge does not identify a subtitle language. The newer movie/TV providers currently remain ?/? unless a provider-specific parser supplies explicit metadata.

--language continues to filter the catalog category (English or Spanish); it does not select audio or subtitle tracks. --dub requests dubbed playback where the provider supports it.

Improving track-level detection requires reliable metadata from the resolved manifest, embed page, or player track list. The client must not infer a language solely from a generic SUB badge, the catalog category, or the presence of a subtitle file.

## Resolver and access boundaries

Non-HiAnime sources currently share generic HTML, iframe, HLS, and direct MP4 extraction, with small provider-specific additions. Custom JavaScript players, encrypted embeds, expiring tokens, and unsupported media hosts may need dedicated resolver work.

Watch does not bypass anti-bot challenges, account registration, paywalls, DRM, or other access controls. When a page requires one of these, resolution should fail or leave the source unavailable rather than attempting to circumvent the restriction.

## Regression tests

Provider parser tests use local HTML fixtures and do not depend on live websites. These tests protect known route and markup behavior, but they cannot guarantee that a third-party site has not changed since the fixture was recorded.
