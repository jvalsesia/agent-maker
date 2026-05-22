import { Outlet } from "react-router-dom";
import { NavSidebar } from "./NavSidebar";

export function Layout() {
  return (
    <div className="flex min-h-screen">
      <NavSidebar />
      <main className="flex-1 overflow-x-hidden p-6">
        <Outlet />
      </main>
    </div>
  );
}
