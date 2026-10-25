"use client";

import Link from "next/link";
import { usePathname } from "next/navigation";

import { cn } from "@/lib/utils";

const LINKS = [
  { href: "/", label: "Dashboard" },
  { href: "/settings/stores", label: "Lojas" },
  { href: "/settings/targets", label: "Alvos" },
];

function NavbarView({ pathname }: { pathname: string | null }) {
  return (
    <header className="border-b border-zinc-200 bg-white">
      <nav className="mx-auto flex max-w-6xl items-center gap-6 px-4 py-3">
        <Link href="/" className="font-semibold text-zinc-900">
          Mac mini Tracker
        </Link>
        <ul className="flex gap-1 text-sm">
          {LINKS.map(({ href, label }) => {
            // "/" só ativa na raiz; as outras ativam também nas subpáginas.
            const active = pathname !== null && (href === "/" ? pathname === "/" : pathname.startsWith(href));
            return (
              <li key={href}>
                <Link
                  href={href}
                  aria-current={active ? "page" : undefined}
                  className={cn(
                    "rounded-md px-3 py-1.5 text-zinc-600 hover:bg-zinc-100 hover:text-zinc-900",
                    active && "bg-zinc-900 text-white hover:bg-zinc-900 hover:text-white",
                  )}
                >
                  {label}
                </Link>
              </li>
            );
          })}
        </ul>
      </nav>
    </header>
  );
}

export default function Navbar() {
  return <NavbarView pathname={usePathname()} />;
}

/** Fallback do Suspense no layout: mesma barra, sem item ativo (o pathname ainda não resolveu). */
export function NavbarFallback() {
  return <NavbarView pathname={null} />;
}
