import "./App.css";
import ArchiveViewer from "./pages/ArchiveViewer";

function App() {
  return (
    <div class="min-h-screen bg-gray-100">
      {/* Header */}
      <nav class="fixed top-0 w-full bg-blue-600 text-white p-4 shadow-lg z-50">
        <div class="px-4">
          <div class="flex justify-between items-center">
            <div>
              <h1 class="text-2xl font-bold">EPICS Archive Viewer</h1>
              <p class="text-sm mt-1 text-blue-100">
                Interactive data visualization for EPICS process variables
              </p>
            </div>
          </div>
        </div>
      </nav>

      {/* Main content with top padding to account for fixed header */}
      <main class="pt-20">
        <ArchiveViewer />
      </main>
    </div>
  );
}

export default App;