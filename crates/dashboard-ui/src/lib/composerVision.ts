export type VisionAttachment = {
  mime_type: string;
  data_base64: string;
  previewUrl: string;
};

export const MAX_IMAGE_BYTES = 4 * 1024 * 1024;
export const MAX_VISION_IMAGES = 3;
export const IMAGE_ACCEPT = "image/*";

export function isImageFile(file: File): boolean {
  if (file.type.startsWith("image/")) return true;
  return /\.(png|jpe?g|gif|webp|bmp|heic|heif)$/i.test(file.name);
}

export async function fileToVisionAttachment(file: File): Promise<VisionAttachment> {
  const buf = await file.arrayBuffer();
  const bytes = new Uint8Array(buf);
  let binary = "";
  for (let i = 0; i < bytes.length; i += 1) {
    binary += String.fromCharCode(bytes[i]!);
  }
  return {
    mime_type: file.type || "image/jpeg",
    data_base64: btoa(binary),
    previewUrl: URL.createObjectURL(file),
  };
}

export function visionAttachmentFromBase64(
  mime_type: string,
  data_base64: string,
): VisionAttachment {
  const binary = atob(data_base64);
  const bytes = new Uint8Array(binary.length);
  for (let i = 0; i < binary.length; i += 1) {
    bytes[i] = binary.charCodeAt(i);
  }
  const blob = new Blob([bytes], { type: mime_type || "image/png" });
  return {
    mime_type: mime_type || "image/png",
    data_base64,
    previewUrl: URL.createObjectURL(blob),
  };
}

export function revokeVisionAttachments(images: VisionAttachment[]): void {
  images.forEach((img) => URL.revokeObjectURL(img.previewUrl));
}

export function visionPayloadsForApi(
  images: VisionAttachment[],
): { mime_type: string; data_base64: string }[] {
  return images.map(({ mime_type, data_base64 }) => ({ mime_type, data_base64 }));
}
