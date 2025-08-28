import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "./ui/select";
import { Badge } from "./ui/badge";
import { Button } from "./ui/button";
import { RefreshCw, Bell, User, Search, Command, Sun, Moon } from "lucide-react";
import { Input } from "./ui/input";
import { useTheme } from "./ThemeProvider";
import { useClusterHealth } from "./ui/use-cluster-health";

interface Pod {
  name: string;
  image: string;
  labels: Record<string, string>;
  node: string;
  status: string;
  cpuUsage: string;
  memoryUsage: string;
  age: string;
  ready: string;
  restarts: number;
  ip: string;
}

interface HeaderProps {
  selectedNamespace: string;
  onNamespaceChange: (namespace: string) => void;
  compact?: boolean;
  mobile?: boolean;
  podCount?: number;
  pods: Pod[];
}

export function Header({ selectedNamespace, onNamespaceChange, compact = false, mobile = false, podCount, pods }: HeaderProps) {
  const namespaces = ["default", "kube-system", "monitoring", "ingress-nginx", "cert-manager"];
  const { theme, toggleTheme } = useTheme();
  const clusterHealth = useClusterHealth(pods);

  if (mobile) {
    return (
      <header className="h-16 bg-card/60 backdrop-blur-xl border-b border-border shadow-lg px-4 flex items-center justify-between relative">
        {/* Background gradient */}
        <div className="absolute inset-0 bg-gradient-to-r from-muted/10 to-muted/20 dark:from-muted/5 dark:to-muted/10"></div>
        
        <div className="flex items-center gap-2 relative z-10">
          <Select value={selectedNamespace} onValueChange={onNamespaceChange}>
            <SelectTrigger className="w-32 h-8 bg-card/80 backdrop-blur-sm border-border/30 shadow-sm text-xs">
              <SelectValue />
            </SelectTrigger>
            <SelectContent className="bg-card/95 backdrop-blur-xl border-border/30">
              {namespaces.map((ns) => (
                <SelectItem key={ns} value={ns} className="hover:bg-accent/60 text-xs">
                  {ns}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
        </div>
        
        <div className="flex items-center gap-2 relative z-10">
          <Button 
            variant="ghost" 
            size="sm" 
            className="w-8 h-8 rounded-lg bg-card/60 hover:bg-card/80"
            onClick={toggleTheme}
          >
            {theme === 'light' ? (
              <Moon className="w-3 h-3" />
            ) : (
              <Sun className="w-3 h-3" />
            )}
          </Button>
          <Button variant="ghost" size="sm" className="w-8 h-8 rounded-lg bg-primary hover:bg-primary/80">
            <User className="w-3 h-3 text-primary-foreground" />
          </Button>
        </div>
      </header>
    );
  }

  const headerHeight = mobile ? "h-16" : compact ? "h-16" : "h-20";
  const paddingX = mobile ? "px-4" : compact ? "px-6" : "px-8";

  return (
    <header className={`${headerHeight} bg-card/60 backdrop-blur-xl border-b border-border shadow-lg ${paddingX} flex items-center justify-between relative`}>
      {/* Background gradient */}
      <div className="absolute inset-0 bg-gradient-to-r from-muted/10 to-muted/20 dark:from-muted/5 dark:to-muted/10"></div>
      
      <div className="flex items-center gap-3 lg:gap-6 relative z-10 flex-1 min-w-0">
        <div className="flex items-center gap-2 lg:gap-3">
          <span className="text-xs lg:text-sm font-semibold text-foreground/70 whitespace-nowrap">Namespace:</span>
          <Select value={selectedNamespace} onValueChange={onNamespaceChange}>
            <SelectTrigger className={`${compact ? 'w-36 h-8' : 'w-48 h-10'} bg-card/80 backdrop-blur-sm border-border/30 shadow-sm hover:shadow-md transition-all`}>
              <SelectValue />
            </SelectTrigger>
            <SelectContent className="bg-card/95 backdrop-blur-xl border-border/30">
              {namespaces.map((ns) => (
                <SelectItem key={ns} value={ns} className="hover:bg-accent/60">
                  {ns}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
        </div>
        
        {!compact && (
          <div className="flex items-center gap-2 lg:gap-3 min-w-0">
            <Badge className={`gap-2 px-2 lg:px-3 py-1 lg:py-1.5 whitespace-nowrap text-xs lg:text-sm transition-colors
              ${clusterHealth.status === 'Healthy' ? 'bg-emerald-50 dark:bg-emerald-950 text-emerald-700 dark:text-emerald-300 border-emerald-200 dark:border-emerald-800 hover:bg-emerald-50 dark:hover:bg-emerald-950' : ''}
              ${clusterHealth.status === 'Warning' ? 'bg-amber-50 dark:bg-amber-950 text-amber-700 dark:text-amber-300 border-amber-200 dark:border-amber-800 hover:bg-amber-50 dark:hover:bg-amber-950' : ''}
              ${clusterHealth.status === 'Critical' ? 'bg-red-50 dark:bg-red-950 text-red-700 dark:text-red-300 border-red-200 dark:border-red-800 hover:bg-red-50 dark:hover:bg-red-950' : ''}
            `}>
              <div className={`w-2 lg:w-2.5 h-2 lg:h-2.5 ${clusterHealth.dotColor} rounded-full ${clusterHealth.status === 'Healthy' ? 'animate-pulse' : ''}`}></div>
              Cluster {clusterHealth.status}
            </Badge>
            <Badge className="gap-2 px-2 lg:px-3 py-1 lg:py-1.5 bg-slate-50 dark:bg-slate-950 text-slate-700 dark:text-slate-300 border-slate-200 dark:border-slate-800 hover:bg-slate-50 dark:hover:bg-slate-950 whitespace-nowrap text-xs lg:text-sm hidden sm:flex">
              <div className="w-2 lg:w-2.5 h-2 lg:h-2.5 bg-slate-500 rounded-full"></div>
              {podCount || 0} Pods Running
            </Badge>
          </div>
        )}

        {/* Search Bar - Only on larger screens */}
        {!compact && (
          <div className="relative ml-2 lg:ml-4 hidden xl:block flex-1 max-w-sm">
            <Search className="absolute left-3 top-1/2 transform -translate-y-1/2 w-4 h-4 text-muted-foreground" />
            <Input
              placeholder="Search resources..."
              className="w-full pl-10 pr-12 h-8 lg:h-10 bg-card/80 backdrop-blur-sm border-border/30 shadow-sm hover:shadow-md transition-all text-sm"
            />
            <div className="absolute right-3 top-1/2 transform -translate-y-1/2 flex items-center gap-1">
              <Command className="w-3 h-3 text-muted-foreground" />
              <span className="text-xs text-muted-foreground">K</span>
            </div>
          </div>
        )}
      </div>
      
      <div className="flex items-center gap-2 lg:gap-3 relative z-10">
        <Button 
          variant="ghost" 
          size="sm" 
          className={`${compact ? 'w-8 h-8' : 'w-10 h-10'} rounded-xl bg-card/60 hover:bg-card/80 hover:shadow-md transition-all`}
          onClick={toggleTheme}
        >
          {theme === 'light' ? (
            <Moon className="w-4 h-4" />
          ) : (
            <Sun className="w-4 h-4" />
          )}
        </Button>
        <Button variant="ghost" size="sm" className={`${compact ? 'w-8 h-8' : 'w-10 h-10'} rounded-xl bg-card/60 hover:bg-card/80 hover:shadow-md transition-all hidden sm:flex`}>
          <RefreshCw className="w-4 h-4" />
        </Button>
        <Button variant="ghost" size="sm" className={`${compact ? 'w-8 h-8' : 'w-10 h-10'} rounded-xl bg-card/60 hover:bg-card/80 hover:shadow-md transition-all relative hidden sm:flex`}>
          <Bell className="w-4 h-4" />
          <div className="absolute -top-1 -right-1 w-3 h-3 bg-red-500 rounded-full flex items-center justify-center">
            <span className="text-xs text-white font-bold">2</span>
          </div>
        </Button>
        <Button variant="ghost" size="sm" className={`${compact ? 'w-8 h-8' : 'w-10 h-10'} rounded-xl bg-card/60 hover:bg-card/80 hover:shadow-md transition-all`}>
          <User className="w-4 h-4 text-primary-foreground" />
        </Button>
      </div>
    </header>
  );
}