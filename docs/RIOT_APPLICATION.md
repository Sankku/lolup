# Solicitud a Riot Developer Relations

Borrador al 2026-11-10. El portal no publica la lista exacta de campos, así que cada bloque está pensado para copiarlo donde corresponda al abrir el formulario en <https://developer.riotgames.com/app-type>.

## Antes de enviar

| Pendiente | Por qué | Estado |
|---|---|---|
| Nombre definitivo | "LolUp" es un nombre de trabajo | Falta decidir |
| Dominio propio para `site/` | Riot verifica el dominio con un archivo `riot.txt` | Publicado en lolup.vercel.app, falta dominio |
| Ampliar la privacidad para la web | El servidor guardará datos públicos de jugadores | Falta |
| Prototipo de la web con key personal | Riot querrá ver la búsqueda de perfiles funcionando | Falta |
| Reemplazar el email de contacto en `site/` por uno dedicado | Hoy usa un email personal de forma temporal | Pendiente |
| Revisión legal de la política de privacidad | Es un borrador, y describe el envío de datos a un proveedor de IA | Falta |
| Prototipo o video con el flujo completo | Riot pide algo que pueda probar o entender | Falta |
| Probar la build de Windows con League | Solo se probó el cliente de macOS | Falta |
| Probar la revisión con IA real | Nunca se llamó a la API real | Falta |
| Definir plan gratuito y pago | La propuesta de abajo es un punto de partida | Por confirmar |

Reemplazar todo lo que esté entre corchetes. No declarar nada que todavía no sea cierto.

---

## Product name

[FINAL PRODUCT NAME] (working name: LolUp)

## One-line description

A desktop companion and website that help League of Legends players review matches and track their improvement between games.

## Detailed description

[PRODUCT NAME] has two parts: a desktop application and a website.

The website lets anyone look up a player by Riot ID and see their public profile, rank, champion mastery, match history and match reviews, as on other established statistics sites. It will only be opened to the public after a production key is approved.

The desktop application is for League of Legends players who want to improve between games. After a match, it shows the player a clear review of their own performance: a full scoreboard, the team gold lead over time, key objectives, and how their gold, CS and experience compared with their likely lane opponent at minutes 10, 15 and 20. It also shows recent performance by queue, champion and position, champion mastery, and a local record of the player's LP over time.

The product is in pre-release and has not been distributed publicly. The goal is to help players understand their own games and build better habits, not to play for them or make decisions for them.

Target platform: Windows. The current prototype has also been run against the macOS League client.

## How it helps players

- Post-game review turns match data into concrete areas to work on.
- Key-minute comparisons show where a lane was won or lost.
- LP and recent-performance tracking lets players see their progress over time.
- An optional AI post-game review explains the player's own statistics in plain language and ends with one measurable goal for the next game. It is told to use only the supplied statistics and to say when something cannot be known.

## Data sources

### League Client API

The app only uses read-only `GET` requests. Every request is checked against an allowlist in the code before it is sent. It contains no code path for write requests.

| Endpoint | Use |
|---|---|
| `/lol-summoner/v1/current-summoner` | Identify the signed-in player |
| `/lol-gameflow/v1/gameflow-phase` | Know whether the player is in champion select or post-game |
| `/lol-champ-select/v1/session` | Read visible champions and the player's assigned position |
| `/lol-ranked/v1/current-ranked-stats` | Show the player's current rank and LP |
| `/lol-champion-mastery/v1/local-player/champion-mastery` | Show the player's champion mastery |
| `/lol-match-history/v1/products/lol/{puuid}/matches` | List the signed-in player's recent matches |
| `/lol-match-history/v1/games/{gameId}` | Show the scoreboard of a match from that list |
| `/lol-match-history/v1/game-timelines/{gameId}` | Show gold over time, key minutes and objectives for that match |
| `/lol-game-data/assets/*` | Local static data and icons for champions, items, spells, runes and queues |

The desktop app only requests the signed-in player's own history. It does not use the League Client API to look up other players.

The app does not use the Live Client Data API and does not read game state during a match. It does not read game memory, inject code, hook graphics, send input, or automate any action in the client or game.

### Riot API (requested production key)

Planned use for the website and for longer history in the desktop app, only after approval:

| API | Use |
|---|---|
| Account-V1 | Look up a player by Riot ID |
| Summoner-V4 | Level and profile icon |
| League-V4 | Rank and LP |
| Champion-Mastery-V4 | Champion mastery |
| Match-V5 | Match history, match details and timelines for profile pages; later, aggregated data for champion statistics |

Matches are cached on our server so each one is requested only once, and requests are queued to respect rate limits. Custom games are not shown publicly unless the player opts in, as the policy requires. We will store rank snapshots for profiles that are looked up, to show LP history over time.

The key would stay on our server and would never ship in the desktop app. Riot Sign On is the preferred way to connect a player's account. The planned aggregated pages would use anonymous, aggregated data and would respect all policy limits, including the ban on displaying win rates for Augments or Arena items.

Expected usage: [ESTIMATE ONCE KNOWN]. We are not requesting a rate-limit increase at this stage.

## Player data and privacy

- The product shows Riot IDs. In the desktop app, other players' names are only shown on the scoreboard of a match that has already finished, as the League client does.
- The website shows public profile and match data for a Riot ID that someone searches for, and stores it on our server as a cache.
- In the desktop app, LP history is stored locally on the player's computer.
- For the optional AI review, only the player's own statistics, champion names and match result are sent to our AI provider. Player names, Riot IDs and account identifiers are not included.
- Privacy policy: [URL]

## Game integrity

| Riot policy | How the product complies |
|---|---|
| No alternatives to official ranking | No MMR or ELO estimate and no player skill score. LP is shown as reported by the client |
| No de-anonymization | No champion select scouting and no lookup of hidden players |
| No automation | Read-only client access, no auto-accept, auto-pick or auto-ban |
| No hidden in-game information | No in-game data access, no enemy cooldown or ultimate tracking |
| Should not remove decisions | Review happens after the game and suggests areas of focus |

## Monetization

The product will have a free tier. We plan to offer an optional subscription at about 12 USD per year. Monetization will only begin once the product status allows it.

| Free | Paid (proposed) |
|---|---|
| Profile, rank, LP history | AI post-game review beyond a monthly free allowance |
| Match history and scoreboard | Long-term improvement analysis across many games |
| Gold chart, key minutes and objectives | Cosmetic themes |

There is no betting, gambling, cryptocurrency or advertising inside Riot properties.

## Prototype and website

- Website: [URL]
- Prototype or video of the user flows: [URL]
- Contact: [CONTACT]

## Questions for Developer Relations

1. Is the listed use of the League Client API acceptable for a public, monetized product? Are any listed endpoints not allowed, and does any restriction apply to specific regions such as Korea?
2. The October 2026 notice on game memory access does not mention local APIs. Can you confirm that apps using only the League Client API and the Live Client Data API are not affected?
3. Is an optional AI post-game review of a player's own statistics acceptable as paid, transformative content?
4. We have built a prototype of a pre-game plan: during champion select, AI writes four short lines based only on the visible champions. The text never changes during the game and is shown on a read-only overlay the player turns on. Does this count as static pre-game data, or should we remove it?
5. Would user-configured reminders, such as a personal CS goal shown on that same overlay, be acceptable?
6. May we import rune pages at the player's explicit request using `/lol-perks` endpoints?
7. May we show opponents' public profile data once their names are visible at the loading screen?
8. Are live AI alerts based on the current game state allowed under any conditions? We will not build this without written approval.
9. For website profile pages looked up by Riot ID, do you expect an opt-out mechanism for players, or any other requirement beyond excluding custom games?
10. Is the product name acceptable given that it includes "Lol"? If not, we will rename it before any public release.

## Legal notice

[PRODUCT NAME] isn't endorsed by Riot Games and doesn't reflect the views or opinions of Riot Games or anyone officially involved in producing or managing Riot Games properties. Riot Games, and all associated properties are trademarks or registered trademarks of Riot Games, Inc.
