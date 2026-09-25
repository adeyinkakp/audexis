import { createFileRoute } from "@tanstack/react-router";
import { RewindOverview } from "../../components/rewind/RewindOverview";
export const Route = createFileRoute("/_noneditor/rewind")({
  component: RewindPage,
});
function RewindPage() {
  return (
    <main className="h-[calc(100dvh-5.5rem)] overflow-auto px-6 pb-10 pt-7 lg:px-9">
      <RewindOverview />
    </main>
  );
}
