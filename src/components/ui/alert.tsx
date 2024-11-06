import { Show } from "solid-js";
import type { ParentComponent } from "solid-js";

type AlertVariant = "default" | "destructive" | "warning" | "info";

interface AlertProps {
  variant?: AlertVariant;
  class?: string;
}

const variantStyles = {
  default: "bg-gray-100 text-gray-800 border-gray-200",
  destructive: "bg-red-100 text-red-800 border-red-200",
  warning: "bg-yellow-100 text-yellow-800 border-yellow-200",
  info: "bg-blue-100 text-blue-800 border-blue-200",
};

export const Alert: ParentComponent<AlertProps> = (props) => {
  const variant = props.variant || "default";
  
  return (
    <div 
      role="alert"
      class={`relative w-full rounded-lg border p-4 ${variantStyles[variant]} ${props.class || ''}`}
    >
      {props.children}
    </div>
  );
};

export const AlertTitle: ParentComponent<{ class?: string }> = (props) => {
  return (
    <h5 class={`mb-1 font-medium leading-none tracking-tight ${props.class || ''}`}>
      {props.children}
    </h5>
  );
};

export const AlertDescription: ParentComponent<{ class?: string }> = (props) => {
  return (
    <div class={`text-sm ${props.class || ''}`}>
      {props.children}
    </div>
  );
};

// Optional Icon component for alerts
export const AlertIcon: ParentComponent<{ 
  show?: boolean;
  variant?: AlertVariant;
  class?: string;
}> = (props) => {
  const iconStyles = {
    default: "text-gray-600",
    destructive: "text-red-600",
    warning: "text-yellow-600",
    info: "text-blue-600",
  };

  return (
    <Show when={props.show !== false}>
      <span class={`mr-2 inline-block ${iconStyles[props.variant || 'default']} ${props.class || ''}`}>
        {/* You can add different icons based on variant */}
        {props.children || '⚠️'}
      </span>
    </Show>
  );
};