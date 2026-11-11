# LolUp

Prueba de concepto de un compañero de escritorio para League of Legends: revisión post-partida con IA y un plan fijo antes de jugar. El diseño completo está en [docs/BASE.md](docs/BASE.md).

## Requisitos

- Node y pnpm
- Rust estable: <https://rustup.rs>
- En Windows, WebView2 (viene con Windows 11)

## Ejecutar

```bash
pnpm install
```

Con datos de ejemplo, sin League:

```bash
LOLUP_DEMO=1 pnpm tauri dev
```

Con League abierto y revisión real:

```bash
ANTHROPIC_API_KEY=tu_key pnpm tauri dev
```

`Ctrl+Alt+D` muestra u oculta el overlay.

## Web

```bash
pnpm build
RIOT_API_KEY=tu_key cargo run -p lolup-server   # http://127.0.0.1:3000
```

Producción en <https://lolup.aoweb.net>: imagen Docker publicada por CI y deploy automático. Ver [docs/DEPLOY.md](docs/DEPLOY.md).

## Tests

```bash
cargo test --manifest-path src-tauri/Cargo.toml
```

LolUp isn't endorsed by Riot Games and doesn't reflect the views or opinions of Riot Games or anyone officially involved in producing or managing Riot Games properties. Riot Games, and all associated properties are trademarks or registered trademarks of Riot Games, Inc.
