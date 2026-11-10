# LolUp: documento base

"LolUp" es un nombre de trabajo. Fecha de este documento: 2026-11-10.

## 1. Qué construimos

Un compañero de escritorio para League of Legends en Windows, centrado en mejorar entre partidas.

- **Propuesta:** después de cada partida, un análisis con IA que cita tus números y termina en una meta concreta para la próxima.
- **Diferencia frente a iTero:** iTero usa IA para el draft y el macro, y durante la partida muestra timers y trackers. Nosotros competimos en la calidad y la utilidad del análisis, no en la cantidad de datos en pantalla.
- **Negocio previsto:** plan gratuito obligatorio más suscripción de 12 USD al año, presentada como 1 USD por mes.

## 2. Alcance del MVP

Entra:

1. Detectar el cliente de League y conectarse por su API local.
2. Listar tus últimas partidas.
3. Calcular una ficha de hechos de una partida, sin nombres de jugadores.
4. Generar la revisión con IA, en streaming.
5. Generar un plan fijo durante la selección de campeón.
6. Mostrar ese plan en un overlay de solo lectura, que no reacciona a la partida.

No entra hasta tener una razón concreta:

| Función | Se agrega cuando |
|---|---|
| Importar runas e item sets | Riot acepta el uso de esos endpoints de escritura |
| Pagos y plan premium | Riot aprueba el producto y lo que se cobra |
| Backend con key propia de la IA | Antes de la primera beta pública |
| Perfiles de otros jugadores, tier lists | Exista key de producción y una fuente de datos propia |
| Recordatorios configurados por el usuario | Riot lo aprueba (hito 3) |
| Avisos con IA según el estado de la partida | Riot lo aprueba por escrito (hito 3) |
| Línea de tiempo por minuto | La revisión básica demuestre valor |

## 3. Reglas de Riot que condicionan el diseño

Fuentes: [políticas generales](https://developer.riotgames.com/policies/general), [documentación de League](https://developer.riotgames.com/docs/lol), [TFT](https://developer.riotgames.com/docs/tft) y [apps de terceros](https://support.riotgames.com/en-us/riot/events/third-party-applications).

| Regla | Cómo se aplica aquí | Dónde |
|---|---|---|
| Registrar el producto, aun usando APIs no documentadas | Registro antes de cualquier distribución | Hito 1 |
| Plan gratuito obligatorio, contenido pago transformativo | Premium se define fuera de la partida | Hito 4 |
| Sin calculadoras de MMR o ELO | No se calcula ni un puntaje que compare jugadores | `facts.rs` |
| No identificar jugadores ocultos | La ficha no contiene nombres ni identificadores | `facts.rs`, con test |
| Sin acciones por el jugador | El cliente solo admite GET | `lcu.rs` |
| Sin información desconocida para el jugador durante la partida | El MVP no usa la API en vivo del puerto 2999 | Ausencia deliberada |
| No dictar decisiones ni sacar conclusiones en partida | El plan solo se genera en selección de campeón | `plan_allowed`, con test |
| Overlays con datos estáticos previos a la partida | El overlay muestra texto fijo, no recibe clics y no lee la partida | `main.rs` |

Dudas que hay que preguntarle a Riot al registrar:

- Si la API del cliente sigue permitida para este uso. Figura como "no soportada oficialmente".
- Si la API en vivo y la API del cliente quedan fuera del bloqueo de memoria de octubre de 2026. El aviso oficial no las menciona.
- Si un plan generado por IA en selección de campeón y mostrado fijo cuenta como "datos estáticos previos a la partida".
- Si la restricción de 2019 sobre la API del cliente en Corea sigue vigente.

La página específica de League en el centro de soporte para developers devolvió 403 al leerla. Hay que revisarla a mano.

## 4. Stack

| Capa | Elección | Motivo |
|---|---|---|
| Núcleo | **Rust** | Tipos fuertes, binario chico, buen manejo de HTTP y streaming |
| Shell | **Tauri 2** (estable) | Núcleo en Rust nativo, ventanas transparentes e ignorar clics incluidos |
| Interfaz | **React 19 + TypeScript** | Es lo que ya usás, y la interfaz no se porta de ningún repo |
| Estilos | **Tailwind 4** | Rápido, sin runtime |
| Estado | `useState` | Zustand entra cuando haya estado compartido real |
| IA | Claude por HTTP desde Rust | No hay SDK oficial de Rust. La key nunca llega a la interfaz |

Alternativas descartadas:

| Opción | Por qué no |
|---|---|
| Electron / ow-electron | Es lo que usa iTero (152 MB en Overwolf). Obliga a Node en el núcleo, y queremos Rust |
| Vue | Sin ventaja propia: su mayor argumento era reciclar la interfaz de rank-analysis, y solo portamos lógica |
| Svelte o Solid | Más livianos, pero la interfaz es chica y el costo de cambiar de ecosistema no se paga |
| Rust nativo (egui, Iced, Slint, GPUI) | Excelente para un overlay simple, mucho más lento para lograr una interfaz de producto cuidada |
| WinUI / WPF con .NET | Solo Windows y sin Rust. Impide desarrollar desde macOS |
| Tauri 3 | Sigue en alfa. Reevaluar cuando salga estable |

Riesgo principal del stack: el overlay depende de WebView2 y hay que probarlo en Windows con League abierto (hito 2). Una ventana sobre el juego no se ve en pantalla completa exclusiva, así que se exige modo sin bordes.

## 5. Referencias: qué tomamos

Todas son MIT. Versiones revisadas entre paréntesis.

| Referencia | Se toma | Se descarta |
|---|---|---|
| [rank-analysis](https://github.com/wnzzer/rank-analysis) (`e75e049353f2`) | Rutas y formato de datos del cliente. Patrón de ficha de hechos, donde cada número va junto a su dueño. Reglas contra datos inventados | `hidden_mmr`, auto aceptar y auto pick/ban, riesgo de lobby, premades, tags de jugadores, datos de OP.GG, nube y actualizador del autor, interfaz Vue |
| [Open League Overlay](https://github.com/bossNilac/Open-League-Overlay) (`2bca5917ce7b`) | Descubrimiento del lockfile en Windows desde `RiotClientInstalls.json`. Referencia de la ventana overlay | Interfaz GDI, recomendaciones desde `mcp-api.op.gg`, oro estimado del rival, comparación en vivo |
| [LeagueAkari](https://github.com/LeagueAkari/LeagueAkari) (`4f8981f84ff8`) | Solo lectura como referencia de ventanas auxiliares | Timers de cooldown, envío de teclas, auto-select, SGP, addons nativos |
| [RiftSense](https://github.com/lowlune/RiftSense) (`ec1fce4d77c2`) | La idea de abstenerse cuando falta un dato | Lectura en vivo cada 60 segundos con instrucción "DO NOW": es el caso de mayor riesgo |

Refactor aplicado: no se copió código. Las piezas se reescribieron en Rust en una versión mínima, porque cada referencia carga con funciones que aquí no pueden existir. Los avisos de licencia están en `THIRD_PARTY_NOTICES.md`.

## 6. Arquitectura

```text
React (main y overlay)
   │ comandos Tauri y canal de streaming
Rust
   ├─ lcu.rs    lockfile, cliente local, lista de endpoints permitidos
   ├─ facts.rs  JSON del cliente → ficha de hechos sin identidades
   └─ ai.rs     prompts, streaming SSE, errores
   │                       │
API local del cliente     API de Claude
(127.0.0.1)               (HTTPS)
```

Decisiones:

- **Lista de endpoints permitidos:** una llamada fuera de la lista falla antes de salir, y es lo primero que hay que mostrar en la auditoría.
- **Lockfile en cada llamada:** el cliente puede reiniciarse y cambia de puerto y token.
- **Sin persistencia:** el historial ya vive en el cliente.
- **Modo demo (`LOLUP_DEMO=1`):** usa datos de ejemplo y permite trabajar sin League.

## 7. IA

- Modelo: `claude-opus-5-5`, constante en `ai.rs`, con esfuerzo `medium`.
- Si el modelo rechaza una solicitud, el servidor pasa a otro modelo (`fallbacks: "default"`, opción beta).
- Clave por ahora en `ANTHROPIC_API_KEY`, solo para desarrollo.
- Antes de la beta pública: un backend propio guarda la key, valida la suscripción y aplica el límite mensual.

Con 12 USD al año quedan unos 0,91 USD por mes después de comisiones. Antes de fijar límites hay que medir el costo real por revisión y probar modelos más baratos con un conjunto de partidas de evaluación.

## 8. Hitos

| Hito | Resultado | Condición de salida |
|---|---|---|
| 0. Prueba de concepto | Este repo | Tests y compilación en verde |
| 1. Validación real | Registro en Riot y prueba con League en Windows | Datos reales fluyen y Riot recibió el detalle de endpoints |
| 2. Overlay | Overlay probado sobre el juego | Legible, no captura clics, sin impacto notable en FPS |
| 3. Respuesta de Riot | Decidir recordatorios y avisos | Respuesta por escrito |
| 4. Beta | Backend, firma de código, plan gratuito y premium | Producto aprobado |
| 5. Lanzamiento | Pagos | 12 USD al año |

Estado al 2026-11-10: conexión, historial y ficha de hechos validados contra el cliente real en macOS. Faltan la sesión real de selección de campeón, la llamada real a Claude, el refresco al terminar una partida y todo lo específico de Windows.

La solicitud para Riot y lo que falta antes de enviarla están en [RIOT_APPLICATION.md](RIOT_APPLICATION.md).

## 10. Equivalencia visual sin IA

Relevado en páginas públicas de OP.GG (perfil web), iTero y Porofessor el 2026-11-10. DPM.LOL bloqueó la lectura, así que falta compararlo a mano.

| Función | La tienen | LolUp | Qué hace falta |
|---|---|---|---|
| Perfil, rango y LP | OP.GG, Porofessor | Hecho | |
| Rendimiento reciente y filtro por cola | OP.GG | Hecho, últimas 20 | |
| Campeones recientes, posiciones, maestría | OP.GG, iTero | Hecho | |
| Historial con objetos, runas y hechizos | OP.GG, DPM (por confirmar) | Hecho | |
| Marcador de 10 jugadores y objetivos | Porofessor, OP.GG | Hecho | |
| Gráfico de ventaja de oro | Porofessor | Hecho | |
| Historial completo de la temporada | OP.GG | Falta | Key de producción de Riot |
| Evolución de LP | OP.GG, Porofessor | Hecho, se acumula desde el 2026-11-10 | Solo registra con LolUp abierto |
| Mapa de actividad por hora y día | OP.GG | Falta | Más historial |
| Minutos clave contra el rival y eventos | iTero, Porofessor | Hecho | El rival solo aparece si la posición es inequívoca |
| Orden de objetos y habilidades | OP.GG (por confirmar) | Falta | La línea de tiempo local no trae esos eventos: key de producción |
| Asesor de draft, counters, builds y tier lists | Todos | Falta | Pipeline propio con key de producción, o licencia de datos |
| Importar runas | Porofessor, iTero | Falta | Respuesta de Riot sobre endpoints de escritura |
| Scouting en pantalla de carga | Porofessor, iTero | No se copia sin OK | Riot restringe identificar jugadores ocultos |
| Timers de jungla, definitivas, oro en vivo | iTero, Porofessor | No se copia sin OK | Riot restringe información en partida |

Los emblemas gráficos de rango no se pudieron obtener por rutas públicas del cliente. El marcador muestra nombres de partidas terminadas, igual que el cliente; la ficha que va a la IA sigue sin identidades.

## 9. Riesgos

| Riesgo | Plan |
|---|---|
| Riot rechaza o limita la API del cliente | Validar en el hito 1, antes de invertir más |
| El overlay falla con WebView2 | Plan B: el overlay pasa a Rust nativo, sin tocar el resto |
| El costo de la IA supera el margen | Límites mensuales y evaluación de modelos |
| Competidores gratuitos | Competir en calidad del análisis, medida con usuarios reales |
| El formato de datos cambia sin aviso | Fixtures y tests, y errores claros en la interfaz |

## 11. Versión web

Decisión del 2026-11-10: la web tiene que ofrecer lo mismo que la app de escritorio, usable como OP.GG o DPM, para cualquier jugador buscado por su Riot ID.

### Por qué es otro producto técnico

La app de escritorio lee el cliente local y solo ve tu cuenta. La web no puede leer ese cliente: necesita la API oficial de Riot, un servidor que guarde la key y una base de datos para cachear partidas y respetar los límites de uso. Para un sitio público, Riot exige la key de producción.

### Paridad

| Función de escritorio | En la web | Fuente |
|---|---|---|
| Perfil, rango, LP actual | Sí, para cualquier Riot ID | Account-V1, Summoner-V4, League-V4 |
| Maestría | Sí | Champion-Mastery-V4 |
| Historial con filtros por cola | Sí, con más partidas que las 20 del cliente | Match-V5 |
| Marcador de 10 jugadores | Sí | Match-V5 |
| Ventaja de oro, eventos y minutos clave | Sí, con posiciones fiables (`teamPosition`) | Match-V5 timeline |
| Orden de objetos y habilidades | Sí, nuevo respecto de escritorio | Match-V5 timeline, a confirmar con la key |
| Evolución de LP | Desde la primera vez que alguien busca ese perfil | Instantáneas guardadas en el servidor |
| Revisión con IA | Sí, a través del servidor y con límite mensual | Mismo prompt, sin identidades |
| Plan previo y overlay | No: necesitan el cliente local | Solo escritorio |
| Partidas personalizadas | Solo si el jugador lo autoriza | Regla de Riot |

### Arquitectura propuesta

- **Interfaz:** la misma app de React, con una capa de datos que en escritorio llama a Tauri y en la web llama al servidor. Las vistas no se duplican.
- **Lógica compartida:** un crate de Rust común para los cálculos (resúmenes, minutos clave, eventos, LP), con dos adaptadores de entrada: cliente local y Match-V5.
- **Servidor:** Rust con axum, la key de Riot en variables de entorno, caché en SQLite y reintentos ante los límites de Riot. Las partidas no cambian, así que se guardan una sola vez. Postgres entra cuando haga falta más de una instancia.
- **Hosting:** la interfaz sigue en Vercel. El servidor necesita estado y tareas continuas, así que va mejor en un hosting de contenedores como Fly.io o Railway, junto a Postgres. Falta decidir.

### Restricciones

- Mientras no haya key de producción, la web solo puede usarse en desarrollo con la key personal, que vence cada 24 horas y no se puede exponer al público.
- La política de privacidad tiene que cubrir los datos públicos de jugadores que guarda el servidor antes de abrir la web.
- No se agregan scouting en partida ni datos de partidas en curso.

### Hitos web

| Hito | Resultado |
|---|---|
| W1 | Crate compartido y adaptador de Match-V5 con datos de prueba |
| W2 | Servidor con key personal: búsqueda por Riot ID, perfil e historial en local |
| W3 | Detalle de partida, línea de tiempo y build |
| W4 | Caché, límites de uso e instantáneas de LP |
| W5 | Despliegue privado, y público cuando Riot apruebe la key de producción |

Estado al 2026-11-10: W1 a W4 funcionan en local con la key personal. Falta una cola por región para los límites de uso, y W5.