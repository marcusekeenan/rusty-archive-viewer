export interface DebugLog {
    timestamp: string;
    message: string;
    type: "info" | "error" | "debug" | "success";
    details?: string | null;
    source?: string;
  }
  
  export const DEBUG_LOG_LIMIT = 50;
  
  export interface DebugDialogProps {
    isOpen: boolean;
    onClose: () => void;
    data: DebugLog[];
  }