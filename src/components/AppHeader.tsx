import { Minimize2, Search, Settings2 } from "lucide-react";
import type { ReactNode } from "react";

import brandMark from "../../assets/reachnote_brand_assets/png/icon/reachnote-symbol-transparent-64.png";
import { NAV_ITEMS } from "../constants";
import type { NavKey } from "../types";

interface AppHeaderProps {
  activeNav: NavKey;
  onNavChange: (key: NavKey) => void;
  onSearchClick: () => void;
  searchActive: boolean;
  onShrink: () => void;
  shrinkDisabled: boolean;
}

export function AppHeader({
  activeNav,
  onNavChange,
  onSearchClick,
  searchActive,
  onShrink,
  shrinkDisabled
}: AppHeaderProps) {
  return (
    <header className="app-header">
      <div className="brand-block">
        <img
          className="brand-mark"
          src={brandMark}
          alt="ReachNote"
        />
        <span className="brand-name">ReachNote</span>
      </div>
      <nav className="top-nav" aria-label="主导航">
        {NAV_ITEMS.map((item) => (
          <button
            key={item.key}
            type="button"
            className={`nav-item ${activeNav === item.key ? "active" : ""}`}
            onClick={() => onNavChange(item.key)}
          >
            {item.label}
          </button>
        ))}
      </nav>
      <div className="header-actions">
        <IconButton label="搜索" active={searchActive} onClick={onSearchClick}>
          <Search size={22} strokeWidth={2.15} />
        </IconButton>
        <IconButton
          label="设置"
          active={activeNav === "settings"}
          onClick={() => onNavChange("settings")}
        >
          <Settings2 size={22} strokeWidth={2.1} />
        </IconButton>
        <IconButton label="隐藏到系统菜单栏" onClick={onShrink} disabled={shrinkDisabled}>
          <Minimize2 size={22} strokeWidth={2.05} />
        </IconButton>
      </div>
    </header>
  );
}

interface IconButtonProps {
  label: string;
  active?: boolean;
  disabled?: boolean;
  onClick?: () => void;
  children: ReactNode;
}

function IconButton({ label, active, disabled, onClick, children }: IconButtonProps) {
  return (
    <button
      type="button"
      className={`icon-button ${active ? "active" : ""}`}
      aria-label={label}
      title={label}
      disabled={disabled}
      onClick={onClick}
    >
      {children}
    </button>
  );
}
