import { describe, expect, it, vi } from "vitest";
import { fireEvent, render, screen } from "@testing-library/react";
import { DraftsProvider } from "@/hooks/useDrafts";
import { Composer } from "./Composer";

function setup(streaming = false) {
  const onSend = vi.fn();
  const onStop = vi.fn();
  render(
    <DraftsProvider>
      <Composer conversationId="c1" streaming={streaming} onSend={onSend} onStop={onStop} />
    </DraftsProvider>,
  );
  return { onSend, onStop };
}

describe("Composer", () => {
  it("sends on Enter and clears the draft", () => {
    const { onSend } = setup();
    const box = screen.getByLabelText("Message") as HTMLTextAreaElement;
    fireEvent.change(box, { target: { value: "hello there" } });
    fireEvent.keyDown(box, { key: "Enter" });
    expect(onSend).toHaveBeenCalledWith("hello there");
    expect(box.value).toBe("");
  });

  it("does not send on Shift+Enter", () => {
    const { onSend } = setup();
    const box = screen.getByLabelText("Message");
    fireEvent.change(box, { target: { value: "line one" } });
    fireEvent.keyDown(box, { key: "Enter", shiftKey: true });
    expect(onSend).not.toHaveBeenCalled();
  });

  it("does not send blank messages", () => {
    const { onSend } = setup();
    const box = screen.getByLabelText("Message");
    fireEvent.change(box, { target: { value: "   " } });
    fireEvent.keyDown(box, { key: "Enter" });
    expect(onSend).not.toHaveBeenCalled();
  });

  it("blocks sending while streaming and shows a Stop button + notice", () => {
    const { onStop } = setup(true);
    expect(screen.queryByLabelText("Send")).not.toBeInTheDocument();
    expect(screen.getByText(/blocked until it finishes/i)).toBeInTheDocument();
    fireEvent.click(screen.getByLabelText("Stop"));
    expect(onStop).toHaveBeenCalled();
  });
});
