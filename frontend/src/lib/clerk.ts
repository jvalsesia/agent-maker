/**
 * Clerk client configuration (F10). Auth is active only when a publishable key
 * is provided via `VITE_CLERK_PUBLISHABLE_KEY`; absent → the login wall is
 * disabled and the app renders directly, mirroring the backend's disabled mode.
 */
export const clerkPublishableKey: string | undefined = import.meta.env
  .VITE_CLERK_PUBLISHABLE_KEY as string | undefined;

export const clerkEnabled = Boolean(clerkPublishableKey);
