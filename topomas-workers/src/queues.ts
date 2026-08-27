export interface Env {
  QUEUE: any;
  TOPOMAS_API: string;
}

export default {
  async queue(batch: any, env: Env): Promise<void> {
    for (const message of batch) {
      const { projectId, task } = message.body;

      const response = await fetch(`${env.TOPOMAS_API}/task/process`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ projectId, task })
      });

      if (!response.ok) {
        message.retry({ delaySeconds: 30 });
      } else {
        const result = await response.json();
        await fetch(`${env.TOPOMAS_API}/internal/update-pareto`, {
          method: 'POST',
          body: JSON.stringify({ projectId, result })
        });
      }
    }
  }
};
