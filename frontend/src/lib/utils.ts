import { clsx, type ClassValue } from "clsx";
import { twMerge } from "tailwind-merge";

import type { Money } from "./types";

export function cn(...inputs: ClassValue[]) {
  return twMerge(clsx(inputs));
}

const brl = new Intl.NumberFormat("pt-BR", { style: "currency", currency: "BRL" });

export function formatBRL(value: Money | number | null | undefined) {
  return value === null || value === undefined ? "—" : brl.format(Number(value));
}

const dateTime = new Intl.DateTimeFormat("pt-BR", { dateStyle: "short", timeStyle: "short" });

export function formatDateTime(value: string | number | null | undefined) {
  return value === null || value === undefined ? "—" : dateTime.format(new Date(value));
}

export const CHIPS = ["M5", "M5 Pro", "M6"] as const;
