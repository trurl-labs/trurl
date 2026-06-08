import type { GraphSnapshot } from '../types';

/** Typed REST client for the map server API. */
export class ApiClient {
  private baseUrl: string;
  private token: string;

  constructor(token: string) {
    this.baseUrl = `${location.protocol}//${location.host}`;
    this.token = token;
  }

  async fetchGraph(): Promise<GraphSnapshot> {
    const res = await fetch(`${this.baseUrl}/api/graph`, {
      headers: { Authorization: `Bearer ${this.token}` },
    });
    if (!res.ok) throw new Error(`GET /api/graph: ${res.status}`);
    return res.json();
  }

  async saveLayout(
    positions: Record<string, { x: number; y: number; pinned: boolean }>,
    version: number,
  ): Promise<number> {
    const res = await fetch(`${this.baseUrl}/api/layout`, {
      method: 'PUT',
      headers: this.jsonHeaders(),
      body: JSON.stringify({ positions, layout_version: version }),
    });
    if (!res.ok) throw new Error(`PUT /api/layout: ${res.status}`);
    const data = await res.json();
    return data.layout_version;
  }

  async resetLayout(): Promise<number> {
    const res = await fetch(`${this.baseUrl}/api/layout/reset`, {
      method: 'POST',
      headers: { Authorization: `Bearer ${this.token}` },
    });
    if (!res.ok) throw new Error(`POST /api/layout/reset: ${res.status}`);
    const data = await res.json();
    return data.layout_version;
  }

  // ── Mutations ───────────────────────────────────────────────────────

  async createComponent(name: string, description: string): Promise<void> {
    const res = await fetch(`${this.baseUrl}/api/component`, {
      method: 'POST',
      headers: this.jsonHeaders(),
      body: JSON.stringify({ name, description }),
    });
    if (!res.ok) {
      const data = await res.json().catch(() => ({}));
      throw new Error(data.error ?? `POST component: ${res.status}`);
    }
  }

  async createConnection(from: string, to: string): Promise<void> {
    const res = await fetch(`${this.baseUrl}/api/connection`, {
      method: 'POST',
      headers: this.jsonHeaders(),
      body: JSON.stringify({ from, to }),
    });
    if (!res.ok) {
      const data = await res.json().catch(() => ({}));
      throw new Error(data.error ?? `POST connection: ${res.status}`);
    }
  }

  async updateDecision(
    name: string,
    body: { choice?: string; reason?: string; tags?: string[] },
  ): Promise<void> {
    const res = await fetch(`${this.baseUrl}/api/decision/${enc(name)}`, {
      method: 'PUT',
      headers: this.jsonHeaders(),
      body: JSON.stringify(body),
    });
    if (!res.ok) {
      const data = await res.json().catch(() => ({}));
      throw new Error(data.error ?? `PUT decision: ${res.status}`);
    }
  }

  async deleteDecision(name: string): Promise<void> {
    const res = await fetch(`${this.baseUrl}/api/decision/${enc(name)}`, {
      method: 'DELETE',
      headers: { Authorization: `Bearer ${this.token}` },
    });
    if (!res.ok) {
      const data = await res.json().catch(() => ({}));
      throw new Error(data.error ?? `DELETE decision: ${res.status}`);
    }
  }

  async deleteComponent(name: string): Promise<void> {
    const res = await fetch(`${this.baseUrl}/api/component/${enc(name)}`, {
      method: 'DELETE',
      headers: { Authorization: `Bearer ${this.token}` },
    });
    if (!res.ok) {
      const data = await res.json().catch(() => ({}));
      throw new Error(data.error ?? `DELETE component: ${res.status}`);
    }
  }

  async deleteConnection(from: string, to: string): Promise<void> {
    const res = await fetch(`${this.baseUrl}/api/connection/${enc(from)}/${enc(to)}`, {
      method: 'DELETE',
      headers: { Authorization: `Bearer ${this.token}` },
    });
    if (!res.ok) {
      const data = await res.json().catch(() => ({}));
      throw new Error(data.error ?? `DELETE connection: ${res.status}`);
    }
  }

  private jsonHeaders(): Record<string, string> {
    return {
      Authorization: `Bearer ${this.token}`,
      'Content-Type': 'application/json',
    };
  }
}

function enc(s: string): string {
  return encodeURIComponent(s);
}
