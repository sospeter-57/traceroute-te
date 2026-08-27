/// Matches `Hop` in back/src/traceroute/hop.rs. `rtt` arrives as
/// milliseconds (a plain number) thanks to the custom serializer on
/// the Rust side — not the default Duration {secs, nanos} shape.
export interface Hop {
  ttl: number;
  addr: string | null;
  rtt: number | null;
  reached_target: boolean;
  hostname: string | null;
}

/// Matches `HopUpdate` in hop.rs — a later, partial patch to a hop
/// that already arrived, carrying just the resolved hostname.
export interface HopUpdate {
  ttl: number;
  hostname: string;
}

/// Matches `HopEvent` in hop.rs. `#[serde(tag = "type")]` puts a
/// `type` discriminant field alongside the variant's own fields
/// (rather than nesting them under a "content" key), so this is a
/// flat discriminated union, not `{ type, data: {...} }`.
export type HopEvent =
  | ({ type: "hop" } & Hop)
  | ({ type: "hostname_resolved" } & HopUpdate);