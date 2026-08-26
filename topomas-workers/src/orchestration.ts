export interface Env {
  PROJECT_DO: any;
  QUEUE: any;
}

export class ProjectDO {
  private state: any;
  private projectData: any;

  constructor(state: any, env: Env) {
    this.state = state;
    this.state.blockConcurrencyWhile(async () => {
      this.projectData = (await this.state.storage.get('project')) || {
        id: this.state.id.toString(),
        status: 'PENDING',
        tasks: [],
        paretoFront: [],
        createdAt: Date.now()
      };
    });
  }

  async fetch(request: Request): Promise<Response> {
    const url = new URL(request.url);
    if (url.pathname === '/status') {
      return Response.json(this.projectData);
    }

    if (url.pathname === '/add-task' && request.method === 'POST') {
      const task = await request.json();
      this.projectData.tasks.push(task);
      await this.state.storage.put('project', this.projectData);
      return Response.json({ success: true });
    }

    if (url.pathname === '/update-pareto' && request.method === 'POST') {
      const pareto = await request.json();
      this.projectData.paretoFront = pareto;
      await this.state.storage.put('project', this.projectData);
      return Response.json({ success: true });
    }

    return new Response('Not found', { status: 404 });
  }
}

export default {
  async fetch(request: Request, env: Env): Promise<Response> {
    const url = new URL(request.url);
    if (url.pathname.startsWith('/api/project/')) {
      const projectId = url.pathname.split('/')[3];
      const id = env.PROJECT_DO.idFromName(projectId);
      const obj = env.PROJECT_DO.get(id);
      return obj.fetch(request);
    }
    return new Response('Not found', { status: 404 });
  }
};
