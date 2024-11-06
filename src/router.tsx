// src/router.tsx
import { Route } from '@solidjs/router';
import App from './App';
import DebugView from './pages/DebugView';
import { Router as SolidRouter } from '@solidjs/router';

export function Router() {
  return (
    <SolidRouter>
      <Route path="/" component={App} />
      <Route path="/debug-view" component={DebugView} />
    </SolidRouter>
  );
}