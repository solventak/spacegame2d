import './app.css';
import App from './App.svelte';
import { readBootstrap } from './bridge';
let player;
try { player = readBootstrap().localPlayer; } catch { player = undefined; }
new App({ target: document.getElementById('app')!, props: { player } });
