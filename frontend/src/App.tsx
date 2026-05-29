import { BrowserRouter } from "react-router-dom";
import { QueryClientProvider } from "@tanstack/react-query";
import { ClerkProvider } from "@clerk/clerk-react";
import { Toaster } from "sonner";
import { queryClient } from "@/lib/queryClient";
import { clerkEnabled, clerkPublishableKey } from "@/lib/clerk";
import { AppRoutes } from "@/router";
import "@/i18n"; // initialize react-i18next before any component renders

export default function App() {
  const tree = (
    <QueryClientProvider client={queryClient}>
      <BrowserRouter>
        <AppRoutes />
        <Toaster richColors position="bottom-right" />
      </BrowserRouter>
    </QueryClientProvider>
  );

  // Mount Clerk only when configured; otherwise the app runs with auth disabled.
  if (!clerkEnabled) return tree;
  return (
    <ClerkProvider publishableKey={clerkPublishableKey!} afterSignOutUrl="/sign-in">
      {tree}
    </ClerkProvider>
  );
}
