import { createSignal } from "solid-js";

interface ConfigurationManagerProps {
  onSave: (name: string) => void;
  onLoad: (name: string) => void;
}

export default function ConfigurationManager(props: ConfigurationManagerProps) {
  const [configName, setConfigName] = createSignal("");
  const [savedConfigs, setSavedConfigs] = createSignal<string[]>([]);

  // In a real implementation, you'd load the saved configurations from storage
  const loadSavedConfigs = () => {
    // This is a placeholder. Implement actual loading logic.
    setSavedConfigs(["Config1", "Config2", "Config3"]);
  };

  const handleSave = () => {
    if (configName()) {
      props.onSave(configName());
      setConfigName("");
      loadSavedConfigs();
    }
  };

  const handleLoad = (name: string) => {
    props.onLoad(name);
  };

  return (
    <div class="bg-white rounded-lg shadow-sm p-4">
      <h3 class="text-lg font-semibold mb-2">Configurations</h3>
      <div class="flex gap-2 mb-4">
        <input
          type="text"
          value={configName()}
          onInput={(e) => setConfigName(e.target.value)}
          placeholder="Configuration name"
          class="flex-grow px-2 py-1 border rounded"
        />
        <button
          onClick={handleSave}
          class="px-3 py-1 bg-blue-500 text-white rounded hover:bg-blue-600"
        >
          Save
        </button>
      </div>
      <div class="space-y-2">
        {savedConfigs().map((config) => (
          <button
            onClick={() => handleLoad(config)}
            class="w-full text-left px-3 py-2 bg-gray-100 rounded hover:bg-gray-200"
          >
            {config}
          </button>
        ))}
      </div>
    </div>
  );
}