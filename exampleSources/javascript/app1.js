// Async fetch with retry + class-based client.

class ApiClient {
  constructor(baseUrl, timeoutMs = 5000) {
    this.baseUrl = baseUrl;
    this.timeoutMs = timeoutMs;
  }

  async fetchJson(path, retries = 2) {
    let lastErr;
    for (let attempt = 0; attempt <= retries; attempt++) {
      try {
        const ctrl = new AbortController();
        const timer = setTimeout(() => ctrl.abort(), this.timeoutMs);
        const res = await fetch(this.baseUrl + path, { signal: ctrl.signal });
        clearTimeout(timer);
        if (!res.ok) throw new Error(`HTTP ${res.status}`);
        return await res.json();
      } catch (err) {
        lastErr = err;
      }
    }
    throw lastErr;
  }
}

async function main() {
  const client = new ApiClient("https://api.example.com");
  const data = await client.fetchJson("/v1/users");
  console.log(`Loaded ${data.length} users`);
}

main().catch((e) => {
  console.error(e);
  process.exit(1);
});