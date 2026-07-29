use crate::agent::AgentMessage;
pub struct ToolContext { pub workspace: String }
impl ToolContext { pub fn new(workspace: String) -> Self { Self { workspace } } }
pub struct SessionManager { _max_history: usize }
#[derive(Clone)]
pub struct Session { pub history: Vec<AgentMessage> }
impl SessionManager {
    pub fn new(max_history: usize) -> Self { Self { _max_history: max_history } }
    pub async fn get_session(&self, _id: &str) -> Option<Session> { Some(Session { history: vec![] }) }
    pub async fn create_session(&self, _id: &str, _ctx: std::sync::Arc<ToolContext>) {}
    pub async fn append_message(&self, _id: &str, _msg: AgentMessage) {}
}
