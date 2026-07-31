import {
  imageFilesFromPasteEvent,
  pastedImageFromBase64,
} from "@/lib/clipboardImage";
import {
  formatTextAttachmentMeta,
  makePastedTextAttachment,
  MAX_TEXT_FILE_BYTES,
  MAX_TEXT_FILES,
  plainTextFromPasteEvent,
  shouldPasteAsTextCard,
  utf8ByteLength,
  type TextAttachment,
} from "@/lib/composerTextAttachment";
import {
  MAX_IMAGE_BYTES,
  MAX_VISION_IMAGES,
  visionAttachmentFromBase64,
  type VisionAttachment,
} from "@/lib/composerVision";
import { readApplePasteboard } from "@/lib/desktopShell";

export type ComposerPasteResult =
  | { kind: "ignored" }
  | { kind: "images"; images: VisionAttachment[]; error?: string }
  | { kind: "text-card"; file: TextAttachment; hint: string }
  | { kind: "error"; error: string };

type PasteHandlers = {
  /** Native vision OR OCR capability — may accept image paste. */
  canAttachImages: boolean;
  attachedImageCount: number;
  attachedTextFiles: TextAttachment[];
  locale: string;
  t: (key: string) => string;
  ingestImageFiles: (files: File[]) => Promise<void>;
};

/**
 * Shared composer paste: images → vision attachments; text >4000 chars → reference card.
 * Must call preventDefault synchronously before any await for long-text / browser images.
 */
export async function handleComposerPasteEvent(
  event: ClipboardEvent,
  handlers: PasteHandlers,
): Promise<ComposerPasteResult> {
  const {
    canAttachImages,
    attachedImageCount,
    attachedTextFiles,
    locale,
    t,
    ingestImageFiles,
  } = handlers;

  const imageFiles = imageFilesFromPasteEvent(event);
  if (imageFiles.length > 0) {
    event.preventDefault();
    await ingestImageFiles(imageFiles);
    return { kind: "ignored" };
  }

  // Long text must be gated before any await — otherwise default paste wins.
  const pastedText = plainTextFromPasteEvent(event);
  if (shouldPasteAsTextCard(pastedText)) {
    event.preventDefault();
    if (attachedTextFiles.length >= MAX_TEXT_FILES) {
      return { kind: "error", error: t("conversations.attachmentPasteLimit") };
    }
    if (utf8ByteLength(pastedText) > MAX_TEXT_FILE_BYTES) {
      return {
        kind: "error",
        error: t("conversations.attachmentTextTooLarge").replace("{name}", "paste.txt"),
      };
    }
    const file = makePastedTextAttachment(
      pastedText,
      attachedTextFiles.map((f) => f.filename),
    );
    return {
      kind: "text-card",
      file,
      hint: t("conversations.attachmentPasteAsCard").replace(
        "{n}",
        formatTextAttachmentMeta(pastedText, locale),
      ),
    };
  }

  // Short/empty text: optionally pull image from Apple pasteboard (desktop).
  if (pastedText.trim().length > 0) {
    return { kind: "ignored" };
  }

  event.preventDefault();
  const pbItems = await readApplePasteboard();
  const imageItem = pbItems.find((item) => item.kind === "image" && item.data_base64);
  if (!imageItem?.data_base64) {
    return { kind: "ignored" };
  }
  if (!canAttachImages) {
    return { kind: "error", error: t("conversations.attachmentVisionDisabled") };
  }
  if (attachedImageCount >= MAX_VISION_IMAGES) {
    return { kind: "ignored" };
  }
  const payload = pastedImageFromBase64(imageItem.mime_type, imageItem.data_base64);
  const approxBytes = Math.floor((payload.data_base64.length * 3) / 4);
  if (approxBytes > MAX_IMAGE_BYTES) {
    return {
      kind: "error",
      error: t("conversations.attachmentImageTooLarge").replace("{name}", "image"),
    };
  }
  return {
    kind: "images",
    images: [visionAttachmentFromBase64(payload.mime_type, payload.data_base64)],
  };
}
