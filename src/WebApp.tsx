import { type FormEvent, useEffect, useState } from 'react'
import { type Catalog, type WebProfile, web } from './api'
import { MatchDetail, MatchList } from './MatchesView'
import { ProfileView } from './ProfileView'

const PLATFORMS: [id: string, label: string][] = [
  ['la2', 'LAS'],
  ['la1', 'LAN'],
  ['br1', 'BR'],
  ['na1', 'NA'],
  ['euw1', 'EUW'],
  ['eun1', 'EUNE'],
  ['tr1', 'TR'],
  ['ru', 'RU'],
  ['me1', 'ME'],
  ['kr', 'KR'],
  ['jp1', 'JP'],
  ['oc1', 'OCE'],
  ['sg2', 'SG'],
  ['tw2', 'TW'],
  ['vn2', 'VN'],
]

function errorText(error: unknown) {
  return typeof error === 'string' ? error : 'Ocurrió un error inesperado.'
}

export function WebApp() {
  const [catalog, setCatalog] = useState<Catalog | null>(null)
  const [riotId, setRiotId] = useState('')
  const [platform, setPlatform] = useState('la2')
  const [data, setData] = useState<WebProfile | null>(null)
  const [view, setView] = useState<'profile' | 'matches'>('profile')
  const [openGame, setOpenGame] = useState<number | null>(null)
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState<string | null>(null)

  useEffect(() => {
    web.catalog().then(setCatalog).catch((e) => setError(errorText(e)))
  }, [])

  async function search(event: FormEvent) {
    event.preventDefault()
    const [name, tag] = riotId.split('#').map((part) => part.trim())
    if (!name || !tag) {
      setError('Escribí el Riot ID completo, por ejemplo Nombre#TAG.')
      return
    }
    setBusy(true)
    setError(null)
    try {
      setData(await web.profile(platform, name, tag))
      setView('profile')
      setOpenGame(null)
    } catch (e) {
      setError(errorText(e))
    } finally {
      setBusy(false)
    }
  }

  return (
    <div className="flex min-h-full flex-col">
      <header className="flex flex-wrap items-center gap-x-6 gap-y-3 border-b border-line bg-ink px-5 py-3">
        <span className="font-display text-2xl font-black tracking-tight">
          LOLUP<span className="text-signal">.</span>
        </span>
        {data && (
          <nav className="flex gap-1" aria-label="Secciones">
            {(['profile', 'matches'] as const).map((id) => (
              <button
                key={id}
                onClick={() => {
                  setView(id)
                  setOpenGame(null)
                }}
                aria-pressed={view === id}
                className={`px-3 py-1 font-display text-lg font-semibold ${view === id ? 'bg-paper text-ink' : 'text-muted hover:text-paper'}`}
              >
                {id === 'profile' ? 'Perfil' : 'Partidas'}
              </button>
            ))}
          </nav>
        )}
        <form onSubmit={search} className="ml-auto flex gap-2" role="search">
          <label className="sr-only" htmlFor="platform">Región</label>
          <select
            id="platform"
            value={platform}
            onChange={(e) => setPlatform(e.target.value)}
            className="border border-line bg-panel px-2 py-1.5 text-sm"
          >
            {PLATFORMS.map(([id, label]) => (
              <option key={id} value={id}>{label}</option>
            ))}
          </select>
          <label className="sr-only" htmlFor="riot-id">Riot ID</label>
          <input
            id="riot-id"
            value={riotId}
            onChange={(e) => setRiotId(e.target.value)}
            placeholder="Nombre#TAG"
            autoComplete="off"
            className="w-56 border border-line bg-panel px-3 py-1.5 text-sm placeholder:text-muted"
          />
          <button disabled={busy} className="bg-signal px-4 py-1.5 font-display text-lg font-bold text-ink disabled:opacity-50">
            {busy ? 'Buscando…' : 'Buscar'}
          </button>
        </form>
      </header>

      <main className="flex-1 px-5 py-7 sm:px-8">
        {error && <p role="alert" className="mx-auto mb-5 max-w-5xl border-l-4 border-loss bg-panel px-4 py-3 text-sm">{error}</p>}
        {!data || !catalog ? (
          <section className="rise mx-auto max-w-3xl pt-10">
            <h1 className="font-display text-6xl leading-none font-black uppercase sm:text-7xl">
              Buscá cualquier <span className="text-signal">invocador.</span>
            </h1>
            <p className="mt-4 text-muted">
              Escribí un Riot ID y elegí la región para ver su perfil, rango e historial de partidas.
            </p>
          </section>
        ) : view === 'profile' ? (
          <ProfileView catalog={catalog} profile={data.profile} matches={data.matches} />
        ) : openGame !== null ? (
          <MatchDetail
            catalog={catalog}
            gameId={openGame}
            onBack={() => setOpenGame(null)}
            load={(id) => web.detail(data.platform, data.puuid, id)}
            review={false}
          />
        ) : (
          <MatchList catalog={catalog} rows={data.matches.rows} onOpen={setOpenGame} />
        )}
      </main>

      <footer className="border-t border-line bg-ink px-5 py-3 text-[11px] leading-snug text-muted">
        LolUp isn't endorsed by Riot Games and doesn't reflect the views or opinions of Riot Games or anyone officially
        involved in producing or managing Riot Games properties. Riot Games, and all associated properties are trademarks or
        registered trademarks of Riot Games, Inc.
      </footer>
    </div>
  )
}
