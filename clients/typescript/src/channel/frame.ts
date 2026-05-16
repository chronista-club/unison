/**
 * Stream channel の wire frame (= Phase 2c)。
 *
 * `UnisonChannel` (= QUIC bidi stream) を流れる length-prefixed frame の
 * encode/decode。 1 frame の layout:
 *
 * ```text
 * [4B BE bodyLen] [2B BE headerLen] [JSON header] [codec-encoded payload]
 * ```
 *
 * `bodyLen` = headerLen field + header + payload の合計。 header は protocol-level
 * の metadata (`id` / `method` / `type`)、 payload は channel codec が encode した
 * application message。 protocol header を JSON 固定にすることで payload codec
 * (JSON / proto) と独立させる (= Rust 側 `ProtocolMessage` 相当の責務分離)。
 */

/** frame の protocol-level header (= payload codec とは独立、 常に JSON) */
export interface FrameHeader {
  /** request/response 相関 ID (= event は 0) */
  id: number;
  /** request / event 名 (= KDL schema 上の名前) */
  method: string;
  /** メッセージ種別 */
  type: "request" | "response" | "event" | "error";
}

const textEncoder = new TextEncoder();
const textDecoder = new TextDecoder("utf-8", { fatal: true });

/** header + codec-encoded payload を 1 本の length-prefixed frame に encode */
export function encodeFrame(header: FrameHeader, payload: Uint8Array): Uint8Array {
  const headerBytes = textEncoder.encode(JSON.stringify(header));
  const bodyLen = 2 + headerBytes.length + payload.length;
  const frame = new Uint8Array(4 + bodyLen);
  const view = new DataView(frame.buffer);
  view.setUint32(0, bodyLen, false);
  view.setUint16(4, headerBytes.length, false);
  frame.set(headerBytes, 6);
  frame.set(payload, 6 + headerBytes.length);
  return frame;
}

/** 1 frame の body (= 4B length prefix を剥がした後) を header + payload に分解 */
export function decodeFrameBody(
  body: Uint8Array,
): { header: FrameHeader; payload: Uint8Array } {
  if (body.length < 2) {
    throw new Error("frame body too short for header length field");
  }
  const headerLen = new DataView(
    body.buffer,
    body.byteOffset,
    body.byteLength,
  ).getUint16(0, false);
  if (body.length < 2 + headerLen) {
    throw new Error("frame body shorter than declared header length");
  }
  const header = JSON.parse(
    textDecoder.decode(body.subarray(2, 2 + headerLen)),
  ) as FrameHeader;
  return { header, payload: body.subarray(2 + headerLen) };
}

/**
 * `ReadableStream<Uint8Array>` から length-prefixed frame body を 1 本ずつ
 * 取り出す async generator。 byte 跨ぎ chunk を内部 buffer で結合する。
 */
export async function* readFrames(
  readable: ReadableStream<Uint8Array>,
): AsyncGenerator<Uint8Array> {
  const reader = readable.getReader();
  let buffer: Uint8Array = new Uint8Array(0);
  try {
    for (;;) {
      // length prefix (4B) が揃うまで読む
      while (buffer.length < 4) {
        const { value, done } = await reader.read();
        if (done) return;
        if (value !== undefined) buffer = concat(buffer, value);
      }
      const bodyLen = new DataView(
        buffer.buffer,
        buffer.byteOffset,
        buffer.byteLength,
      ).getUint32(0, false);
      // body 全体が揃うまで読む
      while (buffer.length < 4 + bodyLen) {
        const { value, done } = await reader.read();
        if (done) return;
        if (value !== undefined) buffer = concat(buffer, value);
      }
      yield buffer.subarray(4, 4 + bodyLen);
      buffer = buffer.slice(4 + bodyLen);
    }
  } finally {
    reader.releaseLock();
  }
}

function concat(
  a: Uint8Array<ArrayBufferLike>,
  b: Uint8Array<ArrayBufferLike>,
): Uint8Array {
  const out = new Uint8Array(a.length + b.length);
  out.set(a, 0);
  out.set(b, a.length);
  return out;
}
