import { useEffect, useRef, useState } from 'react'
import {
  type Catalog,
  type Matches,
  type Profile,
  type Status,
  getCatalog,
  getMatches,
  getProfile,
  getStatus,
  streamPlan,
  toggleOverlay,
} from './api'
import { MatchDetail, MatchList } from './MatchesView'
import { ProfileView } from './ProfileView'

type View = 'profile' | 'matches' | 'plan'
type Data = { catalog: Catalog; profile: Profile; matches: Matches }

const LABELS: [View, string][] = [
  ['profile', 'Perfil'],
  ['matches', 'Partidas'],
  ['plan', 'Plan previo'],
]

function errorText(error: unknown) {
  return typeof error === 'string' ? error : 'Ocurrió un error inesperado.'
}

export function App() {
  const [status, setStatus] = useState<Status | null>(null)
  const [view, setView] = useState<View>('profile')
  const [data, setData] = useState<Data | null>(null)
  const [openGame, setOpenGame] = useState<number | null>(null)
  const [error, setError] = useState<string | null>(null)
  const connected = status?.connected ?? false
  const postGame = status?.phase === 'EndOfGame'

  useEffect(() => {
    const refresh = () => getStatus().then(setStatus).catch(() => {})
    refresh()
    const timer = setInterval(refresh, 3000)
    return () => clearInterval(timer)
  }, [])

  useEffect(() => {
    if (!connected) return
    let current = true
    setError(null)
    Promise.all([getCatalog(), getProfile(), getMatches()])
      .then(([catalog, profile, matches]) => current && setData({ catalog, profile, matches }))
      .catch((e) => current && setError(errorText(e)))
    return () => {
      current = false
    }
  }, [connected, postGame])

  function go(next: View) {
    setView(next)
    setOpenGame(null)
  }

  return (
    <div className="flex h-full flex-col">
      <header className="flex items-center gap-6 border-b border-line bg-ink px-5 py-3">
        <span className="font-display text-2xl font-black tracking-tight">
          LOLUP<span className="text-signal">.</span>
        </span>
        <nav className="flex gap-1" aria-label="Secciones">
          {LABELS.map(([id, label]) => (
            <button
              key={id}
              onClick={() => go(id)}
              aria-pressed={view === id}
              className={`px-3 py-1 font-display text-lg font-semibold ${
                view === id ? 'bg-paper text-ink' : 'text-muted hover:text-paper'
              }`}
            >
              {label}
            </button>
          ))}
        </nav>
        <p className="ml-auto text-sm text-muted" role="status">
          <span className={connected ? 'text-signal' : 'text-loss'} aria-hidden="true">● </span>
          {connected ? 'Cliente conectado' : (status?.message ?? 'Buscando el cliente…')}
          {status?.demo && ' · datos de ejemplo'}
        </p>
      </header>

      <main className="min-h-0 flex-1 overflow-y-auto px-8 py-7">
        {error && <p role="alert" className="mx-auto mb-5 max-w-5xl border-l-4 border-loss bg-panel px-4 py-3 text-sm">{error}</p>}
        {!data ? (
          <p className="mx-auto max-w-5xl text-muted">{connected ? 'Cargando tus datos…' : 'Abrí League e iniciá sesión para empezar.'}</p>
        ) : view === 'profile' ? (
          <ProfileView {...data} />
        ) : view === 'matches' && openGame !== null ? (
          <MatchDetail catalog={data.catalog} gameId={openGame} onBack={() => setOpenGame(null)} />
        ) : view === 'matches' ? (
          <MatchList catalog={data.catalog} rows={data.matches.rows} onOpen={setOpenGame} />
        ) : (
          <PlanView allowed={status?.phase === 'ChampSelect'} />
        )}
      </main>

      <footer className="border-t border-line bg-ink px-5 py-1.5 text-[11px] leading-snug text-muted">
        LolUp isn't endorsed by Riot Games and doesn't reflect the views or opinions of Riot Games or anyone officially
        involved in producing or managing Riot Games properties. Riot Games, and all associated properties are trademarks or
        registered trademarks of Riot Games, Inc.
      </footer>
    </div>
  )
}

function PlanView(props: { allowed: boolean }) {
  const [plan, setPlan] = useState('')
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const run = useRef(0)

  async function generate() {
    const current = ++run.current
    setPlan('')
    setError(null)
    setBusy(true)
    try {
      await streamPlan((text) => {
        if (run.current === current) setPlan((value) => value + text)
      })
    } catch (e) {
      if (run.current === current) setError(errorText(e))
    } finally {
      if (run.current === current) setBusy(false)
    }
  }

  return (
    <article className="rise mx-auto max-w-xl">
      <h1 className="font-display text-6xl leading-none font-black uppercase">Plan previo</h1>
      <p className="mt-3 text-muted">
        Se genera una vez durante la selección de campeón y queda fijo. No cambia con lo que pasa en la partida.
      </p>
      {error && <p role="alert" className="mt-4 border-l-4 border-loss bg-panel px-4 py-3 text-sm">{error}</p>}
      <div className="mt-6 flex flex-wrap gap-3">
        <button
          onClick={generate}
          disabled={busy || !props.allowed}
          className="bg-signal px-5 py-2 font-display text-xl font-bold text-ink hover:brightness-110 disabled:opacity-50"
        >
          {busy ? 'Generando…' : 'Generar plan'}
        </button>
        <button onClick={() => toggleOverlay()} className="border border-line px-5 py-2 font-display text-xl font-bold hover:bg-panel">
          Mostrar u ocultar overlay
        </button>
      </div>
      {!props.allowed && <p className="mt-3 text-sm text-muted">Disponible durante la selección de campeón.</p>}
      {plan && (
        <p aria-live="polite" className={`mt-6 border-l-4 border-signal bg-panel px-5 py-4 leading-relaxed whitespace-pre-wrap ${busy ? 'caret' : ''}`}>
          {plan}
        </p>
      )}
      <p className="mt-6 text-sm text-muted">Atajo del overlay: Ctrl+Alt+D</p>
    </article>
  )
}
