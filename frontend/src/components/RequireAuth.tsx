import { Navigate } from "react-router-dom";
import { SignedIn, SignedOut } from "@clerk/clerk-react";
import { clerkEnabled } from "@/lib/clerk";

/**
 * Route guard (F10). When Clerk is enabled, renders children only for a signed-in
 * session and redirects signed-out visitors to `/sign-in`. When Clerk is disabled
 * (no publishable key), passes through so local dev needs no Clerk account.
 */
export function RequireAuth({ children }: { children: React.ReactNode }) {
  if (!clerkEnabled) return <>{children}</>;
  return (
    <>
      <SignedIn>{children}</SignedIn>
      <SignedOut>
        <Navigate to="/sign-in" replace />
      </SignedOut>
    </>
  );
}
