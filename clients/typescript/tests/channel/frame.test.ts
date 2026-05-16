import { describe, expect, it } from "vitest";
import {
  decodeFrameBody,
  encodeFrame,
  type FrameHeader,
  readFrames,
} from "../../src/channel/frame.js";

const payload = new TextEncoder().encode('{"x":1}');
const header: FrameHeader = { id: 3, method: "Ping", type: "request" };

describe("frame encode/decode", () => {
  it("round-trips a frame header and payload", () => {
    const frame = encodeFrame(header, payload);
    // strip the 4-byte length prefix to get the body
    const body = frame.subarray(4);
    const decoded = decodeFrameBody(body);
    expect(decoded.header).toEqual(header);
    expect([...decoded.payload]).toEqual([...payload]);
  });

  it("writes a big-endian u32 length prefix covering the body", () => {
    const frame = encodeFrame(header, payload);
    const bodyLen = new DataView(frame.buffer).getUint32(0, false);
    expect(bodyLen).toBe(frame.length - 4);
  });

  it("rejects a body shorter than its declared header length", () => {
    expect(() => decodeFrameBody(Uint8Array.from([0xff, 0xff, 0x00]))).toThrow();
  });
});

describe("readFrames", () => {
  /** chunks を 1 個ずつ流す ReadableStream を作る */
  function streamOf(chunks: Uint8Array[]): ReadableStream<Uint8Array> {
    let i = 0;
    return new ReadableStream({
      pull(controller) {
        if (i < chunks.length) controller.enqueue(chunks[i++]);
        else controller.close();
      },
    });
  }

  it("yields complete frame bodies from a single chunk", async () => {
    const frame = encodeFrame(header, payload);
    const bodies: Uint8Array[] = [];
    for await (const body of readFrames(streamOf([frame]))) bodies.push(body);
    expect(bodies).toHaveLength(1);
    expect(decodeFrameBody(bodies[0] as Uint8Array).header).toEqual(header);
  });

  it("reassembles a frame split across chunk boundaries", async () => {
    const frame = encodeFrame(header, payload);
    const chunks = [frame.subarray(0, 2), frame.subarray(2, 7), frame.subarray(7)];
    const bodies: Uint8Array[] = [];
    for await (const body of readFrames(streamOf(chunks))) bodies.push(body);
    expect(bodies).toHaveLength(1);
    expect(decodeFrameBody(bodies[0] as Uint8Array).header).toEqual(header);
  });

  it("yields multiple frames concatenated in one chunk", async () => {
    const a = encodeFrame({ id: 1, method: "A", type: "event" }, payload);
    const b = encodeFrame({ id: 2, method: "B", type: "event" }, payload);
    const merged = new Uint8Array(a.length + b.length);
    merged.set(a, 0);
    merged.set(b, a.length);
    const bodies: Uint8Array[] = [];
    for await (const body of readFrames(streamOf([merged]))) bodies.push(body);
    expect(bodies).toHaveLength(2);
  });
});
