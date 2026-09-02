// Import first so the persisted theme + font vars are applied to <html> before
// React mounts, avoiding a flash of the default theme on load.
import './theme-bootstrap.js';
import React from 'react';
import ReactDOM from 'react-dom/client';
import { App } from './App.js';

ReactDOM.createRoot(document.getElementById('root')!).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>
);
