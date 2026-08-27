export interface Env {
  R2_BUCKET: any;
}

export default {
  async fetch(request: Request, env: Env): Promise<Response> {
    const url = new URL(request.url);

    if (url.pathname === '/api/upload' && request.method === 'POST') {
      const formData = await request.formData();
      const file = formData.get('file') as File;
      const key = formData.get('key') as string;

      await env.R2_BUCKET.put(key, file.stream(), {
        httpMetadata: { contentType: file.type }
      });
      return Response.json({ success: true, key });
    }

    if (url.pathname.startsWith('/api/download/')) {
      const key = url.pathname.replace('/api/download/', '');
      const object = await env.R2_BUCKET.get(key);
      if (!object) {
        return new Response('Not found', { status: 404 });
      }
      return new Response(object.body, {
        headers: { 'Content-Type': object.httpMetadata?.contentType || 'application/octet-stream' }
      });
    }

    return new Response('Not found', { status: 404 });
  }
};
