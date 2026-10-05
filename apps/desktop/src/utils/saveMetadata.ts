import type { DownloadedArtwork } from "./itunesClient";

export type TextChange =
  | { operation: "replace"; values: { type: "Text"; value: string }[] }
  | { operation: "delete" };
export type StoredArtwork = {
  file_id: number;
  mime_type: string;
  data_base64: string;
  picture_type: number;
  description: string;
};
export type SaveResult = {
  updatedFileIds: number[];
  failures: { fileId: number; path: string; message: string }[];
};
type PictureChange = {
  operation: "replace";
  values: { type: "Picture"; value: DownloadedArtwork }[];
};
export type SaveInput = {
  fileIds: number[];
  changes: Record<string, TextChange | PictureChange>;
};

export async function saveMetadataWithCover(
  fileIds: number[],
  changes: Record<string, TextChange>,
  image: DownloadedArtwork | undefined,
  readArtwork: (ids: number[]) => Promise<StoredArtwork[]>,
  write: (input: SaveInput) => Promise<SaveResult>,
): Promise<SaveResult> {
  if (!image) return write({ fileIds, changes });
  const existing = await readArtwork(fileIds);
  const result: SaveResult = { updatedFileIds: [], failures: [] };
  for (const fileId of fileIds) {
    const images = [
      image,
      ...existing
        .filter((item) => item.file_id === fileId && item.picture_type !== 3)
        .map((item) => ({
          mime: item.mime_type,
          data_base64: item.data_base64,
          picture_type: item.picture_type,
          description: item.description,
        })),
    ];
    try {
      const saved = await write({
        fileIds: [fileId],
        changes: {
          ...changes,
          attachedPicture: {
            operation: "replace",
            values: images.map((value) => ({ type: "Picture", value })),
          },
        },
      });
      result.updatedFileIds.push(...saved.updatedFileIds);
      result.failures.push(...saved.failures);
    } catch (error) {
      result.failures.push({ fileId, path: "", message: String(error) });
    }
  }
  return result;
}
