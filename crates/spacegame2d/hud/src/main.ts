import './app.css';
import { mount } from 'svelte';
import App from './App.svelte';
import { readState } from './bridge';
let state;
try { state = readState(); } catch { state = undefined; }
mount(App, { target: document.getElementById('app')!, props: { initialState: state } });
