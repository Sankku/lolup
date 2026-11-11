import { type FormEvent, useState } from 'react'
import { type WebUser, account } from './api'

type Mode = 'login' | 'register'

const field = 'w-full border border-line bg-ink px-3 py-2 text-sm placeholder:text-muted'

function Field(props: {
  id: string
  label: string
  type?: string
  value: string
  onChange: (value: string) => void
  autoComplete: string
}) {
  return (
    <div className="flex flex-col gap-1">
      <label htmlFor={props.id} className="text-xs font-bold tracking-wide text-muted uppercase">
        {props.label}
      </label>
      <input
        id={props.id}
        type={props.type ?? 'text'}
        value={props.value}
        onChange={(e) => props.onChange(e.target.value)}
        autoComplete={props.autoComplete}
        required
        className={field}
      />
    </div>
  )
}

/** Ingreso y alta de la cuenta de LolUp. */
export function AuthView(props: { onSignedIn: (user: WebUser) => void }) {
  const [mode, setMode] = useState<Mode>('login')
  const [login, setLogin] = useState('')
  const [name, setName] = useState('')
  const [email, setEmail] = useState('')
  const [password, setPassword] = useState('')
  const [password2, setPassword2] = useState('')
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState<string | null>(null)

  function switchTo(next: Mode) {
    setMode(next)
    setError(null)
    setPassword('')
    setPassword2('')
  }

  async function submit(event: FormEvent) {
    event.preventDefault()
    if (mode === 'register' && password !== password2) {
      setError('Las contraseñas no coinciden.')
      return
    }
    setBusy(true)
    setError(null)
    try {
      const { user } = mode === 'login' ? await account.login(login, password) : await account.register(name, email, password)
      props.onSignedIn(user)
    } catch (e) {
      setError(typeof e === 'string' ? e : 'Ocurrió un error inesperado.')
    } finally {
      setBusy(false)
    }
  }

  return (
    <section className="rise mx-auto grid max-w-5xl gap-10 pt-6 md:grid-cols-[1fr_380px] md:pt-12">
      <div>
        <h1 className="font-display text-6xl leading-none font-black uppercase sm:text-7xl">
          Tu progreso, <span className="text-signal">partida a partida.</span>
        </h1>
        <p className="mt-4 max-w-md text-muted">
          Creá tu cuenta de LolUp para buscar cualquier invocador y ver su perfil, rango e historial de partidas.
        </p>
      </div>

      <form onSubmit={submit} className="flex flex-col gap-4 border border-line bg-panel p-6" aria-labelledby="auth-title">
        <div className="flex gap-1" role="tablist" aria-label="Cuenta">
          {(['login', 'register'] as const).map((id) => (
            <button
              key={id}
              type="button"
              role="tab"
              aria-selected={mode === id}
              onClick={() => switchTo(id)}
              className={`flex-1 px-3 py-1 font-display text-lg font-semibold ${mode === id ? 'bg-paper text-ink' : 'text-muted hover:text-paper'}`}
            >
              {id === 'login' ? 'Entrar' : 'Crear cuenta'}
            </button>
          ))}
        </div>
        <h2 id="auth-title" className="sr-only">
          {mode === 'login' ? 'Entrar a LolUp' : 'Crear cuenta de LolUp'}
        </h2>

        {mode === 'login' ? (
          <>
            <Field id="login" label="Usuario o email" value={login} onChange={setLogin} autoComplete="username" />
            <Field id="password" label="Contraseña" type="password" value={password} onChange={setPassword} autoComplete="current-password" />
          </>
        ) : (
          <>
            <Field id="name" label="Usuario" value={name} onChange={setName} autoComplete="username" />
            <Field id="email" label="Email" type="email" value={email} onChange={setEmail} autoComplete="email" />
            <Field id="new-password" label="Contraseña" type="password" value={password} onChange={setPassword} autoComplete="new-password" />
            <Field id="new-password2" label="Repetí la contraseña" type="password" value={password2} onChange={setPassword2} autoComplete="new-password" />
            <p className="text-xs text-muted">Usuario de 3 a 20 caracteres (letras, números, _ o -). Contraseña de 8 a 128.</p>
          </>
        )}

        {error && (
          <p role="alert" className="border-l-4 border-loss bg-ink px-3 py-2 text-sm">
            {error}
          </p>
        )}
        <button disabled={busy} className="bg-signal px-4 py-2 font-display text-xl font-bold text-ink disabled:opacity-50">
          {busy ? 'Un momento…' : mode === 'login' ? 'Entrar' : 'Crear cuenta'}
        </button>
      </form>
    </section>
  )
}
