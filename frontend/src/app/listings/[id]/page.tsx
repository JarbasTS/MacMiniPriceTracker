import { Suspense } from "react";

import ListingDetailView from "./ListingDetailView";

// O id só é conhecido na requisição; com cacheComponents o useParams precisa de Suspense.
export default function ListingPage() {
  return (
    <Suspense fallback={<p className="text-sm text-zinc-500">Carregando…</p>}>
      <ListingDetailView />
    </Suspense>
  );
}
