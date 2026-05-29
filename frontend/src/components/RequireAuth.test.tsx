import { describe, expect, it, vi, beforeEach } from "vitest";
import { render, screen } from "@testing-library/react";
import { MemoryRouter, Route, Routes } from "react-router-dom";

// Toggle the simulated Clerk session per test.
const state = vi.hoisted(() => ({ signedIn: true }));

vi.mock("@/lib/clerk", () => ({ clerkEnabled: true, clerkPublishableKey: "pk_test" }));
vi.mock("@clerk/clerk-react", () => ({
  SignedIn: ({ children }: { children: React.ReactNode }) =>
    state.signedIn ? <>{children}</> : null,
  SignedOut: ({ children }: { children: React.ReactNode }) =>
    state.signedIn ? null : <>{children}</>,
}));

import { RequireAuth } from "./RequireAuth";

function renderGuard() {
  return render(
    <MemoryRouter initialEntries={["/"]}>
      <Routes>
        <Route path="/" element={<RequireAuth><div>PROTECTED</div></RequireAuth>} />
        <Route path="/sign-in" element={<div>SIGN-IN PAGE</div>} />
      </Routes>
    </MemoryRouter>,
  );
}

describe("RequireAuth guard (F10)", () => {
  beforeEach(() => {
    state.signedIn = true;
  });

  it("require_auth_renders_children_when_signed_in", () => {
    state.signedIn = true;
    renderGuard();
    expect(screen.getByText("PROTECTED")).toBeInTheDocument();
    expect(screen.queryByText("SIGN-IN PAGE")).not.toBeInTheDocument();
  });

  it("require_auth_redirects_when_signed_out", () => {
    state.signedIn = false;
    renderGuard();
    expect(screen.getByText("SIGN-IN PAGE")).toBeInTheDocument();
    expect(screen.queryByText("PROTECTED")).not.toBeInTheDocument();
  });
});
