import { useEffect } from "react";

interface ToastProps {
  message: string;
  level: "info" | "error" | "warning";
  onDismiss: () => void;
}

export function Toast({ message, level, onDismiss }: ToastProps) {
  useEffect(() => {
    const timer = setTimeout(onDismiss, 4000);
    return () => clearTimeout(timer);
  }, [onDismiss]);

  return (
    <div className={`toast toast--${level}`} onClick={onDismiss}>
      {message}
    </div>
  );
}
