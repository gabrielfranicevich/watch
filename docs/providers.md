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

A result label uses the form audio/subtitles, for example ja/en or ja/es. Each side is independent. ? means Watch does not have reliable information for that field. The shared models retain the evidence source separately from the normalized language value: provider defaults and search-page metadata are recorded for catalog entries; HLS manifests, embed metadata, and HTML track attributes are recorded independently for resolved audio and subtitles.

The existing anime providers currently have configured defaults based on their provider category. These defaults are useful catalog labels, but they are not a live probe of the exact tracks selected from the player. Gnula exposes explicit LAT and CAS audio labels; its generic SUB badge does not identify a subtitle language. For resolved episodes, Watch reads the AUDIO and SUBTITLES renditions declared by an HLS master manifest and exposes the complete rendition list on `ResolvedMedia` for shared frontends. It can also use an explicit language field from the HiAnime embed subtitle metadata or the `srclang` attribute on an HTML track. The CLI prints the default language pair before playback.

The resolved HLS language pair describes the manifest's default tracks. `ResolvedMedia.renditions` also retains declared alternate audio and subtitle renditions, including their language, default flag, and resolved URI when present. Before playback, the CLI offers separate audio and subtitle selection menus when URL-backed alternatives exist, keeping the provider default first. Runtime track switching after playback starts is a separate capability and is not yet implemented. If a manifest or embed page has no explicit language metadata, that field remains ?. The newer movie/TV providers therefore remain ?/? until their media page or manifest exposes track language information.

--language continues to filter the catalog category (English or Spanish); it does not select audio or subtitle tracks. --dub requests dubbed playback where the provider supports it.

`--audio LANGUAGE` and `--subtitles LANGUAGE` are independent source filters. They compare the requested value with the corresponding known field in each source's catalog language pair; when both are supplied, both conditions must match. Language names recognized by Watch normalize to short codes, while region-qualified values such as `es-419` remain distinct. For example:

~~~sh
watch --audio ja "Frieren"
watch --subtitles es-419 "Frieren"
watch --audio Japanese --subtitles Spanish "Frieren"
~~~

A filter excludes sources whose corresponding metadata is unknown. These options filter catalog results only: they do not force a player track, discover additional renditions, or change the active audio/subtitle selection during playback.

The client must not infer a language solely from a generic SUB badge, the catalog category, a display label such as English, or the presence of a subtitle file.

## Resolver and access boundaries

Non-HiAnime sources currently share generic HTML, iframe, HLS, and direct MP4 extraction, with small provider-specific additions. Custom JavaScript players, encrypted embeds, expiring tokens, and unsupported media hosts may need dedicated resolver work.

Watch does not bypass anti-bot challenges, account registration, paywalls, DRM, or other access controls. When a page requires one of these, resolution should fail or leave the source unavailable rather than attempting to circumvent the restriction.

## Regression tests

Provider parser tests use local HTML fixtures and do not depend on live websites. These tests protect known route and markup behavior, but they cannot guarantee that a third-party site has not changed since the fixture was recorded.
