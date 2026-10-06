import { useState } from "react";

export default function ArtworkBackdrop({
  artwork,
}: {
  artwork?: string | null;
}) {
  const [failedArtwork, setFailedArtwork] = useState<string | null>(null);
  return (
    <div
      aria-hidden="true"
      className={`pointer-events-none h fixed inset-0 -z-10 bg-linear-to-br from-muted to-primary/20`}
    >
      {artwork && artwork !== failedArtwork && (
        <img
          src={artwork}
          alt=""
          onError={() => setFailedArtwork(artwork)}
          className="size-full scale-125 object-cover opacity-40 blur-3xl"
        />
      )}
      <div className={`absolute inset-0  "bg-background/25" `} />
    </div>
  );
}
