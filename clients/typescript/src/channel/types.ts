/**
 * Channel 抽象 (= Phase 2c)。
 *
 * 全通信は channel 経由 (RPC は廃止)。 channel には 2 系統ある:
 * - `UnisonChannel` — stream backend (= QUIC bidi stream)、 request/response + event
 * - `DatagramChannel` — datagram backend (= 共有 datagram path)、 broadcast event のみ
 *
 * 両 interface は Phase 1 codegen が出力する `<Name>ChannelMeta` const に対して
 * generic。 `as const` literal narrowing により、 caller code で event 名 /
 * request 名が compile-time に絞り込まれる。
 */

/** Stream backend の channel meta 形状 (= Phase 1 codegen 出力の構造的 subset) */
export interface ChannelMeta {
  readonly name: string;
  readonly backend: "stream";
  readonly from: "client" | "server" | "either";
  readonly lifetime: "transient" | "persistent";
  readonly events: readonly string[];
  readonly requests: Readonly<
    Record<string, { readonly request: string; readonly response: string }>
  >;
}

/** Datagram backend の channel meta 形状 (= `channelId` 必須、 `requests` 空) */
export interface DatagramChannelMeta {
  readonly name: string;
  readonly backend: "datagram";
  readonly channelId: number;
  readonly from: "client" | "server" | "either";
  readonly lifetime: "transient" | "persistent";
  readonly events: readonly string[];
  readonly requests: Readonly<Record<string, never>>;
}

/** meta の `events` 配列から event 名 union を導出 */
export type EventName<M> = M extends { events: readonly (infer N)[] }
  ? N & string
  : never;

/** meta の `requests` map から request 名 union を導出 */
export type RequestName<M> = M extends { requests: infer R }
  ? keyof R & string
  : never;

/**
 * Channel payload は codegen が別 interface として出力する。 meta は型名の
 * string literal しか保持しないため、 wrapper layer は payload を構造型
 * (`Record<string, unknown>`) として扱い、 caller 側で generated interface に
 * narrow する (= design doc §4.3 の type-narrowing 戦略)。
 */
export type ChannelPayload = Record<string, unknown>;

/**
 * Stream channel — request/response + server-pushed event。
 *
 * QUIC bidi stream に対応、 ordered + reliable。 `request()` は length-prefixed
 * frame を送り response frame を await、 `events()` は server push を AsyncIterable
 * で配る。
 */
export interface UnisonChannel<M extends ChannelMeta = ChannelMeta> {
  /** KDL schema 上の channel 名 */
  readonly name: M["name"];

  /**
   * Request を送り response を await する (= ordered/reliable)。
   * `name` は `M["requests"]` の key に narrow される。
   */
  request(
    name: RequestName<M>,
    payload: ChannelPayload,
  ): Promise<ChannelPayload>;

  /** Server push event の購読 (= `for await`、 break で channel close cascade) */
  events(): AsyncIterableIterator<ChannelPayload>;

  /** Event を送信 (= client → server、 応答なし) */
  sendEvent(name: EventName<M>, payload: ChannelPayload): Promise<void>;

  /** Channel を閉じる (= 配下 stream を tear down、 idempotent) */
  close(): Promise<void>;
}

/**
 * Datagram channel — broadcast event のみ (= request 不可)。
 *
 * 共有 datagram path 上の virtual stream、 `channelId` varint prefix で demux。
 * unordered + unreliable。 caller は基本 `events()` で subscribe するのみ。
 */
export interface DatagramChannel<M extends DatagramChannelMeta = DatagramChannelMeta> {
  /** KDL schema 上の channel 名 */
  readonly name: M["name"];

  /** schema-time fixed の demux 識別子 (= varint prefix として wire 出現) */
  readonly channelId: M["channelId"];

  /** Datagram broadcast event の購読 (= unordered/unreliable) */
  events(): AsyncIterableIterator<ChannelPayload>;

  /** Event を datagram で送信 (= best-effort、 MTU 超過は reject) */
  sendEvent(name: EventName<M>, payload: ChannelPayload): Promise<void>;

  /** Channel を閉じる (= dispatcher から unregister、 idempotent) */
  close(): Promise<void>;
}
