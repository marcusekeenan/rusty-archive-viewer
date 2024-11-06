// src/index.tsx
import { render } from 'solid-js/web';
import { Router, Route } from '@solidjs/router';
import './index.css';
import App from './App';
import DebugView from './pages/DebugView';

const root = document.getElementById('root');

if (root) {
  const isDebugView = window.location.hash.includes('/debug-view');
  
  if (isDebugView) {
    console.log('Rendering Debug View');
    render(() => <DebugView />, root);
  } else {
    console.log('Rendering Main App');
    render(() => (
      <Router>
        <Route path="/" component={App} />
      </Router>
    ), root);
  }
}