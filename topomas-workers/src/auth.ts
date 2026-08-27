// src/auth.ts
export interface Env {
  JWT_SECRET: string;
}

export default {
  async fetch(request: Request, env: Env): Promise<Response> {
    const url = new URL(request.url);

    if (url.pathname === '/api/auth/verify' && request.method === 'POST') {
      const payload = { scope: 'user' };
      const token = await signJWT(payload, env.JWT_SECRET);
      return Response.json({ token });
    }

    if (url.pathname === '/api/auth/validate' && request.method === 'GET') {
      const authHeader = request.headers.get('Authorization');
      if (!authHeader?.startsWith('Bearer ')) {
        return new Response('Missing token', { status: 401 });
      }
      const token = authHeader.slice(7);
      try {
        const payload = await verifyJWT(token, env.JWT_SECRET);
        return Response.json({ valid: true, payload });
      } catch {
        return Response.json({ valid: false }, { status: 401 });
      }
    }

    return new Response('Not found', { status: 404 });
  }
};

export async function signJWT(payload: any, secret: string): Promise<string> {
  const encoder = new TextEncoder();
  const header = { alg: 'HS256', typ: 'JWT' };
  const encodedHeader = btoa(JSON.stringify(header));
  const encodedPayload = btoa(JSON.stringify(payload));
  const data = `${encodedHeader}.${encodedPayload}`;
  const key = await crypto.subtle.importKey(
    'raw',
    encoder.encode(secret),
    { name: 'HMAC', hash: 'SHA-256' },
    false,
    ['sign']
  );
  const signature = await crypto.subtle.sign('HMAC', key, encoder.encode(data));
  const encodedSignature = btoa(String.fromCharCode(...new Uint8Array(signature)));
  return `${data}.${encodedSignature}`;
}

export async function verifyJWT(token: string, secret: string): Promise<any> {
  const [encodedHeader, encodedPayload, encodedSignature] = token.split('.');
  const data = `${encodedHeader}.${encodedPayload}`;
  const key = await crypto.subtle.importKey(
    'raw',
    new TextEncoder().encode(secret),
    { name: 'HMAC', hash: 'SHA-256' },
    false,
    ['verify']
  );
  const signature = Uint8Array.from(atob(encodedSignature), c => c.charCodeAt(0));
  const isValid = await crypto.subtle.verify('HMAC', key, signature, new TextEncoder().encode(data));
  if (!isValid) throw new Error('Invalid signature');
  return JSON.parse(atob(encodedPayload));
}
