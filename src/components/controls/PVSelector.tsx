import { createSignal } from 'solid-js';

type PVSelectorProps = {
  selectedPVs: () => string[];
  onAddPV: (pv: string) => void;
  onRemovePV: (pv: string) => void;
};

const PVSelector = (props: PVSelectorProps) => {
  // Add default PV on component mount
  const defaultPV = "ROOM:LI30:1:OUTSIDE_TEMP";
  if (props.selectedPVs().length === 0) {
    props.onAddPV(defaultPV);
  }
 
  const [searchText, setSearchText] = createSignal('');

  const handleSearch = (e: Event) => {
    e.preventDefault();
    if (searchText().trim()) {
      props.onAddPV(searchText().trim());
      setSearchText('');
    }
  };

  return (
    <div class="flex flex-col gap-4">
      <form onSubmit={handleSearch} class="flex gap-2">
        <input
          type="text"
          value={searchText()}
          onInput={(e) => setSearchText((e.target as HTMLInputElement).value)}
          placeholder="Enter PV name"
          class="flex-1 px-3 py-2 border rounded focus:outline-none focus:ring-2 focus:ring-blue-500"
        />
        <button
          type="submit"
          class="px-4 py-2 bg-blue-500 text-white rounded hover:bg-blue-600"
        >
          Add PV
        </button>
      </form>

      <div class="flex flex-col gap-2">
        <h3 class="font-medium">Selected PVs:</h3>
        {props.selectedPVs().length === 0 ? (
          <p class="text-gray-500">No PVs selected</p>
        ) : (
          <ul class="space-y-2">
            {props.selectedPVs().map(pv => (
              <li class="flex justify-between items-center p-2 bg-gray-50 rounded">
                <span>{pv}</span>
                <button
                  onClick={() => props.onRemovePV(pv)}
                  class="text-red-500 hover:text-red-700"
                >
                  Remove
                </button>
              </li>
            ))}
          </ul>
        )}
      </div>
    </div>
  );
};

export default PVSelector;