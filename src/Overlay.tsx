import { listen } from '@tauri-apps/api/event'
import { useEffect, useState } from 'react'
import { getPlan } from './api'

export function Overlay() {
  const [plan, setPlan] = useState('')

  useEffect(() => {
    getPlan().then(setPlan)
    const unlisten = listen<string>('plan', (event) => setPlan(event.payload))
    return () => {
      unlisten.then((stop) => stop())
    }
  }, [])

  const lines = plan.split('\n').filter(Boolean)

  return (
    <section className="m-2 border-l-4 border-signal bg-ink/90 p-4">
      <h1 className="font-display text-sm font-bold tracking-[0.2em] text-signal uppercase">Plan fijo</h1>
      {lines.length === 0 ? (
        <p className="mt-2 text-sm text-muted">Todavía no hay plan.</p>
      ) : (
        <dl className="mt-2 space-y-2">
          {lines.map((line) => {
            const [label, ...rest] = line.split(':')
            return (
              <div key={line}>
                <dt className="font-display text-xs font-bold tracking-wider text-muted uppercase">{label}</dt>
                <dd className="text-sm leading-snug">{rest.join(':').trim()}</dd>
              </div>
            )
          })}
        </dl>
      )}
    </section>
  )
}
