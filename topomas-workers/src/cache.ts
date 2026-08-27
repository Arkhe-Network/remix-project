export default {
  async fetch(request: Request, env: any): Promise<Response> {
    const url = new URL(request.url);

    if (url.pathname === '/api/materials/popular') {
      const cacheKey = new Request(request.url, request);
      const cache = caches.default;

      let response = await cache.match(cacheKey);
      if (response) {
        return response;
      }

      const data = await env.HYPERDRIVE.query(
        `SELECT * FROM Materials ORDER BY CreatedAt DESC LIMIT 100`
      );

      response = Response.json(data);
      await cache.put(cacheKey, response.clone());
      return response;
    }

    return new Response('Not found', { status: 404 });
  }
};
