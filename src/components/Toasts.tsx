import type { ToastItem } from "../types";
import { CheckIcon, CloseIcon, WarningIcon, ShieldIcon } from "./Icons";

interface Props {
  items: ToastItem[];
  onDismiss: (id: number) => void;
}

export function Toasts({ items, onDismiss }: Props) {
  if (items.length === 0) return null;
  return (
    <div className="toasts">
      {items.map((item) => (
        <div key={item.id} className={`toast ${item.kind}`}>
          <span className="ico">
            {item.kind === "success" ? (
              <CheckIcon size={15} />
            ) : item.kind === "error" ? (
              <WarningIcon size={15} />
            ) : (
              <ShieldIcon size={15} />
            )}
          </span>
          <div className="body">
            {item.message}
            {item.action ? (
              <button
                className="act"
                onClick={() => {
                  onDismiss(item.id);
                  void item.action?.run();
                }}
              >
                {item.action.label} →
              </button>
            ) : null}
          </div>
          <button className="close" onClick={() => onDismiss(item.id)} title="关闭">
            <CloseIcon size={13} />
          </button>
        </div>
      ))}
    </div>
  );
}
