import '@fontsource/atkinson-hyperlegible/400.css'
import '@fontsource/atkinson-hyperlegible/700.css'
import './index.css'
import { StrictMode } from 'react'
import { createRoot } from 'react-dom/client'
import { App } from './App'
import { Overlay } from './Overlay'

const isOverlay = location.hash === '#overlay'
document.documentElement.classList.toggle('overlay', isOverlay)

createRoot(document.getElementById('root')!).render(
  <StrictMode>{isOverlay ? <Overlay /> : <App />}</StrictMode>,
)
