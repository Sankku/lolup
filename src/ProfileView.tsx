import { useState } from 'react'
import { type Catalog, type LpPoint, type Matches, type Profile, type Rank, assetUrl } from './api'
import { Bar, GameIcon, ROLES, es, percent, queueName } from './ui'

const TIERS: Record<string, string> = {
  IRON: 'Hierro',
  BRONZE: 'Bronce',
  SILVER: 'Plata',
  GOLD: 'Oro',
  PLATINUM: 'Platino',
  EMERALD: 'Esmeralda',
  DIAMOND: 'Diamante',
  MASTER: 'Maestro',
  GRANDMASTER: 'Gran Maestro',
  CHALLENGER: 'Retador',
}

const LP = { width: 1000, height: 170, left: 58, right: 16, top: 12, bottom: 24 }
const dateFormat = new Intl.DateTimeFormat('es', { day: 'numeric', month: 'short' })
const fullFormat = new Intl.DateTimeFormat('es', { day: 'numeric', month: 'short', hour: '2-digit', minute: '2-digit' })
const rankLabel = (p: LpPoint) => `${TIERS[p.tier] ?? p.tier} ${p.division} ${p.lp} LP`.replace('  ', ' ')

function LpChart(props: { points: LpPoint[] }) {
  const { points } = props
  const [hover, setHover] = useState<number | null>(null)
  const heading = <h2 className="text-xs text-muted">Evolución de LP · Clasificatoria individual</h2>

  if (points.length < 2) {
    return (
      <section className="rise border border-line bg-ink p-4" style={{ animationDelay: '210ms' }}>
        {heading}
        <p className="mt-2 text-sm">
          Empezamos a registrar tu LP. El gráfico aparece después de tu próxima partida clasificatoria con LolUp abierto.
        </p>
      </section>
    )
  }

  const values = points.map((p) => p.ladder)
  const low = Math.min(...values) - 15
  const high = Math.max(...values) + 15
  const first = points[0].ms
  const span = Math.max(points[points.length - 1].ms - first, 1)
  const innerWidth = LP.width - LP.left - LP.right
  const innerHeight = LP.height - LP.top - LP.bottom
  const x = (ms: number) => LP.left + ((ms - first) / span) * innerWidth
  const y = (ladder: number) => LP.top + ((high - ladder) / (high - low)) * innerHeight
  const line = points.map((p, i) => `${i ? 'L' : 'M'}${x(p.ms)},${y(p.ladder)}`).join(' ')
  const active = points[hover ?? points.length - 1]

  function move(event: React.MouseEvent<SVGSVGElement>) {
    const box = event.currentTarget.getBoundingClientRect()
    const ms = first + (((event.clientX - box.left) / box.width) * LP.width - LP.left) / innerWidth * span
    let nearest = 0
    points.forEach((p, i) => {
      if (Math.abs(p.ms - ms) < Math.abs(points[nearest].ms - ms)) nearest = i
    })
    setHover(nearest)
  }

  return (
    <section className="rise border border-line bg-ink p-4" style={{ animationDelay: '210ms' }}>
      <div className="flex items-baseline justify-between">
        {heading}
        <p className="text-sm tabular-nums">{fullFormat.format(active.ms)} · {rankLabel(active)}</p>
      </div>
      <svg
        viewBox={`0 0 ${LP.width} ${LP.height}`}
        className="mt-2 w-full"
        role="img"
        aria-label={`Evolución de LP en ${points.length} registros. Último valor: ${rankLabel(points[points.length - 1])}.`}
        onMouseMove={move}
        onMouseLeave={() => setHover(null)}
      >
        <g stroke="var(--color-line)">
          <line x1={LP.left} x2={LP.width - LP.right} y1={LP.top} y2={LP.top} />
          <line x1={LP.left} x2={LP.width - LP.right} y1={LP.height - LP.bottom} y2={LP.height - LP.bottom} />
        </g>
        <path d={line} fill="none" stroke="var(--color-paper)" strokeWidth="2" />
        {hover !== null && (
          <line x1={x(active.ms)} x2={x(active.ms)} y1={LP.top} y2={LP.height - LP.bottom} stroke="var(--color-muted)" />
        )}
        <circle cx={x(active.ms)} cy={y(active.ladder)} r="5" fill="var(--color-signal)" stroke="var(--color-ink)" strokeWidth="2" />
        <g fill="var(--color-muted)" fontSize="13">
          <text x={LP.left} y={LP.height - 5}>{dateFormat.format(first)}</text>
          <text x={LP.width - LP.right} y={LP.height - 5} textAnchor="end">{dateFormat.format(points[points.length - 1].ms)}</text>
        </g>
      </svg>
      <p className="mt-1 text-xs text-muted">Registrado localmente en este equipo cada vez que cambia tu LP con LolUp abierto.</p>
    </section>
  )
}

function RankCard(props: { title: string; rank: Rank | null }) {
  const { rank } = props
  return (
    <section className="flex-1 border border-line bg-ink p-4">
      <h2 className="text-xs text-muted">{props.title}</h2>
      {rank ? (
        <>
          <p className="font-display text-3xl font-black uppercase">
            {TIERS[rank.tier] ?? rank.tier} {rank.division}
          </p>
          <p className="text-sm">
            {rank.lp} LP · {rank.wins}V {rank.losses}D ·{' '}
            <span className="font-bold">{percent(rank.wins, rank.wins + rank.losses)}%</span>
          </p>
        </>
      ) : (
        <p className="mt-1 font-display text-3xl font-black text-muted uppercase">Sin clasificar</p>
      )}
    </section>
  )
}

export function ProfileView(props: { catalog: Catalog; profile: Profile; matches: Matches }) {
  const { catalog, profile, matches } = props
  const [queue, setQueue] = useState<number | null>(null)
  const summary = matches.summaries.find((s) => s.queueId === queue) ?? matches.summaries[0]
  const losses = summary.games - summary.wins
  const winRate = percent(summary.wins, summary.games)
  const maxRole = Math.max(1, ...summary.roles.map((role) => role.games))

  return (
    <article className="mx-auto max-w-5xl space-y-5">
      <header className="rise flex items-center gap-5">
        <img
          src={assetUrl(`/lol-game-data/assets/v1/profile-icons/${profile.iconId}.jpg`)}
          alt=""
          width={84}
          height={84}
          className="rounded-sm bg-line"
        />
        <div className="min-w-0">
          <h1 className="truncate font-display text-6xl leading-none font-black uppercase">{profile.name}</h1>
          <p className="mt-1 text-muted">#{profile.tag} · Nivel {profile.level}</p>
        </div>
      </header>

      <div className="rise flex gap-3" style={{ animationDelay: '60ms' }}>
        <RankCard title="Clasificatoria individual" rank={profile.solo} />
        <RankCard title="Clasificatoria flexible" rank={profile.flex} />
      </div>

      <nav className="rise flex flex-wrap gap-1" aria-label="Filtrar por cola" style={{ animationDelay: '120ms' }}>
        {matches.summaries.map((item) => (
          <button
            key={item.queueId ?? 'all'}
            onClick={() => setQueue(item.queueId)}
            aria-pressed={item.queueId === summary.queueId}
            className={`border border-line px-3 py-1 text-sm ${
              item.queueId === summary.queueId ? 'bg-paper text-ink' : 'text-muted hover:text-paper'
            }`}
          >
            {item.queueId === null ? 'Todas' : queueName(catalog, item.queueId)} ({item.games})
          </button>
        ))}
      </nav>

      <div className="rise grid grid-cols-3 gap-3" style={{ animationDelay: '180ms' }}>
        <section className="border border-line bg-ink p-4">
          <h2 className="text-xs text-muted">Últimas {summary.games} partidas</h2>
          <p className="font-display text-6xl font-black tabular-nums">{winRate}%</p>
          <p className="text-sm">{summary.wins} victorias · {losses} derrotas</p>
          <div className="mt-3 flex h-2 gap-0.5" aria-hidden="true">
            <div className="rounded-l-full bg-signal" style={{ flexGrow: summary.wins }} />
            <div className="rounded-r-full bg-loss" style={{ flexGrow: losses }} />
          </div>
          <dl className="mt-4 flex justify-between">
            <div>
              <dt className="text-xs text-muted">KDA</dt>
              <dd className="font-display text-3xl font-extrabold tabular-nums">{es(summary.kda)}</dd>
            </div>
            <div className="text-right">
              <dt className="text-xs text-muted">Promedio K / M / A</dt>
              <dd className="font-display text-3xl font-extrabold tabular-nums">
                {es(summary.kills)} / {es(summary.deaths)} / {es(summary.assists)}
              </dd>
            </div>
          </dl>
        </section>

        <section className="border border-line bg-ink p-4">
          <h2 className="text-xs text-muted">Campeones recientes</h2>
          <ul className="mt-2 space-y-2">
            {summary.champions.slice(0, 5).map((champion) => (
              <li key={champion.championId} className="flex items-center gap-2 text-sm">
                <GameIcon entry={catalog.champions[champion.championId]} size={32} />
                <span className="min-w-0 flex-1 truncate font-display text-lg font-bold">{champion.champion}</span>
                <span className="tabular-nums">{percent(champion.wins, champion.games)}%</span>
                <span className="w-12 text-right text-muted tabular-nums">{champion.games} PJ</span>
                <span className="w-12 text-right tabular-nums">{es(champion.kda)}</span>
              </li>
            ))}
          </ul>
        </section>

        <section className="border border-line bg-ink p-4">
          <h2 className="text-xs text-muted">Posiciones</h2>
          <ul className="mt-2 space-y-3">
            {summary.roles.map((role) => (
              <li key={role.role} className="text-sm">
                <div className="mb-1 flex justify-between">
                  <span>{ROLES[role.role]}</span>
                  <span className="text-muted tabular-nums">
                    {role.games} PJ · {percent(role.wins, role.games)}%
                  </span>
                </div>
                <Bar value={role.games} max={maxRole} />
              </li>
            ))}
          </ul>
        </section>
      </div>

      <LpChart points={profile.lpHistory} />

      <section className="rise" style={{ animationDelay: '240ms' }}>
        <h2 className="mb-2 text-xs text-muted">Mayor maestría</h2>
        <ul className="grid grid-cols-6 gap-3">
          {profile.mastery.map((item) => (
            <li key={item.championId} className="border border-line bg-ink p-3 text-center">
              <div className="flex justify-center">
                <GameIcon entry={catalog.champions[item.championId]} size={52} />
              </div>
              <p className="mt-1 truncate font-display text-lg font-bold">{item.champion}</p>
              <p className="text-xs text-muted tabular-nums">Nv. {item.level} · {es(item.points)}</p>
            </li>
          ))}
        </ul>
      </section>
    </article>
  )
}
