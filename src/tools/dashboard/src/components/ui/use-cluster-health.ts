import { useMemo } from 'react';

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

interface Node {
  name: string;
  pods: number;
  status: string;
  cpu: string;
  memory: string;
}

interface ClusterHealth {
  status: "Healthy" | "Warning" | "Critical";
  color: string;
  bgColor: string;
  dotColor: string;
  borderColor: string;
  bgGradient: string;
  runningPods: number;
  pendingPods: number;
  failedPods: number;
  totalPods: number;
  runningPodPercentage: number;
  healthyNodeCount: number;
  totalNodeCount: number;
  nodeHealthPercentage: number;
  nodesData: Node[];
}

export function useClusterHealth(pods: Pod[]): ClusterHealth {
  return useMemo(() => {
    // Mock nodes data (should match Workloads.tsx)
    const nodesData: Node[] = [
      { name: 'worker-node-1', pods: 2, status: 'Ready', cpu: '2.4/4', memory: '3.2/8' },
      { name: 'worker-node-2', pods: 2, status: 'Ready', cpu: '1.8/4', memory: '2.1/8' },
      { name: 'worker-node-3', pods: 1, status: 'Ready', cpu: '0.8/4', memory: '1.5/8' }
    ];

    // Calculate cluster metrics
    const runningPods = pods.filter(pod => pod.status === "Running").length;
    const pendingPods = pods.filter(pod => pod.status === "Pending").length;
    const failedPods = pods.filter(pod => pod.status === "Failed").length;
    const totalPods = pods.length;
    const runningPodPercentage = totalPods > 0 ? (runningPods / totalPods) * 100 : 100;
    const healthyNodeCount = nodesData.filter(node => node.status === 'Ready').length;
    const totalNodeCount = nodesData.length;
    const nodeHealthPercentage = totalNodeCount > 0 ? (healthyNodeCount / totalNodeCount) * 100 : 100;

    // Determine cluster health status
    // Critical: Failed pods > 20% or any nodes down
    if (failedPods > totalPods * 0.2 || nodeHealthPercentage < 100) {
      return {
        status: "Critical",
        color: "text-red-600 dark:text-red-400",
        bgColor: "bg-red-500",
        dotColor: "bg-red-500",
        borderColor: "border-red-200/20 dark:border-red-800/20",
        bgGradient: "from-red-500/10 to-red-600/10",
        runningPods,
        pendingPods,
        failedPods,
        totalPods,
        runningPodPercentage,
        healthyNodeCount,
        totalNodeCount,
        nodeHealthPercentage,
        nodesData
      };
    }
    // Warning: Pending pods > 10% or running pods < 90%
    else if (pendingPods > totalPods * 0.1 || runningPodPercentage < 90) {
      return {
        status: "Warning",
        color: "text-amber-600 dark:text-amber-400",
        bgColor: "bg-amber-500",
        dotColor: "bg-amber-500",
        borderColor: "border-amber-200/20 dark:border-amber-800/20",
        bgGradient: "from-amber-500/10 to-amber-600/10",
        runningPods,
        pendingPods,
        failedPods,
        totalPods,
        runningPodPercentage,
        healthyNodeCount,
        totalNodeCount,
        nodeHealthPercentage,
        nodesData
      };
    }
    // Healthy: All systems operational
    else {
      return {
        status: "Healthy",
        color: "text-emerald-600 dark:text-emerald-400",
        bgColor: "bg-emerald-500",
        dotColor: "bg-emerald-500",
        borderColor: "border-emerald-200/20 dark:border-emerald-800/20",
        bgGradient: "from-emerald-500/10 to-emerald-600/10",
        runningPods,
        pendingPods,
        failedPods,
        totalPods,
        runningPodPercentage,
        healthyNodeCount,
        totalNodeCount,
        nodeHealthPercentage,
        nodesData
      };
    }
  }, [pods]);
}