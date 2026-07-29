/** Copy a raster image blob/URL to the system clipboard. */
export async function copyImageToClipboard(source: string | Blob): Promise<boolean> {
  if (typeof navigator === "undefined" || !navigator.clipboard?.write) {
    return false;
  }
  try {
    const blob =
      typeof source === "string"
        ? await fetch(source).then((res) => {
            if (!res.ok) throw new Error(`fetch failed: ${res.status}`);
            return res.blob();
          })
        : source;
    const mime = blob.type.startsWith("image/") ? blob.type : "image/png";
    if (typeof ClipboardItem !== "undefined" && ClipboardItem.supports?.(mime)) {
      await navigator.clipboard.write([new ClipboardItem({ [mime]: blob })]);
      return true;
    }
    if (typeof ClipboardItem !== "undefined" && ClipboardItem.supports?.("image/png")) {
      const png = mime === "image/png" ? blob : await blobToPng(blob);
      await navigator.clipboard.write([new ClipboardItem({ "image/png": png })]);
      return true;
    }
    return false;
  } catch {
    return false;
  }
}

async function blobToPng(blob: Blob): Promise<Blob> {
  if (typeof document === "undefined") {
    throw new Error("document unavailable");
  }
  const url = URL.createObjectURL(blob);
  try {
    const img = await loadImage(url);
    const canvas = document.createElement("canvas");
    canvas.width = img.naturalWidth || img.width;
    canvas.height = img.naturalHeight || img.height;
    const ctx = canvas.getContext("2d");
    if (!ctx) throw new Error("canvas unavailable");
    ctx.drawImage(img, 0, 0);
    const png = await new Promise<Blob>((resolve, reject) => {
      canvas.toBlob((b) => (b ? resolve(b) : reject(new Error("toBlob failed"))), "image/png");
    });
    return png;
  } finally {
    URL.revokeObjectURL(url);
  }
}

function loadImage(url: string): Promise<HTMLImageElement> {
  return new Promise((resolve, reject) => {
    const img = new Image();
    img.onload = () => resolve(img);
    img.onerror = () => reject(new Error("image load failed"));
    img.src = url;
  });
}

/** Images from a paste event (browser clipboard). */
export function imageFilesFromPasteEvent(event: ClipboardEvent): File[] {
  const files: File[] = [];
  const items = event.clipboardData?.items;
  if (!items) return files;
  for (let i = 0; i < items.length; i += 1) {
    const item = items[i];
    if (!item?.type.startsWith("image/")) continue;
    const file = item.getAsFile();
    if (file) files.push(file);
  }
  return files;
}

export type PastedImagePayload = {
  mime_type: string;
  data_base64: string;
};

/** Decode apple-media pasteboard image item. */
export function pastedImageFromBase64(
  mimeType: string | null | undefined,
  dataBase64: string,
): PastedImagePayload {
  return {
    mime_type: mimeType?.trim() || "image/png",
    data_base64: dataBase64,
  };
}
