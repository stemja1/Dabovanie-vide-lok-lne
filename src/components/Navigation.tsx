import React from 'react';
import { PlayCircle, Edit3, Film, Terminal, Wrench, Sliders } from 'lucide-react';

export type NavTab = 'pipeline' | 'metadata' | 'player' | 'wizard' | 'logs' | 'settings';

interface NavigationProps {
  activeTab: NavTab;
  onTabChange: (tab: NavTab) => void;
  isPausedForReview: boolean;
  hasOutputVideo: boolean;
}

export const Navigation: React.FC<NavigationProps> = ({
  activeTab,
  onTabChange,
  isPausedForReview,
  hasOutputVideo,
}) => {
  const tabs = [
    {
      id: 'pipeline' as NavTab,
      label: 'Pipeline Štúdio',
      icon: PlayCircle,
      badge: null,
    },
    {
      id: 'metadata' as NavTab,
      label: 'Editor Metadát & Prekladu',
      icon: Edit3,
      badge: isPausedForReview ? 'Vyžaduje kontrolu' : null,
      badgeVariant: 'warning',
    },
    {
      id: 'player' as NavTab,
      label: 'Náhľad & Porovnanie',
      icon: Film,
      badge: hasOutputVideo ? 'Hotovo' : null,
      badgeVariant: 'success',
    },
    {
      id: 'wizard' as NavTab,
      label: 'Setup Sprievodca',
      icon: Wrench,
      badge: null,
    },
    {
      id: 'logs' as NavTab,
      label: 'Logy & Terminál',
      icon: Terminal,
      badge: null,
    },
    {
      id: 'settings' as NavTab,
      label: 'Nastavenia TOML',
      icon: Sliders,
      badge: null,
    },
  ];

  return (
    <nav className="h-12 bg-slate-950/80 border-b border-slate-800/80 px-6 flex items-center gap-1 select-none overflow-x-auto">
      {tabs.map((tab) => {
        const Icon = tab.icon;
        const isActive = activeTab === tab.id;

        return (
          <button
            key={tab.id}
            onClick={() => onTabChange(tab.id)}
            className={`flex items-center gap-2 px-3.5 py-1.5 rounded-lg text-xs font-medium transition-all duration-200 whitespace-nowrap ${
              isActive
                ? 'bg-indigo-600/15 text-indigo-400 border border-indigo-500/30 shadow-sm'
                : 'text-slate-400 hover:text-slate-200 hover:bg-slate-900/80 border border-transparent'
            }`}
          >
            <Icon className={`w-3.5 h-3.5 ${isActive ? 'text-indigo-400' : 'text-slate-500'}`} />
            <span>{tab.label}</span>
            {tab.badge && (
              <span
                className={`text-[10px] px-1.5 py-0.2 rounded-full font-semibold animate-pulse ${
                  tab.badgeVariant === 'warning'
                    ? 'bg-amber-500/20 text-amber-400 border border-amber-500/40'
                    : 'bg-emerald-500/20 text-emerald-400 border border-emerald-500/40'
                }`}
              >
                {tab.badge}
              </span>
            )}
          </button>
        );
      })}
    </nav>
  );
};
