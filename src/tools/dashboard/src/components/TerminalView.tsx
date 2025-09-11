import { useState, useEffect, useRef } from "react";
import { Button } from "./ui/button";
import { Badge } from "./ui/badge";
import { X, Minimize2/*, Square*/, Settings } from "lucide-react";

interface TerminalViewProps {
  isVisible: boolean;
  onClose: () => void;
  podName: string;
}

interface CommandHistory {
  command: string;
  output: string[];
  timestamp: Date;
}

export function TerminalView({ isVisible, onClose, podName }: TerminalViewProps) {
  const [currentCommand, setCurrentCommand] = useState("");
  const [commandHistory, setCommandHistory] = useState<CommandHistory[]>([]);
  const [historyIndex, setHistoryIndex] = useState(-1);
  const [isConnected, setIsConnected] = useState(false);
  const inputRef = useRef<HTMLInputElement>(null);
  const terminalRef = useRef<HTMLDivElement>(null);

  // Mock command responses
  const mockResponses: Record<string, string[]> = {
    'ls': ['app.js', 'package.json', 'node_modules', 'logs', 'config'],
    'ls -la': [
      'total 48',
      'drwxr-xr-x 1 root root 4096 Jan 1 12:00 .',
      'drwxr-xr-x 1 root root 4096 Jan 1 12:00 ..',
      '-rw-r--r-- 1 root root 1234 Jan 1 12:00 app.js',
      '-rw-r--r-- 1 root root  567 Jan 1 12:00 package.json',
      'drwxr-xr-x 1 root root 4096 Jan 1 12:00 node_modules'
    ],
    'pwd': ['/app'],
    'whoami': ['root'],
    'ps aux': [
      'USER       PID %CPU %MEM    VSZ   RSS TTY      STAT START   TIME COMMAND',
      'root         1  0.1  1.2  34567  8901 ?        Ss   12:00   0:01 node app.js',
      'root        15  0.0  0.1   4567   890 pts/0    R+   12:15   0:00 ps aux'
    ],
    'top': ['Tasks: 2 total, 1 running, 1 sleeping, 0 stopped, 0 zombie', 'CPU usage: 12.5%', 'Memory usage: 245MB/512MB'],
    'df -h': [
      'Filesystem      Size  Used Avail Use% Mounted on',
      '/dev/sda1        10G  2.1G  7.4G  22% /',
      'tmpfs           256M     0  256M   0% /tmp'
    ],
    'cat /etc/os-release': [
      'NAME="Alpine Linux"',
      'ID=alpine',
      'VERSION_ID=3.18.0',
      'PRETTY_NAME="Alpine Linux v3.18"'
    ],
    'echo $PATH': ['/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin'],
    'help': [
      'Available commands:',
      '  ls, ls -la    - List directory contents',
      '  pwd           - Print working directory',
      '  whoami        - Print current user',
      '  ps aux        - List running processes',
      '  top           - Show system resources',
      '  df -h         - Show disk usage',
      '  cat <file>    - Display file contents',
      '  echo <text>   - Display text',
      '  clear         - Clear terminal',
      '  exit          - Close terminal'
    ]
  };

  // Simulate connection
  useEffect(() => {
    if (isVisible) {
      setIsConnected(false);
      const timer = setTimeout(() => {
        setIsConnected(true);
        setCommandHistory([{
          command: '',
          output: [
            `Connected to ${podName}`,
            'Alpine Linux 3.18.0',
            'Welcome to the pod terminal!',
            'Type "help" for available commands.',
            ''
          ],
          timestamp: new Date()
        }]);
      }, 1500);
      return () => clearTimeout(timer);
    }
  }, [isVisible, podName]);

  // Focus input when terminal becomes visible
  useEffect(() => {
    if (isVisible && isConnected && inputRef.current) {
      inputRef.current.focus();
    }
  }, [isVisible, isConnected]);

  // Auto-scroll to bottom
  useEffect(() => {
    if (terminalRef.current) {
      terminalRef.current.scrollTop = terminalRef.current.scrollHeight;
    }
  }, [commandHistory]);

  const executeCommand = (cmd: string) => {
    const trimmedCmd = cmd.trim();
    
    if (!trimmedCmd) return;

    if (trimmedCmd === 'clear') {
      setCommandHistory([]);
      return;
    }

    if (trimmedCmd === 'exit') {
      onClose();
      return;
    }

    let output: string[] = [];
    
    if (mockResponses[trimmedCmd]) {
      output = mockResponses[trimmedCmd];
    } else if (trimmedCmd.startsWith('echo ')) {
      output = [trimmedCmd.slice(5)];
    } else if (trimmedCmd.startsWith('cat ')) {
      const filename = trimmedCmd.slice(4);
      if (filename === 'app.js') {
        output = [
          'const express = require("express");',
          'const app = express();',
          '',
          'app.get("/", (req, res) => {',
          '  res.json({ status: "healthy" });',
          '});',
          '',
          'app.listen(3000, () => {',
          '  console.log("Server running on port 3000");',
          '});'
        ];
      } else {
        output = [`cat: ${filename}: No such file or directory`];
      }
    } else {
      output = [`bash: ${trimmedCmd}: command not found`];
    }

    const newEntry: CommandHistory = {
      command: trimmedCmd,
      output,
      timestamp: new Date()
    };

    setCommandHistory(prev => [...prev, newEntry]);
  };

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === 'Enter') {
      executeCommand(currentCommand);
      setCurrentCommand("");
      setHistoryIndex(-1);
    } else if (e.key === 'ArrowUp') {
      e.preventDefault();
      const commands = commandHistory.filter(h => h.command).map(h => h.command);
      if (commands.length > 0) {
        const newIndex = historyIndex === -1 ? commands.length - 1 : Math.max(0, historyIndex - 1);
        setHistoryIndex(newIndex);
        setCurrentCommand(commands[newIndex]);
      }
    } else if (e.key === 'ArrowDown') {
      e.preventDefault();
      const commands = commandHistory.filter(h => h.command).map(h => h.command);
      if (historyIndex >= 0) {
        const newIndex = historyIndex + 1;
        if (newIndex >= commands.length) {
          setHistoryIndex(-1);
          setCurrentCommand("");
        } else {
          setHistoryIndex(newIndex);
          setCurrentCommand(commands[newIndex]);
        }
      }
    }
  };

  if (!isVisible) return null;

  return (
    <div className="fixed inset-0 z-50 bg-background">
      {/* Terminal Header */}
      <div className="h-12 bg-card border-b border-border flex items-center justify-between px-4">
        <div className="flex items-center gap-3">
          <div className="flex items-center gap-1">
            <div className="w-3 h-3 rounded-full bg-red-500"></div>
            <div className="w-3 h-3 rounded-full bg-yellow-500"></div>
            <div className="w-3 h-3 rounded-full bg-green-500"></div>
          </div>
          <div className="flex items-center gap-2">
            <span className="font-semibold">Terminal</span>
            <Badge variant="outline" className="text-xs">
              {podName}
            </Badge>
          </div>
        </div>
        
        <div className="flex items-center gap-2">
          <Badge variant={isConnected ? "default" : "secondary"} className="text-xs">
            {isConnected ? (
              <>
                <div className="w-2 h-2 bg-emerald-500 rounded-full mr-1 animate-pulse"></div>
                Connected
              </>
            ) : (
              'Connecting...'
            )}
          </Badge>
          <Button variant="ghost" size="sm" className="h-8 w-8 p-0">
            <Settings className="h-4 w-4" />
          </Button>
          <Button variant="ghost" size="sm" className="h-8 w-8 p-0">
            <Minimize2 className="h-4 w-4" />
          </Button>
          <Button variant="ghost" size="sm" className="h-8 w-8 p-0" onClick={onClose}>
            <X className="h-4 w-4" />
          </Button>
        </div>
      </div>

      {/* Terminal Content */}
      <div className="h-[calc(100vh-48px)] bg-slate-900 text-green-400 overflow-hidden">
        <div
          ref={terminalRef}
          className="h-full overflow-y-auto p-4 font-mono text-sm leading-relaxed"
        >
          {!isConnected ? (
            <div className="flex items-center gap-2">
              <div className="animate-spin w-4 h-4 border-2 border-green-400 border-t-transparent rounded-full"></div>
              <span>Connecting to {podName}...</span>
            </div>
          ) : (
            <>
              {/* Command History */}
              {commandHistory.map((entry, index) => (
                <div key={index} className="mb-2">
                  {entry.command && (
                    <div className="flex items-center gap-2">
                      <span className="text-yellow-400">root@{podName.split('-')[0]}:</span>
                      <span className="text-blue-400">/app$</span>
                      <span className="text-green-400">{entry.command}</span>
                    </div>
                  )}
                  {entry.output.map((line, lineIndex) => (
                    <div key={lineIndex} className="text-gray-300 whitespace-pre-wrap">
                      {line}
                    </div>
                  ))}
                </div>
              ))}

              {/* Current Input Line */}
              <div className="flex items-center gap-2">
                <span className="text-yellow-400">root@{podName.split('-')[0]}:</span>
                <span className="text-blue-400">/app$</span>
                <input
                  ref={inputRef}
                  type="text"
                  value={currentCommand}
                  onChange={(e) => setCurrentCommand(e.target.value)}
                  onKeyDown={handleKeyDown}
                  className="flex-1 bg-transparent border-none outline-none text-green-400 font-mono"
                  placeholder=""
                  autoComplete="off"
                  spellCheck={false}
                />
                <span className="animate-pulse text-green-400">|</span>
              </div>
            </>
          )}
        </div>
      </div>
    </div>
  );
}