export interface Env {
  HYPERDRIVE: any;
}

export default {
  async fetch(request: Request, env: Env): Promise<Response> {
    const url = new URL(request.url);

    if (url.pathname === '/api/sql/query' && request.method === 'POST') {
      const { query, params } = await request.json();
      const result = await env.HYPERDRIVE.query(query, params);
      return Response.json(result);
    }

    if (url.pathname === '/api/metrics') {
      const rows = await env.HYPERDRIVE.query(
        `SELECT * FROM vw_ProjectSummary WHERE ProjectID = $1`,
        [url.searchParams.get('projectId')]
      );
      return Response.json(rows);
    }

    return new Response('Not found', { status: 404 });
  }
};
