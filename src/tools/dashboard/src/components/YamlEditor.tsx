import { useState } from "react";
import { Dialog, DialogContent, DialogDescription, DialogHeader, DialogTitle, DialogFooter } from "./ui/dialog";
import { Button } from "./ui/button";
import { Badge } from "./ui/badge";
import { Textarea } from "./ui/textarea";
import { ScrollArea } from "./ui/scroll-area";
import { Alert, AlertDescription } from "./ui/alert";
import { Save, X, RotateCcw, Download, AlertTriangle, CheckCircle } from "lucide-react";

interface YamlEditorProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  podName: string;
}

export function YamlEditor({ open, onOpenChange, podName }: YamlEditorProps) {
  const [yamlContent, setYamlContent] = useState("");
  const [originalContent, setOriginalContent] = useState("");
  const [isLoading, setIsLoading] = useState(false);
  const [isSaving, setIsSaving] = useState(false);
  const [hasChanges, setHasChanges] = useState(false);
  const [validationError, setValidationError] = useState<string | null>(null);

  // Mock YAML content
  const mockYamlContent = `apiVersion: v1
kind: Pod
metadata:
  name: ${podName}
  namespace: default
  labels:
    app: frontend
    version: v1.2.0
  annotations:
    kubernetes.io/created-by: deployment-controller
spec:
  containers:
  - name: nginx
    image: nginx:1.21
    ports:
    - containerPort: 80
      protocol: TCP
    resources:
      requests:
        memory: "64Mi"
        cpu: "50m"
      limits:
        memory: "128Mi"
        cpu: "100m"
    env:
    - name: ENV
      value: "production"
    - name: LOG_LEVEL
      value: "info"
    volumeMounts:
    - name: config-volume
      mountPath: /etc/nginx/conf.d
    livenessProbe:
      httpGet:
        path: /health
        port: 80
      initialDelaySeconds: 30
      periodSeconds: 10
    readinessProbe:
      httpGet:
        path: /ready
        port: 80
      initialDelaySeconds: 5
      periodSeconds: 5
  volumes:
  - name: config-volume
    configMap:
      name: nginx-config
  restartPolicy: Always
  dnsPolicy: ClusterFirst
  nodeSelector:
    kubernetes.io/os: linux
status:
  phase: Running
  conditions:
  - type: Initialized
    status: "True"
    lastTransitionTime: "2024-01-01T12:00:00Z"
  - type: Ready
    status: "True"
    lastTransitionTime: "2024-01-01T12:00:30Z"
  - type: ContainersReady
    status: "True"
    lastTransitionTime: "2024-01-01T12:00:30Z"
  - type: PodScheduled
    status: "True"
    lastTransitionTime: "2024-01-01T12:00:00Z"
  hostIP: "10.244.1.1"
  podIP: "10.244.1.15"
  startTime: "2024-01-01T12:00:00Z"`;

  // Load YAML when dialog opens
  useState(() => {
    if (open && !yamlContent) {
      setIsLoading(true);
      // Simulate API call
      setTimeout(() => {
        setYamlContent(mockYamlContent);
        setOriginalContent(mockYamlContent);
        setIsLoading(false);
      }, 1000);
    }
  });

  // Validate YAML syntax (basic validation)
  const validateYaml = (content: string): string | null => {
    try {
      // Basic YAML structure validation
      const lines = content.split('\n');
      let indentStack: number[] = [0];
      
      for (let i = 0; i < lines.length; i++) {
        const line = lines[i];
        if (line.trim() === '' || line.trim().startsWith('#')) continue;
        
        const indent = line.length - line.trimStart().length;
        const trimmed = line.trim();
        
        // Check for proper key-value format
        if (trimmed.includes(':') && !trimmed.startsWith('-')) {
          const [key] = trimmed.split(':');
          if (!key.trim()) {
            return `Line ${i + 1}: Invalid key format`;
          }
        }
        
        // Check for proper list format
        if (trimmed.startsWith('- ') && trimmed.length < 3) {
          return `Line ${i + 1}: Empty list item`;
        }
      }
      
      // Check for required fields
      if (!content.includes('apiVersion:')) {
        return 'Missing required field: apiVersion';
      }
      if (!content.includes('kind:')) {
        return 'Missing required field: kind';
      }
      if (!content.includes('metadata:')) {
        return 'Missing required field: metadata';
      }
      
      return null;
    } catch (error) {
      return `Invalid YAML syntax: ${error instanceof Error ? error.message : 'Unknown error'}`;
    }
  };

  const handleContentChange = (newContent: string) => {
    setYamlContent(newContent);
    setHasChanges(newContent !== originalContent);
    
    // Validate on change with debounce
    const error = validateYaml(newContent);
    setValidationError(error);
  };

  const handleSave = async () => {
    const error = validateYaml(yamlContent);
    if (error) {
      setValidationError(error);
      return;
    }

    setIsSaving(true);
    // Simulate API call
    await new Promise(resolve => setTimeout(resolve, 2000));
    
    setIsSaving(false);
    setOriginalContent(yamlContent);
    setHasChanges(false);
    setValidationError(null);
    
    // Show success message (you could use a toast here)
    alert('Pod configuration updated successfully!');
  };

  const handleReset = () => {
    setYamlContent(originalContent);
    setHasChanges(false);
    setValidationError(null);
  };

  const handleDownload = () => {
    const blob = new Blob([yamlContent], { type: 'text/yaml' });
    const url = URL.createObjectURL(blob);
    const link = document.createElement('a');
    link.href = url;
    link.download = `${podName}.yaml`;
    document.body.appendChild(link);
    link.click();
    document.body.removeChild(link);
    URL.revokeObjectURL(url);
  };

  const lineCount = yamlContent.split('\n').length;

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="w-[75vw] min-w-[800px] max-w-6xl h-[45vh] min-h-[400px] max-h-[650px] flex flex-col bg-card/95 backdrop-blur-sm">
        <DialogHeader className="flex-shrink-0">
          <div className="flex items-center justify-between">
            <div>
              <DialogTitle className="text-foreground">Edit Pod Configuration</DialogTitle>
              <DialogDescription>
                Modify YAML configuration for <span className="font-semibold text-primary">{podName}</span>
              </DialogDescription>
            </div>
            <div className="flex items-center gap-2">
              {hasChanges && (
                <Badge variant="outline" className="text-xs">
                  <AlertTriangle className="w-3 h-3 mr-1" />
                  Unsaved changes
                </Badge>
              )}
              <Badge variant="secondary" className="text-xs">
                {lineCount} lines
              </Badge>
            </div>
          </div>
        </DialogHeader>

        {validationError && (
          <Alert className="border-destructive/50 text-destructive">
            <AlertTriangle className="h-4 w-4" />
            <AlertDescription>
              {validationError}
            </AlertDescription>
          </Alert>
        )}

        <div className="flex items-center gap-2 py-2 border-b border-border/20">
          <Button
            variant="default"
            size="sm"
            onClick={handleSave}
            disabled={!hasChanges || !!validationError || isSaving}
            className="h-8"
          >
            {isSaving ? (
              <>
                <div className="w-3 h-3 border-2 border-white border-t-transparent rounded-full animate-spin mr-1" />
                Saving...
              </>
            ) : (
              <>
                <Save className="w-3 h-3 mr-1" />
                Save
              </>
            )}
          </Button>
          <Button
            variant="outline"
            size="sm"
            onClick={handleReset}
            disabled={!hasChanges}
            className="h-8"
          >
            <RotateCcw className="w-3 h-3 mr-1" />
            Reset
          </Button>
          <Button
            variant="outline"
            size="sm"
            onClick={handleDownload}
            className="h-8"
          >
            <Download className="w-3 h-3 mr-1" />
            Download
          </Button>
          <div className="flex-1" />
          {!validationError && yamlContent && (
            <div className="flex items-center gap-1 text-xs text-emerald-600">
              <CheckCircle className="w-3 h-3" />
              Valid YAML
            </div>
          )}
        </div>

        <div className="flex-1 min-h-0">
          {isLoading ? (
            <div className="flex items-center justify-center h-full">
              <div className="flex items-center gap-2 text-muted-foreground">
                <div className="w-4 h-4 border-2 border-primary border-t-transparent rounded-full animate-spin" />
                Loading YAML configuration...
              </div>
            </div>
          ) : (
            <div className="h-full flex">
              {/* Line numbers */}
              <div className="w-12 bg-muted/20 border-r border-border/20 p-2 font-mono text-xs text-muted-foreground text-right">
                {yamlContent.split('\n').map((_, index) => (
                  <div key={index} className="leading-6">
                    {index + 1}
                  </div>
                ))}
              </div>
              
              {/* Editor */}
              <div className="flex-1">
                <Textarea
                  value={yamlContent}
                  onChange={(e) => handleContentChange(e.target.value)}
                  className="h-full w-full border-none resize-none font-mono text-sm leading-6 bg-transparent focus:ring-0"
                  placeholder="Loading..."
                  spellCheck={false}
                />
              </div>
            </div>
          )}
        </div>

        <DialogFooter className="flex-shrink-0">
          <Button variant="outline" onClick={() => onOpenChange(false)}>
            Cancel
          </Button>
          <Button 
            onClick={handleSave}
            disabled={!hasChanges || !!validationError || isSaving}
          >
            {isSaving ? 'Saving...' : 'Apply Changes'}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}