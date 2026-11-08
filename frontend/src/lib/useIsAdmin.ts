"use client";

import { useEffect, useState } from "react";

import { getMe } from "./api";

/** `false` até a resposta chegar: assim os controles de admin não piscam para quem só pode ler. */
export function useIsAdmin() {
  const [admin, setAdmin] = useState(false);
  useEffect(() => {
    let alive = true;
    getMe()
      .then((me) => alive && setAdmin(me.admin))
      .catch(() => {});
    return () => {
      alive = false;
    };
  }, []);
  return admin;
}
