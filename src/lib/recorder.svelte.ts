/**
 * Audio capture. Nothing here is available outside a secure context: in plain
 * HTTP `getUserMedia` and `MediaRecorder` are simply absent from `window`, so
 * there is no fallback to write — only a disabled button and an explanation.
 */

/** Ordered by preference; Safari only ever accepts the last one. */
const CANDIDATE_MIMES = ['audio/webm;codecs=opus', 'audio/webm', 'audio/mp4'];

export function audioSupport(): { ok: boolean; reason: string } {
  if (typeof window === 'undefined') return { ok: false, reason: '' };
  if (!window.isSecureContext) {
    return { ok: false, reason: 'Enregistrement indisponible : connexion non sécurisée' };
  }
  if (!navigator.mediaDevices?.getUserMedia || !window.MediaRecorder) {
    return { ok: false, reason: 'Enregistrement non supporté par ce navigateur' };
  }
  if (!CANDIDATE_MIMES.some(m => MediaRecorder.isTypeSupported(m))) {
    return { ok: false, reason: 'Aucun format audio supporté par ce navigateur' };
  }
  return { ok: true, reason: '' };
}

export type RecorderState = 'idle' | 'requesting' | 'recording' | 'error';

export class AudioRecorder {
  state = $state<RecorderState>('idle');
  seconds = $state(0);
  error = $state('');

  #recorder: MediaRecorder | null = null;
  #stream: MediaStream | null = null;
  #chunks: Blob[] = [];
  #timer: ReturnType<typeof setInterval> | null = null;
  #done: ((r: { blob: Blob; mime: string } | null) => void) | null = null;

  /** Resolves with the recording, or null if it was refused or cancelled. */
  async start(): Promise<{ blob: Blob; mime: string } | null> {
    const support = audioSupport();
    if (!support.ok) {
      this.state = 'error';
      this.error = support.reason;
      return null;
    }

    this.state = 'requesting';
    this.error = '';
    try {
      // Asked for on an explicit click, never ahead of time.
      this.#stream = await navigator.mediaDevices.getUserMedia({ audio: true });
    } catch (e) {
      const name = (e as DOMException)?.name;
      this.error = name === 'NotFoundError'
        ? 'Aucun micro détecté'
        : 'Micro refusé';
      this.state = 'error';
      return null;
    }

    const mime = CANDIDATE_MIMES.find(m => MediaRecorder.isTypeSupported(m))!;
    this.#recorder = new MediaRecorder(this.#stream, { mimeType: mime });
    this.#chunks = [];
    this.#recorder.ondataavailable = (e) => {
      if (e.data.size > 0) this.#chunks.push(e.data);
    };

    const promise = new Promise<{ blob: Blob; mime: string } | null>(resolve => {
      this.#done = resolve;
    });

    this.#recorder.onstop = () => {
      // The mime actually produced, read back from the recorder rather than
      // assumed: Chrome gives webm/opus, Safari mp4, and the difference has to
      // reach the database.
      const actual = this.#recorder?.mimeType || mime;
      const blob = new Blob(this.#chunks, { type: actual });
      this.#cleanup();
      this.#done?.(blob.size > 0 ? { blob, mime: actual } : null);
      this.#done = null;
    };

    this.#recorder.start();
    this.state = 'recording';
    this.seconds = 0;
    this.#timer = setInterval(() => { this.seconds += 1; }, 1000);
    return promise;
  }

  stop() {
    if (this.state === 'recording') this.#recorder?.stop();
  }

  cancel() {
    if (this.state === 'recording') {
      this.#chunks = [];
      this.#recorder?.stop();
    } else {
      this.#cleanup();
    }
  }

  #cleanup() {
    if (this.#timer) { clearInterval(this.#timer); this.#timer = null; }
    // Releasing the tracks is what turns the browser's recording indicator off.
    this.#stream?.getTracks().forEach(t => t.stop());
    this.#stream = null;
    this.#recorder = null;
    this.state = 'idle';
  }
}

export function formatDuration(total: number): string {
  const m = Math.floor(total / 60);
  const s = total % 60;
  return `${String(m).padStart(2, '0')}:${String(s).padStart(2, '0')}`;
}
