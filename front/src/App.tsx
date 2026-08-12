import { useState } from "react";
import { useTraceroute } from "./useTraceroute";

export default function App() {
  const [target, setTarget] = useState("8.8.8.8");
  const { hops, state, start, stop } = useTraceroute();

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    if (target.trim()) start(target.trim());
  };

  return (
    <main style={{ fontFamily: "monospace", padding: "2rem", maxWidth: 600 }}>
      <h1>traceroute-te</h1>

      <form onSubmit={handleSubmit} style={{ marginBottom: "1rem" }}>
        <input
          value={target}
          onChange={(e) => setTarget(e.target.value)}
          placeholder="host or IP"
          disabled={state === "connecting" || state === "running"}
        />
        <button type="submit" disabled={state === "connecting" || state === "running"}>
          Run
        </button>
        {state === "running" && (
          <button type="button" onClick={stop}>
            Stop
          </button>
        )}
      </form>

      <p>status: {state}</p>

      <ol>
        {hops.map((hop) => (
          <li key={hop.ttl}>
            {hop.addr === null
              ? "*"
              : `${hop.hostname ?? hop.addr} (${hop.addr})  ${hop.rtt?.toFixed(3)} ms`}
            {hop.reached_target && "  ← target"}
          </li>
        ))}
      </ol>
    </main>
  );
}