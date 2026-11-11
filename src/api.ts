import { Channel, convertFileSrc, invoke } from '@tauri-apps/api/core'

export type Status = {
  connected: boolean
  phase: string
  demo: boolean
  message: string | null
}

export type Entry = { name: string; icon: string }
type EntryMap = Record<number, Entry | undefined>

export type Catalog = {
  champions: EntryMap
  items: EntryMap
  spells: EntryMap
  runes: EntryMap
  queues: EntryMap
}

export type Rank = { tier: string; division: string; lp: number; wins: number; losses: number }
export type Mastery = { championId: number; champion: string; level: number; points: number }

export type Profile = {
  name: string
  tag: string
  level: number
  iconId: number
  /** Solo en la web: la imagen viene de Data Dragon. */
  iconUrl?: string
  solo: Rank | null
  flex: Rank | null
  mastery: Mastery[]
  lpHistory: LpPoint[]
}

export type LpPoint = { ms: number; tier: string; division: string; lp: number; ladder: number }

export type Loadout = {
  championId: number
  champion: string
  level: number
  items: number[]
  spells: number[]
  keystone: number
  subStyle: number
  kills: number
  deaths: number
  assists: number
  damage: number
  gold: number
  cs: number
  vision: number
  role: string
}

export type MatchRow = Loadout & {
  gameId: number
  win: boolean
  remake: boolean
  minutes: number
  queueId: number
  createdMs: number
}

export type Summary = {
  queueId: number | null
  games: number
  wins: number
  kills: number
  deaths: number
  assists: number
  kda: number
  champions: { championId: number; champion: string; games: number; wins: number; kda: number }[]
  roles: { role: string; games: number; wins: number }[]
}

export type Matches = { rows: MatchRow[]; summaries: Summary[] }
export type Player = Loadout & { name: string; me: boolean }

export type Team = {
  win: boolean
  kills: number
  gold: number
  barons: number
  dragons: number
  heralds: number
  grubs: number
  towers: number
  inhibitors: number
  bans: number[]
  players: Player[]
}

export type GoldPoint = { minute: number; lead: number }
export type FrameStats = { gold: number; cs: number; level: number; xp: number }
export type Checkpoint = { minute: number; mine: FrameStats; rival: FrameStats | null }
export type GameEvent = { minute: number; kind: string; mine: boolean; detail: string }

export type Detail = {
  gameId: number
  minutes: number
  queueId: number
  createdMs: number
  teams: Team[]
  goldLead: GoldPoint[]
  rival: string | null
  checkpoints: Checkpoint[]
  events: GameEvent[]
  build: { minute: number; item: number }[]
  skills: number[]
}

function channel(onText: (text: string) => void) {
  const stream = new Channel<string>()
  stream.onmessage = onText
  return stream
}

export const isDesktop = '__TAURI_INTERNALS__' in window

/** En escritorio las imágenes salen del cliente de League; en la web ya vienen como URL. */
export const assetUrl = (path: string) => (path.startsWith('http') ? path : convertFileSrc(path, 'lcu'))

export const getStatus = () => invoke<Status>('status')
export const getCatalog = () => invoke<Catalog>('catalog')
export const getProfile = () => invoke<Profile>('profile')
export const getMatches = () => invoke<Matches>('matches')
export const getDetail = (gameId: number) => invoke<Detail>('match_detail', { gameId })
export const getPlan = () => invoke<string>('current_plan')
export const toggleOverlay = () => invoke<void>('toggle_overlay')
export const streamReview = (gameId: number, onText: (text: string) => void) =>
  invoke<void>('review', { gameId, onText: channel(onText) })
export const streamPlan = (onText: (text: string) => void) =>
  invoke<void>('generate_plan', { onText: channel(onText) })

export type WebProfile = { puuid: string; platform: string; profile: Profile; matches: Matches }

async function api<T>(path: string, params: Record<string, string>): Promise<T> {
  const response = await fetch(`/api/${path}?${new URLSearchParams(params)}`)
  const body = await response.json().catch(() => ({}))
  if (!response.ok) throw body.error ?? 'El servidor no respondió.'
  return body as T
}

export type WebUser = { name: string }
export type AuthState = { enabled: boolean; user: WebUser | null }

async function send<T>(path: string, body?: unknown): Promise<T> {
  const response = await fetch(`/api/auth/${path}`, {
    method: 'POST',
    headers: body === undefined ? {} : { 'content-type': 'application/json' },
    body: body === undefined ? undefined : JSON.stringify(body),
  })
  const data = await response.json().catch(() => ({}))
  if (!response.ok) throw data.error ?? 'El servidor no respondió.'
  return data as T
}

export const account = {
  me: () => api<AuthState>('auth/me', {}),
  login: (login: string, password: string) => send<{ user: WebUser }>('login', { login, password }),
  register: (name: string, email: string, password: string) =>
    send<{ user: WebUser }>('register', { name, email, password }),
  logout: () => fetch('/api/auth/logout', { method: 'POST' }),
}

export const web = {
  catalog: () => api<Catalog>('catalog', {}),
  profile: (platform: string, name: string, tag: string) => api<WebProfile>('profile', { platform, name, tag }),
  detail: (platform: string, puuid: string, id: number) => api<Detail>('match', { platform, puuid, id: String(id) }),
}
