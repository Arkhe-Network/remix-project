export interface Env {
  AI: any;
  HYPERDRIVE: any;
}

export default {
  async fetch(request: Request, env: Env): Promise<Response> {
    const url = new URL(request.url);

    if (url.pathname === '/api/ontology/search' && request.method === 'POST') {
      const { query } = await request.json();

      const embeddingResponse = await env.AI.run('@cf/baai/bge-base-en-v1.5', {
        text: query
      });
      const embedding = embeddingResponse.data;

      const concepts = await env.HYPERDRIVE.query(
        `SELECT * FROM CancerConcepts WHERE embedding <-> $1 < 0.5`,
        [JSON.stringify(embedding)]
      );

      return Response.json(concepts);
    }

    if (url.pathname === '/api/ontology/material-cancer' && request.method === 'POST') {
      const { materialFormula } = await request.json();

      const prompt = `Given the material ${materialFormula}, which cancer types might it be associated with? Use NCIt and OncoTree terms.`;
      const response = await env.AI.run('@cf/meta/llama-3.1-8b-instruct', {
        messages: [{ role: 'user', content: prompt }]
      });

      return Response.json({ associations: response.response });
    }

    return new Response('Not found', { status: 404 });
  }
};
