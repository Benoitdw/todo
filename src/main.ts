import { mount } from 'svelte'
import App from './App.svelte'

// Self-hosted grotesques — bundled by Vite, so offline/LAN use keeps the
// real typefaces instead of falling back to a system face (the
// optical-alignment pass measures the LOADED font).
import '@fontsource-variable/inter'
import '@fontsource/space-mono/400.css'
import '@fontsource/space-mono/700.css'

import './app.css'

mount(App, { target: document.getElementById('app')! })
