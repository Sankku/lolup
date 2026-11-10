import { type Catalog, type Entry, assetUrl } from './api'

export const es = (value: number) => value.toLocaleString('es')
export const percent = (part: number, total: number) => (total === 0 ? 0 : Math.round((part * 100) / total))
export const kdaOf = (k: number, d: number, a: number) => Math.round(((k + a) * 10) / Math.max(d, 1)) / 10
export const queueName = (catalog: Catalog, id: number) => catalog.queues[id]?.name ?? 'Partida'

export const ROLES: Record<string, string> = {
  TOP: 'Superior',
  JUNGLE: 'Jungla',
  MIDDLE: 'Medio',
  BOTTOM: 'Tirador',
  SUPPORT: 'Soporte',
  UNKNOWN: 'Sin dato',
}

export function GameIcon(props: { entry?: Entry; size: number; round?: boolean }) {
  const { entry, size, round } = props
  const shape = round ? 'rounded-full' : 'rounded-sm'
  if (!entry?.icon) {
    return <span className={`inline-block shrink-0 bg-line ${shape}`} style={{ width: size, height: size }} title={entry?.name} />
  }
  return (
    <img
      src={assetUrl(entry.icon)}
      alt={entry.name}
      title={entry.name}
      width={size}
      height={size}
      loading="lazy"
      className={`shrink-0 bg-line ${shape}`}
    />
  )
}

export function Loadout(props: { catalog: Catalog; spells: number[]; keystone: number; subStyle: number; size?: number }) {
  const { catalog, size = 20 } = props
  return (
    <div className="grid shrink-0 grid-cols-2 gap-0.5" style={{ width: size * 2 + 2 }}>
      <GameIcon entry={catalog.spells[props.spells[0]]} size={size} />
      <GameIcon entry={catalog.runes[props.keystone]} size={size} round />
      <GameIcon entry={catalog.spells[props.spells[1]]} size={size} />
      <GameIcon entry={catalog.runes[props.subStyle]} size={size} round />
    </div>
  )
}

export function Items(props: { catalog: Catalog; items: number[]; size?: number }) {
  const { catalog, size = 26 } = props
  return (
    <div className="flex shrink-0 gap-0.5">
      {props.items.map((id, index) => (
        <GameIcon key={index} entry={id > 0 ? catalog.items[id] : undefined} size={size} round={false} />
      ))}
    </div>
  )
}

export function Bar(props: { value: number; max: number; tone?: string }) {
  return (
    <div className="h-1 rounded-full bg-line" aria-hidden="true">
      <div
        className={`h-full rounded-full ${props.tone ?? 'bg-paper'}`}
        style={{ width: `${props.max > 0 ? Math.min((props.value * 100) / props.max, 100) : 0}%` }}
      />
    </div>
  )
}
