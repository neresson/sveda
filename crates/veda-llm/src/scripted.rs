use std::collections::VecDeque;
use std::future::Future;
use std::pin::Pin;
use std::sync::Mutex;

use futures_util::Stream;

use crate::{queued_stream, LlmChunk, LlmClient, LlmError, ModelSpec, StepRequest};

pub struct ScriptedClient {
    queue: Mutex<Vec<Vec<Result<LlmChunk, LlmError>>>>,
    complete_replies: Mutex<VecDeque<String>>,
    pub attempts: Mutex<Vec<String>>,
    pub last_thinking: Mutex<Option<bool>>,
    pub last_tools: Mutex<Vec<Vec<String>>>,
    pub last_message_counts: Mutex<Vec<usize>>,
    pub last_instructions: Mutex<Option<String>>,
}

impl ScriptedClient {
    pub fn queue(steps: Vec<Vec<Result<LlmChunk, LlmError>>>) -> Self {
        Self {
            queue: Mutex::new(steps),
            complete_replies: Mutex::new(VecDeque::new()),
            attempts: Mutex::new(Vec::new()),
            last_thinking: Mutex::new(None),
            last_tools: Mutex::new(Vec::new()),
            last_message_counts: Mutex::new(Vec::new()),
            last_instructions: Mutex::new(None),
        }
    }

    pub fn hello_world() -> Self {
        Self::queue(vec![vec![
            Ok(LlmChunk::TextDelta("Hello from Veda".into())),
            Ok(LlmChunk::Usage {
                prompt_tokens: 0,
                completion_tokens: 3,
            }),
            Ok(LlmChunk::End {
                finish_reason: "stop".into(),
            }),
        ]])
    }

    pub fn with_complete_replies(self, replies: Vec<String>) -> Self {
        *self.complete_replies.lock().expect("complete lock") = replies.into();
        self
    }

    pub fn last_thinking(&self) -> Option<bool> {
        *self.last_thinking.lock().expect("thinking lock")
    }
}

impl LlmClient for ScriptedClient {
    fn stream_step(
        &self,
        model: &ModelSpec,
        request: StepRequest,
    ) -> Pin<Box<dyn Stream<Item = Result<LlmChunk, LlmError>> + Send>> {
        self.attempts
            .lock()
            .expect("attempts lock")
            .push(model.id.clone());
        *self.last_thinking.lock().expect("thinking lock") = Some(request.thinking);
        *self.last_instructions.lock().expect("instructions lock") = request.instructions.clone();
        self.last_tools
            .lock()
            .expect("tools lock")
            .push(request.tools.iter().map(|tool| tool.name.clone()).collect());
        self.last_message_counts
            .lock()
            .expect("counts lock")
            .push(request.messages.len());
        let mut queue = self.queue.lock().expect("script lock");
        let chunks = if queue.is_empty() {
            vec![Err(LlmError::Other("script exhausted".into()))]
        } else {
            queue.remove(0)
        };
        queued_stream(chunks)
    }

    fn complete_text(
        &self,
        model: &ModelSpec,
        _instructions: &str,
        _prompt: &str,
    ) -> Pin<Box<dyn Future<Output = Result<String, LlmError>> + Send>> {
        self.attempts
            .lock()
            .expect("attempts lock")
            .push(format!("complete:{}", model.id));
        let reply = self
            .complete_replies
            .lock()
            .expect("complete lock")
            .pop_front()
            .unwrap_or_default();
        Box::pin(async move { Ok(reply) })
    }
}
