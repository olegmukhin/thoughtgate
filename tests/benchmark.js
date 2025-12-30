import http from 'k6/http';

export const options = {
  vus: 10,
  duration: '10s',
};

const PAYLOAD = JSON.stringify({ prompt: "A".repeat(1000) });

export default function () {
  // Hit the Proxy (port 4141).
  // Path MUST match what the proxy forwards to the Mock LLM.
  http.post('http://127.0.0.1:4141/v1/chat/completions', PAYLOAD, {
    headers: { 'Content-Type': 'application/json' },
    timeout: '5s',
  });
}
