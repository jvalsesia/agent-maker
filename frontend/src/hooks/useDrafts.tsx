import { createContext, useCallback, useContext, useRef, useState, type ReactNode } from "react";

interface DraftsCtx {
  getDraft: (id: string) => string;
  setDraft: (id: string, value: string) => void;
}

const Ctx = createContext<DraftsCtx | null>(null);

export function DraftsProvider({ children }: { children: ReactNode }) {
  const map = useRef<Map<string, string>>(new Map());
  // bump on mutation so consumers reading via getDraft re-render where needed
  const [, force] = useState(0);

  const getDraft = useCallback((id: string) => map.current.get(id) ?? "", []);
  const setDraft = useCallback((id: string, value: string) => {
    map.current.set(id, value);
    force((n) => n + 1);
  }, []);

  return <Ctx.Provider value={{ getDraft, setDraft }}>{children}</Ctx.Provider>;
}

export function useDrafts(): DraftsCtx {
  const v = useContext(Ctx);
  if (!v) throw new Error("useDrafts must be used inside DraftsProvider");
  return v;
}
