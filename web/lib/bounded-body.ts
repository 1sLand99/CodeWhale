export class BodyReadError extends Error {
  constructor(readonly status: 400 | 413, message: string) {
    super(message);
    this.name = "BodyReadError";
  }
}

/** Count raw bytes while reading; Content-Length is only an early rejection. */
export async function readBoundedBody(request: Request, maxBytes: number): Promise<Uint8Array> {
  const reject = async (status: 400 | 413, message: string): Promise<never> => {
    try { await request.body?.cancel(message); } catch { /* Keep the original rejection. */ }
    throw new BodyReadError(status, message);
  };
  const rawLength = request.headers.get("content-length");
  if (rawLength !== null) {
    if (!/^\d+$/.test(rawLength)) return reject(400, "invalid Content-Length");
    if (Number(rawLength) > maxBytes) return reject(413, "payload too large");
  }
  if (!request.body) return new Uint8Array();

  const reader = request.body.getReader();
  const chunks: Uint8Array[] = [];
  let total = 0;
  try {
    while (true) {
      const { done, value } = await reader.read();
      if (done) break;
      total += value.byteLength;
      if (total > maxBytes) throw new BodyReadError(413, "payload too large");
      chunks.push(value);
    }
  } catch (cause) {
    try { await reader.cancel("body rejected"); } catch { /* Preserve the read/size error. */ }
    throw cause instanceof BodyReadError ? cause : new BodyReadError(400, "body read failed");
  } finally {
    reader.releaseLock();
  }
  const bytes = new Uint8Array(total);
  let offset = 0;
  for (const chunk of chunks) {
    bytes.set(chunk, offset);
    offset += chunk.byteLength;
  }
  return bytes;
}
