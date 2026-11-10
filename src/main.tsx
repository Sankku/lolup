import '@fontsource/atkinson-hyperlegible/400.css'
import '@fontsource/atkinson-hyperlegible/700.css'
import './index.css'
import { StrictMode } from 'react'
import { createRoot } from 'react-dom/client'
import { App } from './App'
import { isDesktop } from './api'
import { Overlay } from './Overlay'
import { WebApp } from './WebApp'

const isOverlay = location.hash === '#overlay'
document.documentElement.classList.toggle('overlay', isOverlay)

createRoot(document.getElementById('root')!).render(
  <StrictMode>{isOverlay ? <Overlay /> : isDesktop ? <App /> : <WebApp />}</StrictMode>,
)
