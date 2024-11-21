// components/Modal.tsx
interface ModalProps {
    isOpen: boolean;
    onClose: () => void;
    children: any;
    title?: string;
  }
  
  export function Modal(props: ModalProps) {
    return (
      <>
        {props.isOpen && (
          <div class="fixed inset-0 z-50">
            {/* Backdrop */}
            <div 
              class="fixed inset-0 bg-black bg-opacity-50"
              onClick={props.onClose}
            />
            
            {/* Modal */}
            <div class="fixed top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2 bg-white rounded-lg shadow-lg p-6 max-w-md w-full max-h-[90vh] overflow-y-auto">
              {props.title && (
                <div class="flex justify-between items-center mb-4">
                  <h2 class="text-lg font-semibold">{props.title}</h2>
                  <button 
                    onClick={props.onClose}
                    class="text-gray-500 hover:text-gray-700"
                  >
                    ×
                  </button>
                </div>
              )}
              {props.children}
            </div>
          </div>
        )}
      </>
    );
  }