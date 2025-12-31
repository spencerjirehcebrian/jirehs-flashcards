import { describe, it, expect, vi } from 'vitest';
import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { TypedAnswerInput } from './TypedAnswerInput';

describe('TypedAnswerInput', () => {
  it('should update value on input change', async () => {
    const user = userEvent.setup();
    const onChange = vi.fn();
    const onSubmit = vi.fn();

    render(
      <TypedAnswerInput value="" onChange={onChange} onSubmit={onSubmit} />
    );

    const textarea = screen.getByPlaceholderText('Type your answer...');
    await user.type(textarea, 'test');

    expect(onChange).toHaveBeenCalled();
  });

  it('should show placeholder text', () => {
    const onChange = vi.fn();
    const onSubmit = vi.fn();

    render(
      <TypedAnswerInput value="" onChange={onChange} onSubmit={onSubmit} />
    );

    expect(screen.getByPlaceholderText('Type your answer...')).toBeInTheDocument();
  });

  it('should disable submit button when empty', () => {
    const onChange = vi.fn();
    const onSubmit = vi.fn();

    render(
      <TypedAnswerInput value="" onChange={onChange} onSubmit={onSubmit} />
    );

    expect(screen.getByRole('button', { name: 'Check Answer' })).toBeDisabled();
  });

  it('should disable submit button when only whitespace', () => {
    const onChange = vi.fn();
    const onSubmit = vi.fn();

    render(
      <TypedAnswerInput value="   " onChange={onChange} onSubmit={onSubmit} />
    );

    expect(screen.getByRole('button', { name: 'Check Answer' })).toBeDisabled();
  });

  it('should enable submit button when value has content', () => {
    const onChange = vi.fn();
    const onSubmit = vi.fn();

    render(
      <TypedAnswerInput value="test answer" onChange={onChange} onSubmit={onSubmit} />
    );

    expect(screen.getByRole('button', { name: 'Check Answer' })).not.toBeDisabled();
  });

  it('should submit on Enter key when value is not empty', async () => {
    const user = userEvent.setup();
    const onChange = vi.fn();
    const onSubmit = vi.fn();

    render(
      <TypedAnswerInput value="test" onChange={onChange} onSubmit={onSubmit} />
    );

    const textarea = screen.getByPlaceholderText('Type your answer...');
    await user.type(textarea, '{Enter}');

    expect(onSubmit).toHaveBeenCalledTimes(1);
  });

  it('should not submit on Enter when value is empty', async () => {
    const user = userEvent.setup();
    const onChange = vi.fn();
    const onSubmit = vi.fn();

    render(
      <TypedAnswerInput value="" onChange={onChange} onSubmit={onSubmit} />
    );

    const textarea = screen.getByPlaceholderText('Type your answer...');
    await user.type(textarea, '{Enter}');

    expect(onSubmit).not.toHaveBeenCalled();
  });

  it('should allow new lines with Shift+Enter', async () => {
    const user = userEvent.setup();
    const onChange = vi.fn();
    const onSubmit = vi.fn();

    render(
      <TypedAnswerInput value="test" onChange={onChange} onSubmit={onSubmit} />
    );

    const textarea = screen.getByPlaceholderText('Type your answer...');
    await user.type(textarea, '{Shift>}{Enter}{/Shift}');

    expect(onSubmit).not.toHaveBeenCalled();
  });

  it('should submit when button is clicked', async () => {
    const user = userEvent.setup();
    const onChange = vi.fn();
    const onSubmit = vi.fn();

    render(
      <TypedAnswerInput value="test" onChange={onChange} onSubmit={onSubmit} />
    );

    await user.click(screen.getByRole('button', { name: 'Check Answer' }));

    expect(onSubmit).toHaveBeenCalledTimes(1);
  });

  it('should disable textarea when disabled prop is true', () => {
    const onChange = vi.fn();
    const onSubmit = vi.fn();

    render(
      <TypedAnswerInput value="test" onChange={onChange} onSubmit={onSubmit} disabled />
    );

    expect(screen.getByPlaceholderText('Type your answer...')).toBeDisabled();
  });

  it('should disable button when disabled prop is true', () => {
    const onChange = vi.fn();
    const onSubmit = vi.fn();

    render(
      <TypedAnswerInput value="test" onChange={onChange} onSubmit={onSubmit} disabled />
    );

    expect(screen.getByRole('button', { name: 'Check Answer' })).toBeDisabled();
  });

  it('should auto-focus textarea on mount when not disabled', () => {
    const onChange = vi.fn();
    const onSubmit = vi.fn();

    render(
      <TypedAnswerInput value="" onChange={onChange} onSubmit={onSubmit} />
    );

    expect(screen.getByPlaceholderText('Type your answer...')).toHaveFocus();
  });

  it('should not auto-focus textarea when disabled', () => {
    const onChange = vi.fn();
    const onSubmit = vi.fn();

    render(
      <TypedAnswerInput value="" onChange={onChange} onSubmit={onSubmit} disabled />
    );

    expect(screen.getByPlaceholderText('Type your answer...')).not.toHaveFocus();
  });

  it('should render with correct class names', () => {
    const onChange = vi.fn();
    const onSubmit = vi.fn();

    const { container } = render(
      <TypedAnswerInput value="" onChange={onChange} onSubmit={onSubmit} />
    );

    expect(container.querySelector('.typed-answer-input')).toBeInTheDocument();
    expect(container.querySelector('.typed-answer-textarea')).toBeInTheDocument();
    expect(container.querySelector('.typed-answer-submit')).toBeInTheDocument();
  });
});
