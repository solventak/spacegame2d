import './app.css';
import { mount } from 'svelte';
import App from './App.svelte';
import { readBootstrap } from './bridge';
let player;
try { player = readBootstrap().localPlayer; } catch { player = undefined; }
mount(App, { target: document.getElementById('app')!, props: { player } });
document.title = 'HUD_READY';
