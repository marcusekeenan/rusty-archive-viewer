// src/pages/DebugView.tsx
import { Component, createSignal, onMount } from 'solid-js';
import { listen } from '@tauri-apps/api/event';

const DebugView: Component = () => {
  console.log('DebugView component created');
  const [logs, setLogs] = createSignal<any[]>([
    {
      timestamp: new Date().toISOString(),
      message: "Debug view initialized",
      type: "info",
      source: "DebugView"
    }
  ]);

  onMount(() => {
    console.log('DebugView mounted');
    
    // Add a visible element to confirm render
    document.body.style.backgroundColor = '#f3f4f6';
    
    listen('debug-log', (event) => {
      console.log('Received debug event:', event);
      setLogs(prev => [...prev, event.payload]);
    }).catch(error => {
      console.error('Failed to set up event listener:', error);
    });
  });

  return (
    <div class="min-h-screen bg-gray-50 p-4">
      <h1 class="text-2xl font-bold text-blue-600 mb-4">Debug Window</h1>
      <div class="space-y-4">
        <div class="bg-yellow-100 p-4 rounded">
          Debug view is active. Logs count: {logs().length}
        </div>
        {logs().map((log, index) => (
          <div class="bg-white shadow rounded p-4">
            <div class="font-bold">{log.message}</div>
            <div class="text-sm text-gray-500">
              {new Date(log.timestamp).toLocaleString()}
            </div>
            {log.details && (
              <pre class="mt-2 bg-gray-50 p-2 rounded text-sm">
                {log.details}
              </pre>
            )}
          </div>
        ))}
      </div>
    </div>
  );
};

export default DebugView;