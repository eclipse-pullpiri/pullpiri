import { useState, useEffect, useRef } from "react";
import { Dialog, DialogContent, DialogDescription, DialogHeader, DialogTitle } from "./ui/dialog";
import { Button } from "./ui/button";
import { Badge } from "./ui/badge";
import { ScrollArea } from "./ui/scroll-area";
import { /*X, */Download, Pause, Play, RefreshCw } from "lucide-react";

interface LogsDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  podName: string;
}

export function LogsDialog({ open, onOpenChange, podName }: LogsDialogProps) {
  const [logs, setLogs] = useState<string[]>([]);
  const [isStreaming, setIsStreaming] = useState(true);
  const [lastUpdate, setLastUpdate] = useState(new Date());
  const scrollAreaRef = useRef<HTMLDivElement>(null);

  // Mock log data generator
  const generateMockLog = () => {
    const logLevels = ['INFO', 'WARN', 'ERROR', 'DEBUG'];
    const messages = [
      'Application started successfully',
      'Database connection established',
      'Processing incoming request',
      'Cache miss for key: user_session_123',
      'Memory usage: 245MB/512MB',
      'HTTP GET /api/users - 200 OK',
      'Scheduled task completed',
      'Configuration reloaded',
      'Health check passed',
      'Background job queued'
    ];
    
    const level = logLevels[Math.floor(Math.random() * logLevels.length)];
    const message = messages[Math.floor(Math.random() * messages.length)];
    const timestamp = new Date().toISOString();
    
    return `${timestamp} [${level}] ${message}`;
  };

  // Simulate log streaming
  useEffect(() => {
    if (!open || !isStreaming) return;

    // Add initial logs
    const initialLogs = Array.from({ length: 15 }, () => generateMockLog());
    setLogs(initialLogs);

    const interval = setInterval(() => {
      const newLog = generateMockLog();
      setLogs(prev => [...prev, newLog].slice(-100)); // Keep only last 100 logs
      setLastUpdate(new Date());
    }, 2000);

    return () => clearInterval(interval);
  }, [open, isStreaming]);

  // Auto-scroll to bottom
  useEffect(() => {
    if (scrollAreaRef.current) {
      const scrollElement = scrollAreaRef.current.querySelector('[data-radix-scroll-area-viewport]');
      if (scrollElement) {
        scrollElement.scrollTop = scrollElement.scrollHeight;
      }
    }
  }, [logs]);

  const handleDownloadLogs = () => {
    const logContent = logs.join('\n');
    const blob = new Blob([logContent], { type: 'text/plain' });
    const url = URL.createObjectURL(blob);
    const link = document.createElement('a');
    link.href = url;
    link.download = `${podName}-logs.txt`;
    document.body.appendChild(link);
    link.click();
    document.body.removeChild(link);
    URL.revokeObjectURL(url);
  };

  const toggleStreaming = () => {
    setIsStreaming(!isStreaming);
  };

  const refreshLogs = () => {
    setLogs([]);
    setIsStreaming(true);
  };

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="w-[75vw] min-w-[800px] max-w-6xl h-[45vh] min-h-[400px] max-h-[650px] flex flex-col bg-card/95 backdrop-blur-sm">
        <DialogHeader className="flex-shrink-0">
          <div className="flex items-center justify-between">
            <div>
              <DialogTitle className="text-foreground">Pod Logs</DialogTitle>
              <DialogDescription>
                Real-time logs for <span className="font-semibold text-primary">{podName}</span>
              </DialogDescription>
            </div>
            <div className="flex items-center gap-2">
              <Badge variant={isStreaming ? "default" : "secondary"} className="text-xs">
                {isStreaming ? (
                  <>
                    <div className="w-2 h-2 bg-emerald-500 rounded-full mr-1 animate-pulse"></div>
                    Live
                  </>
                ) : (
                  'Paused'
                )}
              </Badge>
              <span className="text-xs text-muted-foreground">
                {lastUpdate.toLocaleTimeString()}
              </span>
            </div>
          </div>
        </DialogHeader>

        <div className="flex items-center gap-2 py-2 border-b border-border/20">
          <Button
            variant="outline"
            size="sm"
            onClick={toggleStreaming}
            className="h-8"
          >
            {isStreaming ? (
              <>
                <Pause className="w-3 h-3 mr-1" />
                Pause
              </>
            ) : (
              <>
                <Play className="w-3 h-3 mr-1" />
                Resume
              </>
            )}
          </Button>
          <Button
            variant="outline"
            size="sm"
            onClick={refreshLogs}
            className="h-8"
          >
            <RefreshCw className="w-3 h-3 mr-1" />
            Refresh
          </Button>
          <Button
            variant="outline"
            size="sm"
            onClick={handleDownloadLogs}
            className="h-8"
          >
            <Download className="w-3 h-3 mr-1" />
            Download
          </Button>
          <div className="flex-1" />
          <span className="text-xs text-muted-foreground">
            {logs.length} lines
          </span>
        </div>

        <div className="flex-1 min-h-0">
          <ScrollArea ref={scrollAreaRef} className="h-full w-full">
            <div className="p-4 font-mono text-sm space-y-1 bg-muted/20 rounded-lg">
              {logs.length === 0 ? (
                <div className="text-muted-foreground italic text-center py-8">
                  No logs available
                </div>
              ) : (
                logs.map((log, index) => (
                  <div key={index} className="hover:bg-accent/20 px-2 py-1 rounded text-xs leading-relaxed">
                    <span className="text-muted-foreground select-none mr-2">
                      {String(index + 1).padStart(3, '0')}
                    </span>
                    <span className="whitespace-pre-wrap break-all">{log}</span>
                  </div>
                ))
              )}
            </div>
          </ScrollArea>
        </div>
      </DialogContent>
    </Dialog>
  );
}