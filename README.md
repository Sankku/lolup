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

## Tests

```bash
cargo test --manifest-path src-tauri/Cargo.toml
```

LolUp isn't endorsed by Riot Games and doesn't reflect the views or opinions of Riot Games or anyone officially involved in producing or managing Riot Games properties. Riot Games, and all associated properties are trademarks or registered trademarks of Riot Games, Inc.
