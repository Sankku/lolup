import { useEffect, useRef, useState } from 'react'
import { type Catalog, type Detail, type GameEvent, type GoldPoint, type MatchRow, type Team, getDetail, streamReview } from './api'
import { Bar, GameIcon, Items, Loadout, ROLES, es, kdaOf, queueName } from './ui'

const NEXT_GOAL = 'Próxima partida:'
const relative = new Intl.RelativeTimeFormat('es', { numeric: 'auto' })

function ago(ms: number) {
  const hours = Math.round((ms - Date.now()) / 3_600_000)
  return Math.abs(hours) < 24 ? relative.format(hours, 'hour') : relative.format(Math.round(hours / 24), 'day')
}

function result(row: { win: boolean; remake?: boolean }) {
  if (row.remake) return { label: 'Remake', text: 'text-muted', bar: 'bg-muted' }
  return row.win
    ? { label: 'Victoria', text: 'text-signal', bar: 'bg-signal' }
    : { label: 'Derrota', text: 'text-loss', bar: 'bg-loss' }
}

export function MatchList(props: { catalog: Catalog; rows: MatchRow[]; onOpen: (id: number) => void }) {
  const { catalog } = props
  return (
    <div className="mx-auto max-w-5xl">
      <h1 className="rise mb-4 font-display text-5xl font-black uppercase">Partidas</h1>
      <ul className="space-y-1.5">
        {props.rows.map((row) => {
          const outcome = result(row)
          return (
            <li key={row.gameId}>
              <button
                onClick={() => props.onOpen(row.gameId)}
                className="flex w-full items-center gap-4 border border-line bg-ink py-2 pr-4 text-left hover:bg-panel"
              >
                <span className={`h-14 w-1 shrink-0 ${outcome.bar}`} aria-hidden="true" />
                <span className="w-44 shrink-0 text-xs text-muted">
                  <span className={`block font-display text-lg font-bold ${outcome.text}`}>{outcome.label}</span>
                  <span className="block truncate">{queueName(catalog, row.queueId)}</span>
                  <span className="block">{es(row.minutes)} min · {ago(row.createdMs)}</span>
                </span>
                <GameIcon entry={catalog.champions[row.championId]} size={48} />
                <Loadout catalog={catalog} spells={row.spells} keystone={row.keystone} subStyle={row.subStyle} />
                <span className="w-24 shrink-0">
                  <span className="block font-display text-xl font-bold tabular-nums">
                    {row.kills}/{row.deaths}/{row.assists}
                  </span>
                  <span className="block text-xs text-muted tabular-nums">
                    {es(kdaOf(row.kills, row.deaths, row.assists))} KDA
                  </span>
                </span>
                <span className="w-24 shrink-0 text-xs text-muted tabular-nums">
                  <span className="block">{row.cs} farmeo</span>
                  <span className="block">{es(row.minutes > 0 ? Math.round((row.cs * 10) / row.minutes) / 10 : 0)} por min</span>
                  <span className="block">{row.vision} visión</span>
                </span>
                <span className="ml-auto">
                  <Items catalog={catalog} items={row.items} />
                </span>
              </button>
            </li>
          )
        })}
      </ul>
    </div>
  )
}

const CHART = { width: 1000, height: 190, left: 54, right: 16, top: 12, bottom: 26 }
const compact = (value: number) => `${value > 0 ? '+' : ''}${es(Math.round(value / 100) / 10)}k`

const DRAGONS: Record<string, string> = {
  FIRE_DRAGON: 'Infernal',
  EARTH_DRAGON: 'de Montaña',
  AIR_DRAGON: 'de Nube',
  WATER_DRAGON: 'de Océano',
  HEXTECH_DRAGON: 'Hextech',
  CHEMTECH_DRAGON: 'Quimtech',
  ELDER_DRAGON: 'Ancestral',
}

const OBJECTIVES: Record<string, [letter: string, label: string]> = {
  dragon: ['D', 'Dragón'],
  baron: ['B', 'Barón'],
  herald: ['H', 'Heraldo'],
  grubs: ['L', 'Larva'],
  tower: ['T', 'Torre'],
  inhibitor: ['I', 'Inhibidor'],
}

const PERSONAL: Record<string, string> = { kill: 'Asesinato', death: 'Muerte', assist: 'Asistencia' }

function eventLabel(event: GameEvent) {
  if (PERSONAL[event.kind]) return PERSONAL[event.kind]
  const name = OBJECTIVES[event.kind]?.[1] ?? event.kind
  const dragon = event.kind === 'dragon' && DRAGONS[event.detail] ? ` ${DRAGONS[event.detail]}` : ''
  return `${name}${dragon} · ${event.mine ? 'tu equipo' : 'rival'}`
}

function GoldChart(props: { points: GoldPoint[]; events: GameEvent[] }) {
  const { points, events } = props
  const [hover, setHover] = useState<number | null>(null)
  if (points.length < 2) return null

  const last = points[points.length - 1].minute
  const peak = Math.max(1000, ...points.map((p) => Math.abs(p.lead)))
  const innerWidth = CHART.width - CHART.left - CHART.right
  const innerHeight = CHART.height - CHART.top - CHART.bottom
  const x = (minute: number) => CHART.left + (minute / Math.max(last, 1)) * innerWidth
  const y = (lead: number) => CHART.top + ((peak - lead) / (2 * peak)) * innerHeight
  const zero = y(0)
  const line = points.map((p, i) => `${i ? 'L' : 'M'}${x(p.minute)},${y(p.lead)}`).join(' ')
  const area = `${line} L${x(last)},${zero} L${x(points[0].minute)},${zero} Z`
  const ticks = points.filter((p) => p.minute % 5 === 0)
  const active = hover === null ? null : points[hover]

  function move(event: React.MouseEvent<SVGSVGElement>) {
    const box = event.currentTarget.getBoundingClientRect()
    const minute = (((event.clientX - box.left) / box.width) * CHART.width - CHART.left) / innerWidth * last
    let nearest = 0
    points.forEach((p, i) => {
      if (Math.abs(p.minute - minute) < Math.abs(points[nearest].minute - minute)) nearest = i
    })
    setHover(nearest)
  }

  return (
    <section className="rise border border-line bg-ink p-4" style={{ animationDelay: '60ms' }}>
      <div className="flex items-baseline justify-between">
        <h2 className="text-xs text-muted">Ventaja de oro de tu equipo</h2>
        <p className="text-sm tabular-nums" aria-live="off">
          {active ? `Minuto ${es(active.minute)}: ${compact(active.lead)}` : `Final: ${compact(points[points.length - 1].lead)}`}
        </p>
      </div>
      <svg
        viewBox={`0 0 ${CHART.width} ${CHART.height}`}
        className="mt-2 w-full"
        role="img"
        aria-label={`Ventaja de oro por minuto. Valor final ${compact(points[points.length - 1].lead)}.`}
        onMouseMove={move}
        onMouseLeave={() => setHover(null)}
      >
        <defs>
          <clipPath id="above"><rect x="0" y="0" width={CHART.width} height={zero} /></clipPath>
          <clipPath id="below"><rect x="0" y={zero} width={CHART.width} height={CHART.height} /></clipPath>
        </defs>
        <line x1={CHART.left} x2={CHART.width - CHART.right} y1={zero} y2={zero} stroke="var(--color-muted)" strokeWidth="1" />
        <path d={area} clipPath="url(#above)" fill="var(--color-signal)" opacity="0.28" />
        <path d={area} clipPath="url(#below)" fill="var(--color-loss)" opacity="0.28" />
        <path d={line} clipPath="url(#above)" fill="none" stroke="var(--color-signal)" strokeWidth="2" />
        <path d={line} clipPath="url(#below)" fill="none" stroke="var(--color-loss)" strokeWidth="2" />
        <g fill="var(--color-muted)" fontSize="13">
          <text x={CHART.left - 8} y={CHART.top + 10} textAnchor="end">{compact(peak)}</text>
          <text x={CHART.left - 8} y={zero + 4} textAnchor="end">0</text>
          <text x={CHART.left - 8} y={CHART.height - CHART.bottom} textAnchor="end">{compact(-peak)}</text>
          {ticks.map((p) => (
            <text key={p.minute} x={x(p.minute)} y={CHART.height - 6} textAnchor="middle">{p.minute}</text>
          ))}
        </g>
        {active && (
          <>
            <line x1={x(active.minute)} x2={x(active.minute)} y1={CHART.top} y2={CHART.height - CHART.bottom} stroke="var(--color-muted)" strokeWidth="1" />
            <circle cx={x(active.minute)} cy={y(active.lead)} r="5" fill="var(--color-paper)" stroke="var(--color-ink)" strokeWidth="2" />
          </>
        )}
      </svg>
      {events.length > 0 && (
        <>
          <svg viewBox={`0 0 ${CHART.width} 96`} className="w-full" role="img" aria-label="Eventos de la partida por minuto. La lista completa está debajo.">
            <g fill="var(--color-muted)" fontSize="13" textAnchor="end">
              <text x={CHART.left - 8} y="20">Vos</text>
              <text x={CHART.left - 8} y="52">Equipo</text>
              <text x={CHART.left - 8} y="84">Rival</text>
            </g>
            {[16, 48, 80].map((row) => (
              <line key={row} x1={CHART.left} x2={CHART.width - CHART.right} y1={row} y2={row} stroke="var(--color-line)" />
            ))}
            {events.map((event, index) => {
              const cx = x(event.minute)
              const title = <title>{`Minuto ${es(event.minute)}: ${eventLabel(event)}`}</title>
              if (event.kind === 'kill') return <circle key={index} cx={cx} cy="16" r="6" fill="var(--color-signal)">{title}</circle>
              if (event.kind === 'assist') return <circle key={index} cx={cx} cy="16" r="4" fill="var(--color-muted)">{title}</circle>
              if (event.kind === 'death') {
                return (
                  <path key={index} d={`M${cx - 6},10 L${cx + 6},22 M${cx + 6},10 L${cx - 6},22`} stroke="var(--color-loss)" strokeWidth="3" strokeLinecap="round">
                    {title}
                  </path>
                )
              }
              const cy = event.mine ? 48 : 80
              return (
                <g key={index}>
                  {title}
                  <circle cx={cx} cy={cy} r="10" fill="var(--color-panel)" stroke={event.mine ? 'var(--color-signal)' : 'var(--color-loss)'} strokeWidth="1.5" />
                  <text x={cx} y={cy + 4} fill="var(--color-paper)" fontSize="12" textAnchor="middle" fontWeight="700">{OBJECTIVES[event.kind]?.[0]}</text>
                </g>
              )
            })}
          </svg>
          <p className="mt-2 text-xs text-muted">
            ● asesinato · ✕ muerte · punto gris asistencia · D dragón · B barón · H heraldo · L larva · T torre · I inhibidor
          </p>
          <details className="mt-2 text-sm">
            <summary className="cursor-pointer text-muted hover:text-paper">Ver lista de eventos</summary>
            <ol className="mt-2 grid grid-cols-3 gap-x-6 gap-y-1">
              {events.map((event, index) => (
                <li key={index} className="flex gap-2">
                  <span className="w-10 text-right text-muted tabular-nums">{es(event.minute)}</span>
                  <span>{eventLabel(event)}</span>
                </li>
              ))}
            </ol>
          </details>
        </>
      )}
    </section>
  )
}

function KeyMinutes(props: { detail: Detail }) {
  const { checkpoints, rival } = props.detail
  if (checkpoints.length === 0) return null
  const hasRival = rival !== null && checkpoints.some((c) => c.rival)
  const metric = (mine: number, other?: number) => {
    const diff = other === undefined ? null : mine - other
    return (
      <>
        {es(mine)}
        {diff !== null && (
          <span className={`ml-2 text-xs ${diff >= 0 ? 'text-signal' : 'text-loss'}`}>{diff > 0 ? '+' : ''}{es(diff)}</span>
        )}
      </>
    )
  }

  return (
    <section className="rise border border-line bg-ink" style={{ animationDelay: '90ms' }}>
      <header className="border-b border-line px-4 py-2">
        <h2 className="font-display text-xl font-bold">Minutos clave</h2>
        <p className="text-xs text-muted">{hasRival ? `Comparación con el rival probable de tu línea: ${rival}` : 'No se pudo identificar con certeza a tu rival de línea.'}</p>
      </header>
      <table className="w-full text-sm">
        <thead className="text-left text-xs text-muted">
          <tr>
            <th className="px-4 py-1.5 font-normal">Minuto</th>
            <th className="font-normal">Oro</th>
            <th className="font-normal">Farmeo</th>
            <th className="font-normal">Experiencia</th>
            <th className="pr-4 font-normal">Nivel</th>
          </tr>
        </thead>
        <tbody>
          {checkpoints.map((c) => (
            <tr key={c.minute} className="border-t border-line tabular-nums">
              <td className="px-4 py-2 font-display text-lg font-bold">{c.minute}</td>
              <td>{metric(c.mine.gold, c.rival?.gold)}</td>
              <td>{metric(c.mine.cs, c.rival?.cs)}</td>
              <td>{metric(c.mine.xp, c.rival?.xp)}</td>
              <td className="pr-4">{metric(c.mine.level, c.rival?.level)}</td>
            </tr>
          ))}
        </tbody>
      </table>
    </section>
  )
}

const SKILL_KEYS = ['Q', 'W', 'E', 'R']

/** Orden en que se maximizan Q, W y E: la que llega antes a 5 puntos va primero. */
function skillPriority(skills: number[]) {
  const reachedMax = (slot: number) => {
    let points = 0
    const index = skills.findIndex((s) => s === slot && ++points === 5)
    return index === -1 ? Infinity : index
  }
  const count = (slot: number) => skills.filter((s) => s === slot).length
  return [1, 2, 3]
    .filter((slot) => count(slot) > 0)
    .sort((a, b) => reachedMax(a) - reachedMax(b) || count(b) - count(a) || skills.indexOf(a) - skills.indexOf(b))
    .map((slot) => SKILL_KEYS[slot - 1])
}

function BuildPanel(props: { catalog: Catalog; build: Detail['build']; skills: number[] }) {
  const { catalog, build, skills } = props
  if (build.length === 0 && skills.length === 0) return null

  // Las compras del mismo minuto van juntas, como una vuelta a base.
  const groups: { minute: number; items: number[] }[] = []
  for (const step of build) {
    const minute = Math.floor(step.minute)
    const last = groups[groups.length - 1]
    if (last && last.minute === minute) last.items.push(step.item)
    else groups.push({ minute, items: [step.item] })
  }
  const priority = skillPriority(skills)

  return (
    <section className="rise border border-line bg-ink" style={{ animationDelay: '100ms' }}>
      <header className="border-b border-line px-4 py-2">
        <h2 className="font-display text-xl font-bold">Tu build</h2>
      </header>
      {groups.length > 0 && (
        <ol className="flex flex-wrap items-start gap-x-2 gap-y-3 px-4 py-3" aria-label="Orden de compra">
          {groups.map((group, index) => (
            <li key={index} className="flex items-start gap-2">
              {index > 0 && <span className="pt-1.5 text-muted" aria-hidden="true">›</span>}
              <div>
                <div className="flex gap-0.5">
                  {group.items.map((item, i) => (
                    <GameIcon key={i} entry={catalog.items[item]} size={28} />
                  ))}
                </div>
                <p className="mt-0.5 text-center text-xs text-muted tabular-nums">min {group.minute}</p>
              </div>
            </li>
          ))}
        </ol>
      )}
      {skills.length > 0 && (
        <div className="border-t border-line px-4 py-3">
          {priority.length > 0 && (
            <p className="mb-2 text-sm">
              Prioridad: <span className="font-display text-lg font-bold">{priority.join(' › ')}</span>
            </p>
          )}
          <table className="text-xs tabular-nums" aria-label="Habilidad mejorada en cada nivel">
            <thead className="sr-only">
              <tr>
                <th>Habilidad</th>
                {skills.map((_, level) => <th key={level}>Nivel {level + 1}</th>)}
              </tr>
            </thead>
            <tbody>
              {SKILL_KEYS.map((key, row) => (
                <tr key={key}>
                  <th scope="row" className="w-6 pr-2 text-left font-display text-sm font-bold">{key}</th>
                  {skills.map((slot, level) => (
                    <td key={level} className="p-px">
                      <span
                        className={`flex h-6 w-6 items-center justify-center ${
                          slot === row + 1 ? (key === 'R' ? 'bg-signal text-ink' : 'bg-paper text-ink') : 'bg-panel text-transparent'
                        }`}
                      >
                        {slot === row + 1 ? level + 1 : '·'}
                      </span>
                    </td>
                  ))}
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}
    </section>
  )
}

function TeamTable(props: { catalog: Catalog; team: Team; title: string; maxDamage: number; minutes: number }) {
  const { catalog, team, maxDamage, minutes } = props
  const outcome = result(team)
  const objectives = [
    ['Barones', team.barons],
    ['Dragones', team.dragons],
    ['Heraldos', team.heralds],
    ['Larvas', team.grubs],
    ['Torres', team.towers],
    ['Inhibidores', team.inhibitors],
  ] as const

  return (
    <section className="rise border border-line bg-ink" style={{ animationDelay: '120ms' }}>
      <header className="flex flex-wrap items-center gap-x-5 gap-y-1 border-b border-line px-4 py-2">
        <h2 className="font-display text-xl font-bold">
          {props.title} · <span className={outcome.text}>{outcome.label}</span>
        </h2>
        <p className="text-xs text-muted tabular-nums">{team.kills} asesinatos · {es(team.gold)} oro</p>
        <p className="text-xs text-muted tabular-nums">
          {objectives.filter(([, value]) => value > 0).map(([label, value]) => `${label} ${value}`).join(' · ')}
        </p>
        {team.bans.length > 0 && (
          <div className="ml-auto flex items-center gap-1" aria-label="Campeones bloqueados">
            <span className="mr-1 text-xs text-muted">Bloqueos</span>
            {team.bans.map((id) => <GameIcon key={id} entry={catalog.champions[id]} size={20} />)}
          </div>
        )}
      </header>
      <table className="w-full text-sm">
        <thead className="text-left text-xs text-muted">
          <tr>
            <th className="px-4 py-1.5 font-normal">Jugador</th>
            <th className="font-normal">KDA</th>
            <th className="w-28 font-normal">Daño</th>
            <th className="font-normal">Oro</th>
            <th className="font-normal">Farmeo</th>
            <th className="font-normal">Visión</th>
            <th className="pr-4 font-normal">Objetos</th>
          </tr>
        </thead>
        <tbody>
          {team.players.map((player) => (
            <tr key={player.name + player.championId} className={`border-t border-line ${player.me ? 'bg-panel' : ''}`}>
              <td className="px-4 py-1.5">
                <div className="flex items-center gap-2">
                  <GameIcon entry={catalog.champions[player.championId]} size={36} />
                  <Loadout catalog={catalog} spells={player.spells} keystone={player.keystone} subStyle={player.subStyle} size={17} />
                  <div className="min-w-0 max-w-40">
                    <p className={`truncate ${player.me ? 'font-bold' : ''}`}>{player.name}</p>
                    <p className="text-xs text-muted">
                      {player.champion} · Nv. {player.level}
                      {player.role !== 'UNKNOWN' && ` · ${ROLES[player.role]}`}
                    </p>
                  </div>
                </div>
              </td>
              <td className="tabular-nums">
                {player.kills}/{player.deaths}/{player.assists}
                <span className="block text-xs text-muted">{es(kdaOf(player.kills, player.deaths, player.assists))}</span>
              </td>
              <td className="pr-4 tabular-nums">
                {es(player.damage)}
                <Bar value={player.damage} max={maxDamage} />
              </td>
              <td className="tabular-nums">{es(player.gold)}</td>
              <td className="tabular-nums">
                {player.cs}
                <span className="block text-xs text-muted">{es(minutes > 0 ? Math.round((player.cs * 10) / minutes) / 10 : 0)}/min</span>
              </td>
              <td className="tabular-nums">{player.vision}</td>
              <td className="pr-4"><Items catalog={catalog} items={player.items} size={24} /></td>
            </tr>
          ))}
        </tbody>
      </table>
    </section>
  )
}

function ReviewPanel(props: { gameId: number }) {
  const [text, setText] = useState('')
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const run = useRef(0)

  useEffect(() => {
    run.current++
    setText('')
    setBusy(false)
    setError(null)
  }, [props.gameId])

  async function analyze() {
    const current = ++run.current
    setText('')
    setError(null)
    setBusy(true)
    try {
      await streamReview(props.gameId, (chunk) => {
        if (run.current === current) setText((value) => value + chunk)
      })
    } catch (e) {
      if (run.current === current) setError(typeof e === 'string' ? e : 'Ocurrió un error inesperado.')
    } finally {
      if (run.current === current) setBusy(false)
    }
  }

  const lines = text.split('\n')
  const goal = lines.find((line) => line.startsWith(NEXT_GOAL))
  const body = lines.filter((line) => line !== goal).join('\n').trim()

  return (
    <section className="rise border border-line bg-ink p-4" style={{ animationDelay: '180ms' }}>
      <div className="flex items-center justify-between">
        <h2 className="font-display text-xl font-bold">Revisión con IA</h2>
        <button onClick={analyze} disabled={busy} className="border border-line px-4 py-1 font-display text-lg font-bold hover:bg-panel disabled:opacity-50">
          {busy ? 'Analizando…' : text ? 'Volver a analizar' : 'Analizar partida'}
        </button>
      </div>
      {error && <p role="alert" className="mt-3 border-l-4 border-loss bg-panel px-4 py-2 text-sm">{error}</p>}
      {(text || busy) && <p aria-live="polite" className={`mt-3 leading-relaxed whitespace-pre-wrap ${busy ? 'caret' : ''}`}>{body}</p>}
      {goal && (
        <aside className="mt-4 bg-signal px-5 py-3 text-ink">
          <p className="font-display text-sm font-bold tracking-[0.18em] uppercase">{NEXT_GOAL.slice(0, -1)}</p>
          <p className="font-display text-2xl font-extrabold">{goal.slice(NEXT_GOAL.length).trim()}</p>
        </aside>
      )}
    </section>
  )
}

export function MatchDetail(props: {
  catalog: Catalog
  gameId: number
  onBack: () => void
  /** La web carga la partida desde su servidor; escritorio, desde el cliente. */
  load?: (gameId: number) => Promise<Detail>
  review?: boolean
}) {
  const { catalog, gameId, load = getDetail, review = true } = props
  const [detail, setDetail] = useState<Detail | null>(null)
  const [error, setError] = useState<string | null>(null)

  useEffect(() => {
    let current = true
    setDetail(null)
    setError(null)
    load(gameId)
      .then((value) => current && setDetail(value))
      .catch((e) => current && setError(typeof e === 'string' ? e : 'No se pudo cargar la partida.'))
    return () => {
      current = false
    }
  }, [gameId])

  const back = (
    <button onClick={props.onBack} className="mb-3 text-sm text-muted hover:text-paper">← Volver a partidas</button>
  )
  if (error) return <div className="mx-auto max-w-5xl">{back}<p role="alert" className="border-l-4 border-loss bg-panel px-4 py-3 text-sm">{error}</p></div>
  if (!detail) return <div className="mx-auto max-w-5xl">{back}<p className="text-muted">Cargando partida…</p></div>

  const myTeam = detail.teams[0]
  const maxDamage = Math.max(...detail.teams.flatMap((team) => team.players.map((p) => p.damage)))
  const outcome = result(myTeam)

  return (
    <article className="mx-auto max-w-5xl space-y-3">
      <div className="rise">
        {back}
        <p className={`font-display text-xl font-bold tracking-[0.18em] uppercase ${outcome.text}`}>
          {outcome.label} · {es(detail.minutes)} min
        </p>
        <h1 className="font-display text-5xl font-black uppercase">{queueName(catalog, detail.queueId)}</h1>
        <p className="text-sm text-muted">{ago(detail.createdMs)} · {myTeam.kills} a {detail.teams[1]?.kills ?? 0} en asesinatos</p>
      </div>
      <GoldChart points={detail.goldLead} events={detail.events} />
      <KeyMinutes detail={detail} />
      <BuildPanel catalog={catalog} build={detail.build} skills={detail.skills} />
      {detail.teams.map((team, index) => (
        <TeamTable key={index} catalog={catalog} team={team} title={index === 0 ? 'Tu equipo' : 'Equipo rival'} maxDamage={maxDamage} minutes={detail.minutes} />
      ))}
      {review && <ReviewPanel gameId={gameId} />}
    </article>
  )
}
