type Props = {
  src: string;
  title: string;
  size?: "thumb" | "dialog";
  className?: string;
};

const SIZE_CLASS = {
  thumb: "w-full h-full border-0 bg-white pointer-events-none",
  dialog: "w-full h-[min(480px,70vh)] border-0 bg-white",
} as const;

export function DeliverableIframePreview({
  src,
  title,
  size = "dialog",
  className = "",
}: Props) {
  return (
    <iframe
      src={src}
      title={title}
      className={className || SIZE_CLASS[size]}
      sandbox="allow-scripts allow-same-origin"
    />
  );
}
