/**
 * Stream channel wrapper (= Phase 2c)。
 *
 * QUIC bidi stream 上の request/response + server-pushed event。 内部で 1 本の
 * recv loop を持ち、 受信 frame を type tag で振り分ける:
 * - `response` / `error` → `id` 対応の pending request を resolve/reject
 * - `event` → events() の AsyncIterable queue に流す
 *
 * Rust `network/channel.rs` の `UnisonChannel` に対応する TS port。
 */

import type { Codec } from "../codec/codec.js";
import type { BidiStream } from "../transport/types.js";
import { defaultCodec } from "./default_codec.js";
import { AsyncQueue } from "./async_queue.js";
import { decodeFrameBody, encodeFrame, type FrameHeader, readFrames } from "./frame.js";
import type {
  ChannelMeta,
  ChannelPayload,
  EventName,
  RequestName,
  UnisonChannel,
} from "./types.js";

/** request() のデフォルト timeout (= Rust 側と同じ 30 秒) */
const DEFAULT_REQUEST_TIMEOUT_MS = 30_000;

/** 応答待ち request 1 件の resolver ペア */
interface PendingRequest {
  resolve(payload: ChannelPayload): void;
  reject(error: Error): void;
}

/**
 * `UnisonChannel` の concrete impl。 `openChannel(meta)` から構築する
 * (= caller は直接 new せず factory 経由)。
 */
export class UnisonChannelImpl<M extends ChannelMeta>
  implements UnisonChannel<M>
{
  readonly name: M["name"];

  readonly #stream: BidiStream;
  readonly #codec: Codec<ChannelPayload>;
  readonly #writer: WritableStreamDefaultWriter<Uint8Array>;
  /** id → 応答待ち request */
  readonly #pending = new Map<number, PendingRequest>();
  /** server push event の queue (= events() が配る) */
  readonly #events = new AsyncQueue<ChannelPayload>();
  /** recv loop の完了 promise */
  readonly #recvLoop: Promise<void>;
  #nextId = 1;
  #closed = false;

  /** @internal `openChannel` から呼ぶ。 */
  constructor(
    meta: M,
    stream: BidiStream,
    codec: Codec<ChannelPayload> = defaultCodec,
  ) {
    this.name = meta.name;
    this.#stream = stream;
    this.#codec = codec;
    this.#writer = stream.writable.getWriter();
    this.#recvLoop = this.#runRecvLoop();
  }

  /** 受信 frame を type tag で振り分ける background loop */
  async #runRecvLoop(): Promise<void> {
    try {
      for await (const body of readFrames(this.#stream.readable)) {
        let header: FrameHeader;
        let payload: Uint8Array;
        try {
          ({ header, payload } = decodeFrameBody(body));
        } catch {
          continue; // malformed frame は drop
        }
        if (header.type === "response" || header.type === "error") {
          const pending = this.#pending.get(header.id);
          if (pending === undefined) continue;
          this.#pending.delete(header.id);
          if (header.type === "error") {
            pending.reject(new Error(this.#errorText(payload)));
          } else {
            this.#tryResolve(pending, payload);
          }
        } else {
          // event / request → events queue
          this.#tryPushEvent(payload);
        }
      }
    } catch {
      // stream error は terminate 扱い
    } finally {
      this.#failAllPending("channel closed");
      this.#events.end();
    }
  }

  #tryResolve(pending: PendingRequest, payload: Uint8Array): void {
    try {
      pending.resolve(this.#codec.decode(payload));
    } catch (cause) {
      pending.reject(cause instanceof Error ? cause : new Error(String(cause)));
    }
  }

  #tryPushEvent(payload: Uint8Array): void {
    try {
      this.#events.push(this.#codec.decode(payload));
    } catch {
      // decode 不能 event は drop
    }
  }

  #errorText(payload: Uint8Array): string {
    try {
      return `channel "${this.name}" request error: ${JSON.stringify(this.#codec.decode(payload))}`;
    } catch {
      return `channel "${this.name}" request error`;
    }
  }

  #failAllPending(reason: string): void {
    for (const pending of this.#pending.values()) {
      pending.reject(new Error(reason));
    }
    this.#pending.clear();
  }

  async request(
    name: RequestName<M>,
    payload: ChannelPayload,
  ): Promise<ChannelPayload> {
    if (this.#closed) throw new Error(`channel "${this.name}" is closed`);
    const id = this.#nextId++;
    const frame = encodeFrame(
      { id, method: name, type: "request" },
      this.#codec.encode(payload),
    );
    const result = new Promise<ChannelPayload>((resolve, reject) => {
      this.#pending.set(id, { resolve, reject });
    });
    let timer: ReturnType<typeof setTimeout> | undefined;
    const timeout = new Promise<never>((_, reject) => {
      timer = setTimeout(() => {
        this.#pending.delete(id);
        reject(new Error(`request "${name}" timed out`));
      }, DEFAULT_REQUEST_TIMEOUT_MS);
    });
    try {
      await this.#writer.write(frame);
      return await Promise.race([result, timeout]);
    } finally {
      if (timer !== undefined) clearTimeout(timer);
    }
  }

  events(): AsyncIterableIterator<ChannelPayload> {
    return this.#events;
  }

  async sendEvent(name: EventName<M>, payload: ChannelPayload): Promise<void> {
    if (this.#closed) throw new Error(`channel "${this.name}" is closed`);
    await this.#writer.write(
      encodeFrame(
        { id: 0, method: name, type: "event" },
        this.#codec.encode(payload),
      ),
    );
  }

  async close(): Promise<void> {
    if (this.#closed) return;
    this.#closed = true;
    try {
      this.#writer.releaseLock();
    } catch {
      // already released
    }
    await this.#stream.close();
    await this.#recvLoop;
  }
}
