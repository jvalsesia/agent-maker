import { describe, expect, it, vi } from "vitest";
import { fireEvent, render, screen } from "@testing-library/react";
import type { Message } from "@/lib/api";
import { MessageBubble } from "./MessageBubble";

function msg(over: Partial<Message>): Message {
  return {
    id: "m1",
    conversation_id: "c1",
    role: "assistant",
    content: "hi",
    status: "complete",
    model: null,
    token_count: null,
    created_at: new Date().toISOString(),
    ...over,
  };
}

describe("MessageBubble", () => {
  it("renders markdown with a highlighted code block", () => {
    const { container } = render(
      <MessageBubble message={msg({ content: "text\n\n```js\nconst x = 1;\n```" })} />,
    );
    expect(container.querySelector("code.hljs")).toBeTruthy();
  });

  it("shows a model + token chip on assistant messages", () => {
    render(<MessageBubble message={msg({ model: "gpt-4o", token_count: 42 })} />);
    expect(screen.getByText(/gpt-4o/)).toBeInTheDocument();
    expect(screen.getByText(/~42 tokens/)).toBeInTheDocument();
  });

  it("renders a retry action on an errored turn", () => {
    const onRetry = vi.fn();
    render(<MessageBubble message={msg({ status: "error", content: "" })} onRetry={onRetry} />);
    fireEvent.click(screen.getByText("Retry"));
    expect(onRetry).toHaveBeenCalled();
  });

  it("renders a delegated turn as a labeled sub-agent bubble", () => {
    render(
      <MessageBubble
        message={msg({ content: "found a null deref", subagent_alias: "code-reviewer" })}
      />,
    );
    expect(screen.getByText("@code-reviewer")).toBeInTheDocument();
    expect(screen.getByText(/found a null deref/)).toBeInTheDocument();
  });

  it("shows a consulted-count chip on a parent reply", () => {
    render(<MessageBubble message={msg({ content: "synthesis" })} consultedCount={2} />);
    expect(screen.getByText(/Consulted 2 sub-agents/)).toBeInTheDocument();
  });

  it("expands recalled earlier turns", () => {
    render(
      <MessageBubble
        message={msg({
          recalled: [
            {
              message_id: "r1",
              role: "user",
              content: "the secret code is platypus",
              created_at: new Date().toISOString(),
              similarity: 0.83,
            },
          ],
        })}
      />,
    );
    fireEvent.click(screen.getByText(/Recalled 1 earlier turn/));
    expect(screen.getByText(/the secret code is platypus/)).toBeInTheDocument();
    expect(screen.getByText(/83% match/)).toBeInTheDocument();
  });
});
