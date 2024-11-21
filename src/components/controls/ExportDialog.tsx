import { createSignal } from "solid-js";
import { NormalizedPVData } from "../../types";

interface ExportDataDialogProps {
  isOpen: boolean;
  onClose: () => void;
  data: NormalizedPVData[];
  visibleData: NormalizedPVData[];
}

export default function ExportDataDialog(props: ExportDataDialogProps) {
  const [exportType, setExportType] = createSignal<'visible' | 'raw'>('visible');
  const [format, setFormat] = createSignal<'csv' | 'json'>('csv');

  const handleExport = () => {
    const dataToExport = exportType() === 'visible' ? props.visibleData : props.data;
    // Implement export logic here
    console.log(`Exporting ${exportType()} data in ${format()} format`);
    props.onClose();
  };

  if (!props.isOpen) return null;

  return (
    <div class="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center">
      <div class="bg-white rounded-lg p-6 w-96">
        <h2 class="text-xl font-semibold mb-4">Export Data</h2>
        <div class="space-y-4">
          <div>
            <label class="block mb-2">Export Type</label>
            <select
              value={exportType()}
              onChange={(e) => setExportType(e.target.value as 'visible' | 'raw')}
              class="w-full px-3 py-2 border rounded"
            >
              <option value="visible">Visible Data</option>
              <option value="raw">Raw Data</option>
            </select>
          </div>
          <div>
            <label class="block mb-2">Format</label>
            <select
              value={format()}
              onChange={(e) => setFormat(e.target.value as 'csv' | 'json')}
              class="w-full px-3 py-2 border rounded"
            >
              <option value="csv">CSV</option>
              <option value="json">JSON</option>
            </select>
          </div>
          <div class="flex justify-end gap-2">
            <button
              onClick={props.onClose}
              class="px-4 py-2 bg-gray-200 rounded hover:bg-gray-300"
            >
              Cancel
            </button>
            <button
              onClick={handleExport}
              class="px-4 py-2 bg-blue-500 text-white rounded hover:bg-blue-600"
            >
              Export
            </button>
          </div>
        </div>
      </div>
    </div>
  );
}