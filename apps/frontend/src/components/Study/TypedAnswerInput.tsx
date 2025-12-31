import { useState, useRef, useEffect } from 'react';

interface TypedAnswerInputProps {
  value: string;
  onChange: (value: string) => void;
  onSubmit: () => void;
  disabled?: boolean;
}

export function TypedAnswerInput({
  value,
  onChange,
  onSubmit,
  disabled = false,
}: TypedAnswerInputProps) {
  const textareaRef = useRef<HTMLTextAreaElement>(null);

  useEffect(() => {
    if (textareaRef.current && !disabled) {
      textareaRef.current.focus();
    }
  }, [disabled]);

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault();
      if (value.trim()) {
        onSubmit();
      }
    }
  };

  return (
    <div className="typed-answer-input">
      <textarea
        ref={textareaRef}
        className="form-input typed-answer-textarea"
        value={value}
        onChange={(e) => onChange(e.target.value)}
        onKeyDown={handleKeyDown}
        placeholder="Type your answer..."
        disabled={disabled}
        rows={3}
      />
      <button
        type="button"
        className="button typed-answer-submit"
        onClick={onSubmit}
        disabled={disabled || !value.trim()}
      >
        Check Answer
      </button>
    </div>
  );
}
