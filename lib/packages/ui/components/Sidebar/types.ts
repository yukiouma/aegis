import type { ComponentType } from 'react';

export interface SubMenuItem {
  link: string;
  title: string;
  icon: ComponentType;
}

export interface MenuItem extends SubMenuItem {
  subMenu?: SubMenuItem[];
}

export interface SidebarProps {
  title: string;
  menu: MenuItem[];
  open: boolean;
  onToggle: () => void;
  onNavigate?: (link: string) => void;
  width?: number;
  collapsedWidth?: number;
}