use super::provider::ChatError;
use futures::{Stream, StreamExt};
use std::collections::VecDeque;

struct SseState<S> {
    bytes: S,
    buf: String,
    queue: VecDeque<String>,
    done: bool,
}

/// Turns a streaming HTTP response into a stream of SSE `data:` payloads.
///
/// `event:` lines are ignored — every provider we target encodes the frame
/// type inside the JSON payload (Anthropic) or doesn't need it (OpenAI). The
/// sentinel `[DONE]` payload (OpenAI) terminates the stream.
pub fn sse_data_stream(
    resp: reqwest::Response,
) -> impl Stream<Item = Result<String, ChatError>> + Send {
    let state = SseState {
        bytes: resp.bytes_stream(),
        buf: String::new(),
        queue: VecDeque::new(),
        done: false,
    };

    futures::stream::unfold(state, |mut st| async move {
        loop {
            if let Some(payload) = st.queue.pop_front() {
                return Some((Ok(payload), st));
            }
            if st.done {
                return None;
            }
            match st.bytes.next().await {
                Some(Ok(chunk)) => {
                    st.buf.push_str(&String::from_utf8_lossy(&chunk));
                    while let Some(idx) = st.buf.find('\n') {
                        let line: String = st.buf.drain(..=idx).collect();
                        let line = line.trim_end();
                        if let Some(rest) = line.strip_prefix("data:") {
                            let payload = rest.trim();
                            if payload == "[DONE]" {
                                st.done = true;
                            } else if !payload.is_empty() {
                                st.queue.push_back(payload.to_string());
                            }
                        }
                    }
                }
                Some(Err(e)) => {
                    st.done = true;
                    return Some((Err(ChatError::Stream(e.to_string())), st));
                }
                None => return None,
            }
        }
    })
}
