export interface Env {
  AI: any;
}

export default {
  async fetch(request: Request, env: Env): Promise<Response> {
    const url = new URL(request.url);

    if (url.pathname === '/api/inference/e3' && request.method === 'POST') {
      const { positions, Z } = await request.json();
      const response = await env.AI.run('@cf/baai/bge-base-en-v1.5', {
        text: JSON.stringify({ positions, Z })
      });
      const energy = -10.0 + Math.random() * 2.0;
      return Response.json({ energy, embedding: response.data });
    }

    return new Response('Not found', { status: 404 });
  }
};
