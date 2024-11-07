import "./App.css";
import ArchiveViewer from "./pages/ArchiveViewer";
import { invoke } from "@tauri-apps/api/tauri";

function App() {
  const toggleDebugWindow = async () => {
    try {
      await invoke("toggle_debug_window");
    } catch (error) {
      console.error("Failed to toggle debug window:", error);
    }
  };

  return (
    <div class="min-h-screen bg-gray-100">
      {/* Header */}
      <header class="fixed top-0 w-full bg-blue-600 text-white shadow-lg z-50">
        <div class="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8">
          <div class="flex justify-between items-center h-16">
            {/* Logo and Title */}
            <div>
              <h1 class="text-2xl font-bold">EPICS Archive Viewer</h1>
              <p class="text-sm text-blue-100">
                Interactive data visualization for EPICS process variables
              </p>
            </div>

            {/* Toolbar */}
            <div class="flex items-center gap-4">
              <button
                onClick={toggleDebugWindow}
                class="inline-flex items-center px-4 py-2 bg-white text-blue-600 
                       rounded shadow-sm text-sm font-medium
                       hover:bg-blue-50 focus:outline-none focus:ring-2 
                       focus:ring-offset-2 focus:ring-blue-500 transition-colors"
                aria-label="Toggle Debug Window"
              >
                Debug Info
              </button>
            </div>
          </div>
        </div>
      </header>

      {/* Main Content */}
      <main class="pt-20">
        <ArchiveViewer />
      </main>
    </div>
  );
}

export default App;