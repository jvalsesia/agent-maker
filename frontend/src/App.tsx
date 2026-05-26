import { BrowserRouter } from "react-router-dom";
import { QueryClientProvider } from "@tanstack/react-query";
import { Toaster } from "sonner";
import { queryClient } from "@/lib/queryClient";
import { AppRoutes } from "@/router";
import "@/i18n"; // initialize react-i18next before any component renders

export default function App() {
  return (
    <QueryClientProvider client={queryClient}>
      <BrowserRouter>
        <AppRoutes />
        <Toaster richColors position="bottom-right" />
      </BrowserRouter>
    </QueryClientProvider>
  );
}
