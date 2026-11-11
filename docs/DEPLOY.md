# Despliegue de la web: lolup.aoweb.net

Estado al 2026-11-10: la web corre en la VM de Lightsail `aura-prod` (São Paulo, `56.124.44.58`,
usuario `ubuntu`), junto a AURA y AURORA, detrás de un túnel de Cloudflare propio. Abierta al público,
con cuentas de LolUp.

## Arquitectura

```
navegador ─► Cloudflare (DNS lolup.aoweb.net) ─► túnel `lolup` ─► cloudflared (contenedor)
                                                                      │ red de compose
                                                                      ▼
                                              lolup-server  http://lolup:3000  (sin puertos publicados)
                                                ├─ interfaz de React (dist/)
                                                ├─ API de Riot (RIOT_API_KEY)
                                                ├─ cuentas: login directo, servidor a servidor
                                                └─ volumen /data/lolup.db (caché, LP, sesiones)
```

- **Imagen:** `Dockerfile` (node compila la interfaz, rust compila `crates/server`, corre en
  `debian:bookworm-slim`). La publica `.github/workflows/web.yml` en `ghcr.io/sankku/lolup`.
- **Deploy continuo:** cada push a `main` publica `:latest` y el Watchtower de AURA (vigila todos los
  contenedores del host) la despliega en ~1 minuto. Los PRs publican `:pr-<número>`.
- **Túnel:** `lolup` (`ebdf7626-ea59-4311-b435-2f9cc85aa2d3`), con credenciales en archivo como el de
  AURORA. El CNAME `lolup.aoweb.net` apunta a ese túnel.

## Cuentas

La pantalla de ingreso y alta es de LolUp. Por debajo, el servidor valida o crea la cuenta en el
servicio de cuentas configurado (`ACCOUNTS_ISSUER`, cliente `lolup` con `aoweb_direct_login: true`)
con `POST /api/directo/entrar|crear`, y guarda solo una sesión: token aleatorio en la cookie
`lolup_sesion` (`HttpOnly`, `Secure`, `SameSite=Lax`, 30 días) y su hash SHA-256 en SQLite. La
contraseña pasa por el servidor y no se guarda ni se registra.

| Ruta                      | Qué hace                                         |
| ------------------------- | ------------------------------------------------ |
| `GET /api/auth/me`        | `{enabled, user}`                                 |
| `POST /api/auth/login`    | `{login, password}` → sesión                      |
| `POST /api/auth/register` | `{name, email, password}` → cuenta nueva y sesión |
| `POST /api/auth/logout`   | cierra la sesión                                  |

Con cuentas configuradas, `/api/profile` y `/api/match` piden sesión (401 sin ella). Sin las variables
`ACCOUNTS_*` la web queda abierta, como en desarrollo local.

## Operación

```bash
ssh ubuntu@56.124.44.58
cd ~/lolup
docker compose ps
docker compose logs --tail 50 lolup
```

**Cargar o cambiar la key de Riot** (la de desarrollo vence cada 24 horas):

```bash
ssh ubuntu@56.124.44.58 "cd ~/lolup && sed -i 's/^RIOT_API_KEY=.*/RIOT_API_KEY=RGAPI-nueva/' .env && docker compose up -d lolup"
```

**Instalar desde cero en otra VM:** Docker, `docker login ghcr.io` con un PAT `read:packages` sin
vencimiento, copiar `deploy/docker-compose.yml`, `deploy/.env.example` como `.env`,
`deploy/cloudflared/config.yml` y las credenciales del túnel a `~/lolup/cloudflared/`, y
`docker compose up -d`. El túnel es el mismo, así que la URL no cambia.

**Backup de la base:**

```bash
docker run --rm -v lolup_lolup-data:/data -v ~/backups:/bkp alpine \
  sh -c 'apk add --no-cache sqlite >/dev/null && sqlite3 /data/lolup.db ".backup /bkp/lolup-$(date +%F).db"'
```

## Pendiente

- La política de Riot no permite exponer al público una key de desarrollo ni personal: el sitio
  público necesita la key de producción (ver [RIOT_APPLICATION.md](RIOT_APPLICATION.md)).
- La política de privacidad tiene que cubrir las cuentas y los datos que guarda el servidor.
