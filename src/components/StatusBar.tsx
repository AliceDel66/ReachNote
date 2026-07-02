import { Beaker, ShieldCheck } from "lucide-react";

interface StatusBarProps {
  providerLabel: string;
}

export function StatusBar({ providerLabel }: StatusBarProps) {
  return (
    <footer className="status-bar">
      <span>
        <ShieldCheck size={26} />
        本地优先
      </span>
      <span>
        <Beaker size={28} />
        Pre-alpha
      </span>
      <span>
        <span className="ai-badge">AI</span>
        {providerLabel}
      </span>
    </footer>
  );
}
