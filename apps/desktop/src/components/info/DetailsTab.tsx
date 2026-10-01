import { FilesResponse } from "../../hooks/useFileWatcher";
import { useMediaFiles } from "../../hooks/useMediaFiles";

interface Props {
  files: FilesResponse | undefined;
}
export default function DetailsTab({
  files,
}: {
  files: FilesResponse | undefined;
}) {
  if (!files) {
    console.log("j");
    return <></>;
  }

  console.log("hi");
  const editableKeys = ["title", "album", "artist"];
  return <>{JSON.stringify(files)} </>;
}
