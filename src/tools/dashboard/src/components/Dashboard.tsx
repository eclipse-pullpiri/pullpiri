import { useEffect, useState } from "react";
import { Sidebar } from "./Sidebar";
import { Header } from "./Header";
import { Overview } from "./Overview";
import { Workloads } from "./Workloads";
import { Services } from "./Services";
import { Storage } from "./Storage";
import { Cluster } from "./Cluster";
import { Scenarios } from "./Scenarios";
import { PodDetail } from "./PodDetail";

type View =
  | "overview"
  | "workloads"
  | "services"
  | "storage"
  | "cluster"
  | "scenarios"
  | "pod-detail";

// Pod interface
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

export function Dashboard() {
  const [currentView, setCurrentView] = useState<View>("workloads");
  const [sidebarCollapsed, setSidebarCollapsed] = useState(false);
  const [selectedPodName, setSelectedPodName] = useState<string>("");

  // Move pods state to Dashboard level

  // Lgsi pod data
  const [podsToUse, setPods] = useState<Pod[]>([]);
  const [podsFetchSuccess, setPodsFetchSuccess] = useState(false);

  useEffect(() => {
    const settingserviceApiUrl = import.meta.env.VITE_SETTING_SERVICE_API_URL;
    if (!settingserviceApiUrl) return;
    fetch(`${settingserviceApiUrl}/api/v1/metrics`)
      .then(res => res.json())
      .then(data => {
        // Filter for containers only
        const containers = Array.isArray(data)
          ? data.filter(
              (item: any) =>
                item.component === "container" &&
                item.metric_type === "ContainerInfo" &&
                item.value &&
                item.value.value
            )
          : [];
        if (containers.length > 0) {
          // Map API data to Pod[]
          const mappedPods = containers.map((item: any, idx: number) => {
            const val = item.value.value;
            return {
              name: (val.names && val.names[0]) || val.id || `pod-${idx}`,
              image: val.image || "",
              labels: item.labels || {},
              node: (val.state && (val.state.node_name || val.state.hostname)) || (val.config.Hostname) || "",
              status: (val.state && (val.state.status || val.state.Status)) || "",
              cpuUsage: val.stats?.CpuTotalUsage || "",
              memoryUsage: val.stats?.MemoryUsage || "",
              age: item.timestamp || "",
              ready: "", // Not available in this API, leave blank or compute if possible
              restarts: 0, // Not available, default to 0
              ip: item.labels?.ip || "",
            };
          });
          setPods(mappedPods);
          setPodsFetchSuccess(true);
          console.log("✅ Pods API (metrics) fetched:", mappedPods);
        } else {
          setPodsFetchSuccess(false);
        }
      })
      .catch(e => {
        setPodsFetchSuccess(false);
        console.error("❌ Pods API (metrics) fetch failed:", e);
      });
  }, []);


  const pods = podsFetchSuccess && podsToUse.length > 0
    ? podsToUse
    : [
        {
          name: "frontend-app-7d4b8c9f8d-xyz12",
          image: "nginx:1.21",
          labels: { app: "frontend", version: "v1.2.0" },
          node: "worker-node-1",
          status: "Running",
          cpuUsage: "45m",
          memoryUsage: "128Mi",
          age: "2d",
          ready: "1/1",
          restarts: 0,
          ip: "10.244.1.15",
        },
        {
          name: "frontend-app-7d4b8c9f8d-abc34",
          image: "nginx:1.21",
          labels: { app: "frontend", version: "v1.2.0" },
          node: "worker-node-2",
          status: "Running",
          cpuUsage: "38m",
          memoryUsage: "115Mi",
          age: "2d",
          ready: "1/1",
          restarts: 0,
          ip: "10.244.2.18",
        },
        {
          name: "backend-api-5f6a7b8c9d-def56",
          image: "node:18-alpine",
          labels: { app: "backend", tier: "api" },
          node: "worker-node-1",
          status: "Running",
          cpuUsage: "120m",
          memoryUsage: "256Mi",
          age: "5d",
          ready: "1/1",
          restarts: 1,
          ip: "10.244.1.22",
        },
        {
          name: "redis-cache-8e9f0a1b2c-ghi78",
          image: "redis:7-alpine",
          labels: { app: "redis", role: "cache" },
          node: "worker-node-3",
          status: "Running",
          cpuUsage: "25m",
          memoryUsage: "64Mi",
          age: "1d",
          ready: "1/1",
          restarts: 0,
          ip: "10.244.3.9",
        },
        {
          name: "database-migration-1a2b3c4d5e-jkl90",
          image: "postgres:14",
          labels: { job: "migration", app: "database" },
          node: "worker-node-2",
          status: "Pending",
          cpuUsage: "0m",
          memoryUsage: "0Mi",
          age: "30m",
          ready: "0/1",
          restarts: 0,
          ip: "N/A",
        },
      ];

  // Calculate running pods count
  const runningPodsCount = pods.filter(
    (pod) => pod.status === "Running"
  ).length;

  const handleViewChange = (view: View, podName?: string) => {
    setCurrentView(view);
    if (podName) {
      setSelectedPodName(podName);
    }
  };

  const renderContent = () => {
    switch (currentView) {
      case "overview":
        return <Overview />;
      case "workloads":
        return (
          <Workloads
            onPodClick={(podName) => handleViewChange("pod-detail", podName)}
            pods={pods}
            setPods={setPods}
          />
        );
      case "services":
        return <Services />;
      case "storage":
        return <Storage />;
      case "cluster":
        return <Cluster />;
      case "scenarios":
        return <Scenarios namespace="default" />; //2025-09-23 comment out
      case "pod-detail":
        const selectedPod = pods.find((pod) => pod.name === selectedPodName);
        return (
          <PodDetail
            podName={selectedPodName}
            podData={selectedPod}
            onBack={() => setCurrentView("workloads")}
          />
        );
      default:
        return <Overview />;
    }
  };

  return (
    <div className="min-h-screen bg-gradient-to-br from-background via-primary/5 to-chart-1/10 dark:from-background dark:via-background dark:to-chart-1/20 transition-colors duration-300">
      {/* Desktop Layout: 1024px and up */}
      <div className="hidden lg:flex h-screen">
        <Sidebar
          currentView={currentView === "pod-detail" ? "workloads" : currentView}
          onViewChange={setCurrentView}
          collapsed={sidebarCollapsed}
          onToggle={() => setSidebarCollapsed(!sidebarCollapsed)}
        />
        <div className="flex-1 flex flex-col min-w-0">
          <Header podCount={runningPodsCount} pods={pods} />
          <main className="flex-1 overflow-auto">
            <div className="h-full p-4 xl:p-6 2xl:p-8">
              <div className="max-w-none 2xl:max-w-[1600px] mx-auto h-full">
                {renderContent()}
              </div>
            </div>
          </main>
        </div>
      </div>

      {/* Tablet Layout: 768px to 1023px */}
      <div className="hidden md:flex lg:hidden h-screen">
        <Sidebar
          currentView={currentView === "pod-detail" ? "workloads" : currentView}
          onViewChange={setCurrentView}
          collapsed={true}
          onToggle={() => {}}
        />
        <div className="flex-1 flex flex-col min-w-0">
          <Header compact={true} podCount={runningPodsCount} pods={pods} />
          <main className="flex-1 overflow-auto">
            <div className="h-full p-4">
              <div className="max-w-none mx-auto h-full">{renderContent()}</div>
            </div>
          </main>
        </div>
      </div>

      {/* Mobile Layout: Below 768px */}
      <div className="flex md:hidden flex-col h-screen">
        <Header mobile={true} podCount={runningPodsCount} pods={pods} />
        <div className="flex-1 flex">
          <Sidebar
            currentView={
              currentView === "pod-detail" ? "workloads" : currentView
            }
            onViewChange={setCurrentView}
            mobile={true}
          />
          <main className="flex-1 overflow-auto">
            <div className="h-full p-3">
              <div className="max-w-none mx-auto h-full">{renderContent()}</div>
            </div>
          </main>
        </div>
      </div>
    </div>
  );
}
