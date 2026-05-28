import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { authFetch } from "@/lib/authFetch";

export interface MeResponse {
  email: string;
  roles: string[];
}

export class AuthError extends Error {
  constructor(public status: number, public code: string) {
    super(code);
  }
}

async function fetchMe(): Promise<MeResponse | null> {
  const res = await authFetch("/auth/me");
  if (res.status === 401) return null;
  if (!res.ok) throw new AuthError(res.status, "me_failed");
  return (await res.json()) as MeResponse;
}

async function postLogin(body: { email: string; password: string }): Promise<MeResponse> {
  const res = await fetch("/auth/login", {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: JSON.stringify(body),
    credentials: "same-origin",
  });
  if (res.ok) return (await res.json()) as MeResponse;
  let code = "invalid_credentials";
  try {
    const data = await res.json();
    if (typeof data?.error === "string") code = data.error;
  } catch {
    // ignore
  }
  throw new AuthError(res.status, code);
}

async function postLogout(): Promise<void> {
  await fetch("/auth/logout", { method: "POST", credentials: "same-origin" });
}

export function useMe() {
  return useQuery({
    queryKey: ["me"],
    queryFn: fetchMe,
    staleTime: 30_000,
    retry: false,
  });
}

export function useLogin() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: postLogin,
    onSuccess: (me) => {
      qc.setQueryData(["me"], me);
    },
  });
}

export function useLogout() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: postLogout,
    onSuccess: () => {
      qc.setQueryData(["me"], null);
      qc.clear();
    },
  });
}
