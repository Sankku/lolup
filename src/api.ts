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
}

function channel(onText: (text: string) => void) {
  const stream = new Channel<string>()
  stream.onmessage = onText
  return stream
}

/** Imagen servida por el cliente de League a través del núcleo en Rust. */
export const assetUrl = (path: string) => convertFileSrc(path, 'lcu')

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
