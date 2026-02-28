import React, { useEffect, useLayoutEffect, useRef, useState } from 'react';

interface ContextMenuItem {
  icon: string;
  label: string;
  onClick: () => void;
  disabled?: boolean;
  separator?: boolean;
}

interface ContextMenuProps {
  x: number;
  y: number;
  onClose: () => void;
  items: ContextMenuItem[];
}

const ContextMenu: React.FC<ContextMenuProps> = ({ x, y, onClose, items }) => {
  const menuRef = useRef<HTMLDivElement>(null);
  const [nudge, setNudge] = useState({ left: 0, top: 0 });

  useLayoutEffect(() => {
    if (!menuRef.current) return;
    const r = menuRef.current.getBoundingClientRect();
    const pad = 8;
    let left = 0;
    let top = 0;
    if (r.right > window.innerWidth) left = window.innerWidth - r.right - pad;
    if (r.bottom > window.innerHeight) top = window.innerHeight - r.bottom - pad;
    if (r.left + left < pad) left = pad - r.left;
    if (r.top + top < pad) top = pad - r.top;
    setNudge({ left, top });
  }, [x, y, items.length]);

  useEffect(() => {
    const handleClickOutside = (e: MouseEvent) => {
      if (menuRef.current && !menuRef.current.contains(e.target as Node)) {
        onClose();
      }
    };

    const handleEscape = (e: KeyboardEvent) => {
      if (e.key === 'Escape') {
        onClose();
      }
    };

    document.addEventListener('mousedown', handleClickOutside);
    document.addEventListener('keydown', handleEscape);

    return () => {
      document.removeEventListener('mousedown', handleClickOutside);
      document.removeEventListener('keydown', handleEscape);
    };
  }, [onClose]);

  return (
    <div
      ref={menuRef}
      className="context-menu"
      style={{ left: `${x + nudge.left}px`, top: `${y + nudge.top}px` }}
    >
      {items.map((item, index) => (
        item.separator ? (
          <div key={index} className="context-menu-separator" />
        ) : (
          <div
            key={index}
            className={`context-menu-item ${item.disabled ? 'disabled' : ''}`}
            onClick={() => {
              if (item.disabled) return;
              item.onClick?.();
              onClose();
            }}
          >
            <span className="context-menu-icon">{item.icon}</span>
            <span>{item.label}</span>
          </div>
        )
      ))}
    </div>
  );
};

export default ContextMenu;
