import { useState, type FormEvent } from "react";
import {
  Body1,
  Button,
  Caption1,
  Input,
  Subtitle2,
  makeStyles,
  tokens,
} from "@fluentui/react-components";
import { SendRegular } from "@fluentui/react-icons";

export type ChatRole = "user" | "assistant" | "error";

export interface ChatMessage {
  id: string;
  role: ChatRole;
  text: string;
}

export interface ChatPanelProps {
  provider: (prompt: string) => Promise<string>;
  emptyHint: string;
  ariaLabel: string;
  title?: string;
  submitLabel?: string;
}

const useStyles = makeStyles({
  root: {
    display: "flex",
    flexDirection: "column",
    minHeight: 0,
    gap: tokens.spacingVerticalS,
    height: "100%",
  },
  transcript: {
    flex: 1,
    minHeight: 0,
    overflowY: "auto",
    display: "flex",
    flexDirection: "column",
    gap: tokens.spacingVerticalXS,
    padding: tokens.spacingHorizontalS,
    backgroundColor: tokens.colorNeutralBackground2,
    borderRadius: tokens.borderRadiusMedium,
  },
  emptyState: {
    color: tokens.colorNeutralForeground3,
    textAlign: "center",
    padding: tokens.spacingVerticalM,
  },
  bubble: {
    padding: `${tokens.spacingVerticalXS} ${tokens.spacingHorizontalS}`,
    borderRadius: tokens.borderRadiusMedium,
    maxWidth: "90%",
    overflowWrap: "anywhere",
  },
  user: {
    alignSelf: "flex-end",
    backgroundColor: tokens.colorBrandBackground2,
    color: tokens.colorNeutralForeground1,
  },
  assistant: {
    alignSelf: "flex-start",
    backgroundColor: tokens.colorNeutralBackground1,
    color: tokens.colorNeutralForeground1,
  },
  error: {
    alignSelf: "flex-start",
    backgroundColor: tokens.colorPaletteRedBackground2,
    color: tokens.colorPaletteRedForeground1,
  },
  composer: {
    display: "flex",
    gap: tokens.spacingHorizontalS,
    alignItems: "center",
  },
  input: { flex: 1 },
});

let bubbleSeq = 0;
const nextId = () => {
  bubbleSeq += 1;
  return `chat-${bubbleSeq}`;
};

const bubbleClassFor = (
  role: ChatRole,
  styles: ReturnType<typeof useStyles>,
): string => {
  const base = styles.bubble;
  switch (role) {
    case "user":
      return `${base} ${styles.user}`;
    case "error":
      return `${base} ${styles.error}`;
    case "assistant":
    default:
      return `${base} ${styles.assistant}`;
  }
};

const errorMessage = (error: unknown): string => {
  if (error instanceof Error) {
    return error.message;
  }
  return String(error);
};

export const ChatPanel = ({
  provider,
  emptyHint,
  ariaLabel,
  title,
  submitLabel = "Send",
}: ChatPanelProps) => {
  const styles = useStyles();
  const [messages, setMessages] = useState<ChatMessage[]>([]);
  const [input, setInput] = useState("");
  const [busy, setBusy] = useState(false);

  const onSubmit = async (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    const prompt = input.trim();
    if (!prompt || busy) {
      return;
    }
    setBusy(true);
    setMessages((prev) => [
      ...prev,
      { id: nextId(), role: "user", text: prompt },
    ]);
    setInput("");
    try {
      const reply = await provider(prompt);
      setMessages((prev) => [
        ...prev,
        { id: nextId(), role: "assistant", text: reply },
      ]);
    } catch (error) {
      setMessages((prev) => [
        ...prev,
        { id: nextId(), role: "error", text: errorMessage(error) },
      ]);
    } finally {
      setBusy(false);
    }
  };

  return (
    <section className={styles.root} aria-label={ariaLabel}>
      {title && <Subtitle2>{title}</Subtitle2>}
      <div className={styles.transcript} role="log" aria-live="polite">
        {messages.length === 0 ? (
          <Caption1 className={styles.emptyState}>{emptyHint}</Caption1>
        ) : (
          messages.map((message) => (
            <div
              key={message.id}
              className={bubbleClassFor(message.role, styles)}
              data-role={message.role}
            >
              <Body1>{message.text}</Body1>
            </div>
          ))
        )}
      </div>
      <form className={styles.composer} onSubmit={onSubmit}>
        <Input
          className={styles.input}
          value={input}
          aria-label={`${ariaLabel} prompt`}
          placeholder="Ask the AI…"
          onChange={(_, data) => setInput(data.value)}
          disabled={busy}
        />
        <Button
          appearance="primary"
          icon={<SendRegular />}
          type="submit"
          disabled={busy || input.trim().length === 0}
        >
          {submitLabel}
        </Button>
      </form>
    </section>
  );
};
