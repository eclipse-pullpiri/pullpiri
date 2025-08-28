import { Dashboard } from "./components/Dashboard";
import { ThemeProvider } from "./components/ThemeProvider";

export default function App() {
  return (
    <ThemeProvider>
      <div className="min-h-screen bg-background">
        <Dashboard />
      </div>
    </ThemeProvider>
  );
}