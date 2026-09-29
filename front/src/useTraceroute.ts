import { useCallback, useRef, useState } from "react";
import type { Hop, HopEvent } from "./types";

export type ConnectionState = "idle" | "connecting" | "running" | "done" | "error";

/// Backend host/port — hardcoded for now since front/ and back/ are
/// both local during dev. Worth moving to an env var once this is
/// deployed anywhere.
// const WS_BASE = "https://traceroute-te-api.onrender.com";
const WS_BASE = "localhsot:3000"

export function useTraceroute() {
  const [hops, setHops] = useState<Hop[]>([]);
  const [state, setState] = useState<ConnectionState>("idle");
  const socketRef = useRef<WebSocket | null>(null);

  const start = useCallback((target: string) => {
    // Close any previous run before starting a new one — one socket
    // per traceroute, matching the backend's "one run per connection"
    // model.
    socketRef.current?.close();

    setHops([]);
    setState("connecting");

    const url = `${WS_BASE}/ws/traceroute?target=${encodeURIComponent(target)}`;
    const socket = new WebSocket(url);
    socketRef.current = socket;

    socket.onopen = () => setState("running");

    socket.onmessage = (event) => {
      const parsed: HopEvent = JSON.parse(event.data);

      if (parsed.type === "hop") {
        const { type: _type, ...hop } = parsed;
        setHops((prev) => [...prev, hop]);
      } else {
        // hostname_resolved — patch the matching hop by ttl rather
        // than appending, since it's an update to an existing row.
        setHops((prev) =>
          prev.map((h) => (h.ttl === parsed.ttl ? { ...h, hostname: parsed.hostname } : h))
        );
      }
    };

    socket.onclose = () => {
      // Only mark "done" if we didn't already hit an error — onerror
      // fires before onclose on a failed connection, so don't
      // overwrite that state.
      setState((prev) => (prev === "error" ? prev : "done"));
    };

    socket.onerror = () => setState("error");
  }, []);

  const stop = useCallback(() => {
    socketRef.current?.close();
  }, []);

  return { hops, state, start, stop };
}