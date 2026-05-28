import { describe, expect, it } from "vitest";
import { render, screen, waitFor } from "@testing-library/react";
import { Route, Routes } from "react-router-dom";
import { RequireAuth } from "@/components/auth/RequireAuth";
import { Wrap, jsonResponse, makeQueryClient, mockFetch } from "@/test-utils";

function renderProtected(initialPath = "/secret") {
  return render(
    <Wrap client={makeQueryClient()} initialEntries={[initialPath]}>
      <Routes>
        <Route path="/login" element={<div>login screen</div>} />
        <Route
          path="/secret"
          element={
            <RequireAuth>
              <div>protected content</div>
            </RequireAuth>
          }
        />
      </Routes>
    </Wrap>,
  );
}

describe("RequireAuth", () => {
  it("renders children when /auth/me returns the current user", async () => {
    mockFetch(async () => jsonResponse({ email: "a@b.c", roles: [] }));
    renderProtected();
    await waitFor(() =>
      expect(screen.getByText("protected content")).toBeTruthy(),
    );
  });

  it("redirects to /login when /auth/me returns 401", async () => {
    mockFetch(async () => new Response(null, { status: 401 }));
    renderProtected();
    await waitFor(() => expect(screen.getByText("login screen")).toBeTruthy());
  });
});
