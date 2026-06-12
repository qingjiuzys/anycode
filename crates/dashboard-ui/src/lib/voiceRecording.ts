export type VoiceRecordingErrorCode =
  | "unsupported"
  | "permission_denied"
  | "no_recorder"
  | "empty_recording";

export class VoiceRecordingError extends Error {
  code: VoiceRecordingErrorCode;

  constructor(code: VoiceRecordingErrorCode, message: string) {
    super(message);
    this.code = code;
    this.name = "VoiceRecordingError";
  }
}

/** Reject accidental tap-and-stop before STT (Apple Speech / whisper). */
export const MIN_RECORDING_MS = 500;

type ActiveRecording = {
  recorder: MediaRecorder;
  stream: MediaStream;
  chunks: BlobPart[];
  mimeType: string;
  startedAt: number;
};

let active: ActiveRecording | null = null;

function pickMimeType(): string | undefined {
  if (typeof MediaRecorder === "undefined") return undefined;
  const candidates = [
    "audio/webm;codecs=opus",
    "audio/webm",
    "audio/mp4",
    "audio/mp4;codecs=mp4a",
    "audio/ogg;codecs=opus",
    "audio/ogg",
  ];
  return candidates.find((t) => MediaRecorder.isTypeSupported(t));
}

/** True when getUserMedia + MediaRecorder exist (mime type may be chosen at record time). */
export function isVoiceRecordingSupported(): boolean {
  return (
    typeof navigator !== "undefined" &&
    !!navigator.mediaDevices?.getUserMedia &&
    typeof MediaRecorder !== "undefined"
  );
}

export async function startRecording(): Promise<void> {
  if (!isVoiceRecordingSupported()) {
    throw new VoiceRecordingError("unsupported", "Voice recording is not supported in this browser");
  }
  if (active) {
    await stopRecording();
  }
  let stream: MediaStream;
  try {
    stream = await navigator.mediaDevices.getUserMedia({ audio: true });
  } catch (e) {
    const name = e instanceof DOMException ? e.name : "";
    if (name === "NotAllowedError" || name === "PermissionDeniedError") {
      throw new VoiceRecordingError("permission_denied", "Microphone permission denied");
    }
    throw new VoiceRecordingError("unsupported", e instanceof Error ? e.message : String(e));
  }
  const mimeType = pickMimeType();
  const recorder = mimeType
    ? new MediaRecorder(stream, { mimeType })
    : new MediaRecorder(stream);
  const chunks: BlobPart[] = [];
  recorder.ondataavailable = (ev) => {
    if (ev.data.size > 0) chunks.push(ev.data);
  };
  recorder.start(250);
  active = {
    recorder,
    stream,
    chunks,
    mimeType: mimeType ?? recorder.mimeType ?? "audio/mp4",
    startedAt: Date.now(),
  };
}

export function recordingElapsedMs(): number {
  if (!active) return 0;
  return Date.now() - active.startedAt;
}

export function isRecording(): boolean {
  return active !== null && active.recorder.state === "recording";
}

export async function stopRecording(): Promise<Blob> {
  const session = active;
  if (!session) {
    throw new VoiceRecordingError("no_recorder", "No active recording");
  }
  active = null;

  const blob = await new Promise<Blob>((resolve, reject) => {
    session.recorder.onstop = () => {
      const out = new Blob(session.chunks, { type: session.mimeType });
      if (out.size === 0) {
        reject(new VoiceRecordingError("empty_recording", "Recording was empty"));
        return;
      }
      resolve(out);
    };
    session.recorder.onerror = () => {
      reject(new VoiceRecordingError("unsupported", "Recording failed"));
    };
    if (session.recorder.state === "inactive") {
      const out = new Blob(session.chunks, { type: session.mimeType });
      if (out.size === 0) {
        reject(new VoiceRecordingError("empty_recording", "Recording was empty"));
      } else {
        resolve(out);
      }
      return;
    }
    session.recorder.stop();
  });

  for (const track of session.stream.getTracks()) {
    track.stop();
  }
  const elapsed = Date.now() - session.startedAt;
  if (elapsed < MIN_RECORDING_MS) {
    throw new VoiceRecordingError("empty_recording", "Recording too short");
  }
  return blob;
}

export function extensionForMime(mime: string): string {
  if (mime.includes("wav")) return "wav";
  if (mime.includes("mp4") || mime.includes("m4a")) return "m4a";
  if (mime.includes("ogg")) return "ogg";
  return "webm";
}

/** Encode mono f32 samples as 16-bit PCM WAV. */
export function encodeWav16kMono(samples: Float32Array, sampleRate = 16_000): Blob {
  const numChannels = 1;
  const bitsPerSample = 16;
  const blockAlign = (numChannels * bitsPerSample) / 8;
  const byteRate = sampleRate * blockAlign;
  const dataSize = samples.length * blockAlign;
  const buffer = new ArrayBuffer(44 + dataSize);
  const view = new DataView(buffer);

  const writeStr = (offset: number, str: string) => {
    for (let i = 0; i < str.length; i += 1) {
      view.setUint8(offset + i, str.charCodeAt(i));
    }
  };

  writeStr(0, "RIFF");
  view.setUint32(4, 36 + dataSize, true);
  writeStr(8, "WAVE");
  writeStr(12, "fmt ");
  view.setUint32(16, 16, true);
  view.setUint16(20, 1, true);
  view.setUint16(22, numChannels, true);
  view.setUint32(24, sampleRate, true);
  view.setUint32(28, byteRate, true);
  view.setUint16(32, blockAlign, true);
  view.setUint16(34, bitsPerSample, true);
  writeStr(36, "data");
  view.setUint32(40, dataSize, true);

  let offset = 44;
  for (let i = 0; i < samples.length; i += 1) {
    const s = Math.max(-1, Math.min(1, samples[i] ?? 0));
    view.setInt16(offset, s < 0 ? s * 0x8000 : s * 0x7fff, true);
    offset += 2;
  }

  return new Blob([buffer], { type: "audio/wav" });
}

/** Convert any browser audio blob to 16 kHz mono WAV for built-in whisper STT. */
export async function blobToWav16k(blob: Blob): Promise<Blob> {
  const arrayBuffer = await blob.arrayBuffer();
  const ctx = new AudioContext();
  try {
    const audioBuffer = await ctx.decodeAudioData(arrayBuffer.slice(0));
    const duration = audioBuffer.duration;
    const targetRate = 16_000;
    const offline = new OfflineAudioContext(1, Math.ceil(duration * targetRate), targetRate);
    const source = offline.createBufferSource();
    const mono = offline.createBuffer(1, audioBuffer.length, audioBuffer.sampleRate);
    const channel = mono.getChannelData(0);
    if (audioBuffer.numberOfChannels === 1) {
      channel.set(audioBuffer.getChannelData(0));
    } else {
      const left = audioBuffer.getChannelData(0);
      const right = audioBuffer.getChannelData(1);
      for (let i = 0; i < audioBuffer.length; i += 1) {
        channel[i] = (left[i]! + right[i]!) * 0.5;
      }
    }
    source.buffer = mono;
    source.connect(offline.destination);
    source.start(0);
    const rendered = await offline.startRendering();
    return encodeWav16kMono(rendered.getChannelData(0), targetRate);
  } finally {
    await ctx.close();
  }
}
