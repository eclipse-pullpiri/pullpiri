import { Button } from "./ui/button";
import { Separator } from "./ui/separator";
import { 
  Box, 
  Network, 
  Database,
  Server,
  ChevronLeft,
  ChevronRight,
  //Menu,
//  Hexagon,
  Shield
} from "lucide-react";
import { ImageWithFallback } from "./common/ImageWithFallback";
import pullpiriLogo from "../assets/02bb575d7f3a54097c4c2d1077de32d01bed5ead.png";

interface SidebarProps {
  currentView: string;
  onViewChange: (view: "workloads" | "services" | "storage" | "cluster") => void;
  collapsed?: boolean;
  onToggle?: () => void;
  mobile?: boolean;
}

export function Sidebar({ currentView, onViewChange, collapsed = false, onToggle, mobile = false }: SidebarProps) {
  const menuItems = [
    { id: "workloads", label: "Workloads", icon: Box },
    { id: "services", label: "Services", icon: Network },
    { id: "storage", label: "Storage", icon: Database },
    { id: "cluster", label: "Cluster", icon: Server },
  ];

  // Mobile sidebar (bottom navigation style)
  if (mobile) {
    return (
      <div className="w-16 bg-card/70 backdrop-blur-xl border-r border-border shadow-2xl flex flex-col relative transition-colors duration-300">
        <div className="absolute inset-0 bg-gradient-to-b from-muted/20 to-muted/40 dark:from-muted/10 dark:to-muted/20"></div>
        <nav className="flex-1 p-2 space-y-2 relative z-10 flex flex-col items-center justify-center">
          {menuItems.map((item) => {
            const Icon = item.icon;
            const isActive = currentView === item.id;
            return (
              <Button
                key={item.id}
                variant="ghost"
                className={`w-12 h-12 p-0 rounded-xl transition-all duration-300 ${
                  isActive 
                    ? "bg-primary/10 shadow-lg text-foreground border border-border/30" 
                    : "hover:bg-muted/40 hover:shadow-md text-muted-foreground hover:text-foreground"
                }`}
                onClick={() => onViewChange(item.id as any)}
                title={item.label}
              >
                <div className={`w-8 h-8 rounded-lg flex items-center justify-center ${
                  isActive 
                    ? "bg-primary text-primary-foreground" 
                    : "text-muted-foreground group-hover:text-foreground"
                } transition-all duration-300`}>
                  <Icon className="w-4 h-4" />
                </div>
              </Button>
            );
          })}
        </nav>
      </div>
    );
  }

  // Desktop/Tablet sidebar
  const sidebarWidth = collapsed ? "w-16" : "w-72";

  return (
    <div className={`${sidebarWidth} bg-card/70 backdrop-blur-xl border-r border-border shadow-2xl flex flex-col relative transition-all duration-300`}>
      {/* Background Pattern */}
      <div className="absolute inset-0 bg-gradient-to-b from-muted/20 to-muted/40 dark:from-muted/10 dark:to-muted/20"></div>
      <div className="absolute inset-0 opacity-5 dark:opacity-10">
        <div className="absolute top-8 left-8 w-2 h-2 bg-primary/20 rounded-full"></div>
        <div className="absolute top-16 right-12 w-1 h-1 bg-primary/20 rounded-full"></div>
        <div className="absolute top-32 left-16 w-1.5 h-1.5 bg-primary/20 rounded-full"></div>
        <div className="absolute bottom-32 right-8 w-2 h-2 bg-primary/20 rounded-full"></div>
        <div className="absolute bottom-16 left-12 w-1 h-1 bg-primary/20 rounded-full"></div>
      </div>
      
      <div className="relative z-10 p-4 lg:p-8">
        <div className="flex items-center justify-between">
          <Button
            variant="ghost" 
            className={`flex items-center gap-3 ${collapsed ? 'justify-center p-2' : 'p-2'} hover:bg-muted/20 rounded-xl transition-all duration-300`}
            onClick={() => onViewChange("workloads")}
            title="Go to Workloads"
          >
            <div className="w-18 h-18 rounded-2xl flex items-center justify-center shadow-lg overflow-hidden bg-white/10 backdrop-blur-sm">
              <ImageWithFallback
                src={pullpiriLogo}
                alt="PULLPIRI Logo"
                className="w-full h-full object-contain p-1"
              //  fallback={<Hexagon className="w-10 h-10 text-primary" />}
              />
            </div>
            {!collapsed && (
              <div className="text-left">
                <h2 className="font-bold text-foreground">
                  PULLPIRI
                </h2>
                <p className="text-sm text-muted-foreground font-medium">Control Center</p>
              </div>
            )}
          </Button>
          {onToggle && !mobile && (
            <Button
              variant="ghost"
              size="sm"
              onClick={onToggle}
              className="w-8 h-8 rounded-lg hidden lg:flex"
            >
              {collapsed ? <ChevronRight className="w-4 h-4" /> : <ChevronLeft className="w-4 h-4" />}
            </Button>
          )}
        </div>
      </div>
      
      <Separator className="bg-border/30" />
      
      <nav className={`flex-1 p-3 lg:p-6 space-y-2 lg:space-y-3 relative z-10 ${collapsed ? 'px-2' : ''}`}>
        {menuItems.map((item) => {
          const Icon = item.icon;
          const isActive = currentView === item.id;
          return (
            <Button
              key={item.id}
              variant="ghost"
              className={`w-full ${collapsed ? 'h-12 justify-center' : 'justify-start h-12 lg:h-14'} gap-4 text-left transition-all duration-300 relative group ${
                isActive 
                  ? "bg-primary/10 shadow-lg scale-105 text-foreground border border-border/30" 
                  : "hover:bg-muted/40 hover:shadow-md hover:scale-[1.02] text-muted-foreground hover:text-foreground"
              }`}
              onClick={() => onViewChange(item.id as any)}
              title={collapsed ? item.label : undefined}
            >
              <div className={`w-8 lg:w-10 h-8 lg:h-10 rounded-xl flex items-center justify-center ${
                isActive 
                  ? "bg-primary text-primary-foreground shadow-md" 
                  : "bg-muted/60 text-muted-foreground group-hover:bg-primary/10 group-hover:text-foreground"
              } transition-all duration-300`}>
                <Icon className="w-4 lg:w-5 h-4 lg:h-5" />
              </div>
              {!collapsed && <span className="font-medium text-sm lg:text-base">{item.label}</span>}
              {isActive && !collapsed && (
                <div className="absolute right-3 w-2 h-2 rounded-full bg-primary"></div>
              )}
            </Button>
          );
        })}
      </nav>
      
      {!collapsed && (
        <div className="p-3 lg:p-6 border-t border-border/20 relative z-10">
          <div className="bg-card/50 backdrop-blur-sm rounded-2xl p-3 lg:p-4 border border-border/30">
            <div className="flex items-center gap-2 lg:gap-3 text-xs lg:text-sm">
              <div className="w-6 lg:w-8 h-6 lg:h-8 bg-gradient-to-r from-emerald-500 to-emerald-600 rounded-lg flex items-center justify-center">
                <Shield className="w-3 lg:w-4 h-3 lg:h-4 text-white" />
              </div>
              <div>
                <p className="font-semibold text-foreground text-xs lg:text-sm">Production Cluster</p>
                <div className="flex items-center gap-1">
                  <div className="w-2 h-2 bg-emerald-500 rounded-full animate-pulse"></div>
                  <span className="text-xs text-muted-foreground">All systems operational</span>
                </div>
              </div>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
